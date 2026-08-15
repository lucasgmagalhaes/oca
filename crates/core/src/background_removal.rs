//! AI background removal — per `request.md`'s Fase 4 "Remoção de fundo por IA": cut the person
//! out of the background without a green screen, complementing chroma key (which needs a real
//! green/chroma backdrop). Unlike chroma key's `colorkey` avfilter expression (a per-pixel
//! formula FFmpeg's filter graph can evaluate directly), a neural matte can't be expressed as an
//! avfilter string — [`segment_person`] runs the model against one decoded frame in Rust and
//! returns a plain alpha buffer; turning a whole clip's worth of per-frame mattes into an actual
//! exported/previewed alpha channel is a separate, not-yet-built pipeline stage (see
//! `crate::timeline::ClipInstance::background_removal_enabled`'s doc comment for the current
//! wiring gap, same shape as `gain_db`/`blur_intensity` before they got wired to export).
//!
//! **Model:** MODNet (`ZHKKKe/MODNet`, "photographic" weights ported to ONNX by
//! `yakhyo/modnet`, Apache-2.0), downloaded on demand via
//! [`crate::model_download::download_background_removal_model`] — same "not bundled in the
//! installer yet" gap as the Whisper and UltraFace models. Preprocessing (resize so both
//! dimensions are multiples of 32 targeting ~512px on the constrained side, `(pixel/255 - 0.5) /
//! 0.5` normalization, RGB/CHW) and postprocessing (single-channel matte in `0.0..=1.0`, resized
//! back to the source frame) come from that repo's own `onnx_inference.py`/`modnet_onnx.py`
//! reference implementation, not guessed. **Confirmed working end-to-end** on this dev machine:
//! a real inference pass against the downloaded model succeeded against a synthetic frame with
//! a bright rectangle on a dark background, producing a plausible low-average matte (no real
//! person shape in the fixture, so a mostly-background result is expected).

use std::path::Path;

/// MODNet is fully convolutional — any input size works, but the reference implementation
/// targets roughly this size on the constrained dimension (then rounds both dimensions down to
/// a multiple of 32, required by MODNet's downsampling stages) for a speed/quality balance.
const TARGET_SIZE: u32 = 512;

#[derive(Debug)]
pub enum SegmentError {
    Onnx(String),
}

impl std::fmt::Display for SegmentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SegmentError::Onnx(e) => write!(f, "ONNX inference failed: {e}"),
        }
    }
}

impl std::error::Error for SegmentError {}

/// Picks the model's input resolution for a `source_w x source_h` frame: scales so the
/// constrained side lands near [`TARGET_SIZE`] (matching `modnet_onnx.py`'s `preprocess`), then
/// rounds each dimension down to the nearest multiple of 32 (MODNet's encoder downsamples by
/// 32x internally, so any other input size fails at a shape-mismatch layer). Never rounds down
/// to zero — a frame smaller than 32px on a side still gets a minimum of one 32px block.
fn model_input_size(source_w: u32, source_h: u32) -> (u32, u32) {
    let (mut new_w, mut new_h) =
        if source_w.max(source_h) < TARGET_SIZE || source_w.min(source_h) > TARGET_SIZE {
            if source_w >= source_h {
                let new_h = TARGET_SIZE;
                let new_w = source_w * TARGET_SIZE / source_h;
                (new_w, new_h)
            } else {
                let new_w = TARGET_SIZE;
                let new_h = source_h * TARGET_SIZE / source_w;
                (new_w, new_h)
            }
        } else {
            (source_w, source_h)
        };
    new_w -= new_w % 32;
    new_h -= new_h % 32;
    (new_w.max(32), new_h.max(32))
}

/// Nearest-neighbor resize of an RGBA buffer's RGB channels into MODNet's expected NCHW,
/// `(pixel/255 - 0.5) / 0.5`-normalized float layout at `(model_w, model_h)`.
fn preprocess(rgba: &[u8], width: u32, height: u32, model_w: u32, model_h: u32) -> Vec<f32> {
    let mut chw = vec![0f32; 3 * (model_w * model_h) as usize];
    let plane = (model_w * model_h) as usize;
    for y in 0..model_h {
        let src_y = (y * height / model_h).min(height - 1);
        for x in 0..model_w {
            let src_x = (x * width / model_w).min(width - 1);
            let src = ((src_y * width + src_x) * 4) as usize;
            let dst = (y * model_w + x) as usize;
            chw[dst] = (rgba[src] as f32 / 255.0 - 0.5) / 0.5;
            chw[plane + dst] = (rgba[src + 1] as f32 / 255.0 - 0.5) / 0.5;
            chw[2 * plane + dst] = (rgba[src + 2] as f32 / 255.0 - 0.5) / 0.5;
        }
    }
    chw
}

/// Nearest-neighbor resize of a single-channel matte from `(model_w, model_h)` back up to
/// `(width, height)` — the source frame's own resolution.
fn resize_matte(matte: &[f32], model_w: u32, model_h: u32, width: u32, height: u32) -> Vec<f32> {
    let mut out = vec![0f32; (width * height) as usize];
    for y in 0..height {
        let src_y = (y * model_h / height).min(model_h - 1);
        for x in 0..width {
            let src_x = (x * model_w / width).min(model_w - 1);
            out[(y * width + x) as usize] = matte[(src_y * model_w + src_x) as usize];
        }
    }
    out
}

/// Runs MODNet against one decoded RGBA frame and returns a per-pixel alpha matte
/// (`width * height` values, `0.0` = background, `1.0` = foreground/person), same resolution as
/// the input frame. `model_path` is a MODNet ONNX file (see module docs for where to get one).
pub fn segment_person(
    model_path: &Path,
    rgba: &[u8],
    width: u32,
    height: u32,
) -> Result<Vec<f32>, SegmentError> {
    use ort::session::Session;
    use ort::value::Value;

    let mut session = Session::builder()
        .map_err(|e| SegmentError::Onnx(e.to_string()))?
        .commit_from_file(model_path)
        .map_err(|e| SegmentError::Onnx(e.to_string()))?;

    let (model_w, model_h) = model_input_size(width, height);
    let input_data = preprocess(rgba, width, height, model_w, model_h);
    let input = Value::from_array(([1, 3, model_h as usize, model_w as usize], input_data))
        .map_err(|e| SegmentError::Onnx(e.to_string()))?;

    let outputs = session
        .run(ort::inputs![input])
        .map_err(|e| SegmentError::Onnx(e.to_string()))?;

    let (_name, value) = outputs
        .iter()
        .next()
        .ok_or_else(|| SegmentError::Onnx("model produced no output tensors".into()))?;
    let (_shape, data) = value
        .try_extract_tensor::<f32>()
        .map_err(|e| SegmentError::Onnx(e.to_string()))?;

    Ok(resize_matte(
        &data.to_vec(),
        model_w,
        model_h,
        width,
        height,
    ))
}

#[cfg(test)]
#[path = "background_removal/background_removal_test.rs"]
mod tests;
