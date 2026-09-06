use crate::screens::editor::timeline_panel::{ClipDrag, TrimEdge};

/// Deferred clip and overlay mutations collected while timeline widgets are painted.
#[derive(Default)]
pub(super) struct ClipRequests {
    pub(super) clicked_clip_id: Option<u64>,
    pub(super) clicked_text_clip_id: Option<u64>,
    pub(super) clicked_shape_clip_id: Option<u64>,
    pub(super) delete_text_clips: Vec<u64>,
    pub(super) delete_shape_clips: Vec<u64>,
    pub(super) deletes: Vec<u64>,
    pub(super) copies: Vec<u64>,
    pub(super) cuts: Vec<u64>,
    pub(super) copy_formatting: Vec<u64>,
    pub(super) paste_formatting: Vec<u64>,
    pub(super) multi_select: Vec<u64>,
    pub(super) color_labels: Vec<(u64, Option<[u8; 3]>)>,
    pub(super) detach_audio: Vec<u64>,
    pub(super) speed_ramps: Vec<(u64, f32, f32)>,
    pub(super) custom_speed_ramp: Option<u64>,
    pub(super) create_compound: bool,
    pub(super) open_nested_sequence: Option<u64>,
    pub(super) paste: bool,
    pub(super) merge_into_composite: bool,
    pub(super) split_at_playhead: bool,
    pub(super) trims: Vec<(u64, TrimEdge)>,
    pub(super) drags: Vec<ClipDrag>,
    pub(super) text_drags: Vec<(u64, f64)>,
    pub(super) shape_drags: Vec<(u64, f64)>,
    pub(super) text_trims: Vec<(u64, TrimEdge)>,
    pub(super) shape_trims: Vec<(u64, TrimEdge)>,
}
