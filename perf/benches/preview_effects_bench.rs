// Copyright (C) 2026 by Lucas Gomes <lucasgsm88@gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

//! Benchmarks for `crates/core/src/preview_effects.rs`'s four per-frame CPU-side preview
//! effects, run against a synthetic 1920x1080 RGBA8 buffer — the resolution `App::
//! pump_preview_frame` applies these to on every decoded preview frame at Full HD. See
//! `PLAN.md`/`REPORT.md` for methodology and results.

use std::time::Duration;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use oca_perf::{
    apply_deflicker_to_rgba_after, apply_deflicker_to_rgba_before, apply_glitch_to_rgba_after,
    apply_glitch_to_rgba_before, apply_lut_to_rgba_after, apply_lut_to_rgba_before,
    apply_vignette_to_rgba, identity_cube, DeflickerHistory, Lut3D,
};

const WIDTH: u32 = 1920;
const HEIGHT: u32 = 1080;

fn frame() -> Vec<u8> {
    (0..(WIDTH as usize * HEIGHT as usize * 4))
        .map(|i| (i % 256) as u8)
        .collect()
}

fn bench_lut(c: &mut Criterion) {
    let lut = Lut3D::parse(&identity_cube(17)).unwrap(); // 17^3, a common real-world LUT size.
    let base = frame();
    let mut group = c.benchmark_group("apply_lut_to_rgba (1920x1080)");
    group.bench_function(BenchmarkId::new("before", "chunks_exact_mut"), |b| {
        b.iter_batched(
            || base.clone(),
            |mut rgba| apply_lut_to_rgba_before(black_box(&mut rgba), black_box(&lut)),
            criterion::BatchSize::LargeInput,
        )
    });
    group.bench_function(BenchmarkId::new("after", "as_chunks_mut"), |b| {
        b.iter_batched(
            || base.clone(),
            |mut rgba| apply_lut_to_rgba_after(black_box(&mut rgba), black_box(&lut)),
            criterion::BatchSize::LargeInput,
        )
    });
    group.finish();
}

fn bench_glitch(c: &mut Criterion) {
    let base = frame();
    let mut group = c.benchmark_group("apply_glitch_to_rgba (1920x1080)");
    group.bench_function(BenchmarkId::new("before", "chunks_exact_mut"), |b| {
        b.iter_batched(
            || base.clone(),
            |mut rgba| apply_glitch_to_rgba_before(black_box(&mut rgba), 1.0, black_box(12345)),
            criterion::BatchSize::LargeInput,
        )
    });
    group.bench_function(BenchmarkId::new("after", "as_chunks_mut"), |b| {
        b.iter_batched(
            || base.clone(),
            |mut rgba| apply_glitch_to_rgba_after(black_box(&mut rgba), 1.0, black_box(12345)),
            criterion::BatchSize::LargeInput,
        )
    });
    group.finish();
}

fn bench_deflicker(c: &mut Criterion) {
    let base = frame();
    let mut group = c.benchmark_group("apply_deflicker_to_rgba (1920x1080)");
    group.bench_function(BenchmarkId::new("before", "chunks_exact"), |b| {
        b.iter_batched(
            || (base.clone(), DeflickerHistory::new()),
            |(mut rgba, mut history)| {
                apply_deflicker_to_rgba_before(black_box(&mut rgba), &mut history)
            },
            criterion::BatchSize::LargeInput,
        )
    });
    group.bench_function(BenchmarkId::new("after", "as_chunks"), |b| {
        b.iter_batched(
            || (base.clone(), DeflickerHistory::new()),
            |(mut rgba, mut history)| {
                apply_deflicker_to_rgba_after(black_box(&mut rgba), &mut history)
            },
            criterion::BatchSize::LargeInput,
        )
    });
    group.finish();
}

fn bench_vignette(c: &mut Criterion) {
    let base = frame();
    c.bench_function("apply_vignette_to_rgba (1920x1080, unchanged)", |b| {
        b.iter_batched(
            || base.clone(),
            |mut rgba| apply_vignette_to_rgba(black_box(&mut rgba), WIDTH, HEIGHT, 0.6),
            criterion::BatchSize::LargeInput,
        )
    });
}

criterion_group! {
    name = benches;
    // This sandbox's CPU is noisy (shared/virtualized — confirmed empirically: repeated runs of
    // the identical binary swing +/-25% run to run, see REPORT.md). A longer measurement window
    // and more samples narrow the confidence interval but cannot eliminate that noise floor —
    // treat single-run deltas smaller than ~25% as inconclusive in this environment specifically,
    // not as a property of the code.
    config = Criterion::default()
        .measurement_time(Duration::from_secs(15))
        .sample_size(200);
    targets = bench_lut, bench_glitch, bench_deflicker, bench_vignette
}
criterion_main!(benches);
