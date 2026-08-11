//! Tracks performance of the engine's actual hot paths: parsing `loudnorm` filter stderr and
//! (de)serializing a project. Run with `cargo bench -p core` (or `make bench`) and
//! compare against a previous run's `target/criterion/` report to catch regressions as the
//! engine grows — this is the "acompanhar o desempenho da aplicação" half of the ask; the
//! test suite (`cargo test`) is the correctness half.

use std::hint::black_box;
use std::path::PathBuf;

use criterion::{criterion_group, criterion_main, Criterion};

use avcore::loudness::parse_loudnorm_stderr;
use avcore::persistence::{from_json, to_json};
use avcore::timeline::{ClipInstance, Timeline, Track, TrackKind};
use avcore::{LoudnessMetrics, MediaAsset, MediaKind, Project, Recency};

const LOUDNORM_STDERR_FIXTURE: &str = r#"
ffmpeg version 6.0 Copyright (c) 2000-2023 the FFmpeg developers
  built with gcc 12.2.0
Input #0, mov,mp4,m4a,3gp,3g2,mj2, from 'boss03_ribby_croaks.mp4':
  Duration: 00:02:14.23, start: 0.000000, bitrate: 42021 kb/s
[Parsed_loudnorm_0 @ 0000020a1b2c3d40]
{
	"input_i" : "-19.40",
	"input_tp" : "-3.20",
	"input_lra" : "8.10",
	"input_thresh" : "-29.80",
	"output_i" : "-16.02",
	"output_tp" : "-1.50",
	"output_lra" : "7.00",
	"output_thresh" : "-26.40",
	"normalization_type" : "dynamic",
	"target_offset" : "0.00"
}
frame=  8043 fps=812 q=-1.0 Lsize=N/A time=00:02:14.20 bitrate=N/A speed=101x
video:0kB audio:0kB subtitle:0kB other streams:0kB global headers:0kB muxing overhead: unknown
"#;

/// A project sized like a long-running edit session (Minecraft-longplay-into-many-shorts
/// territory) rather than the small mock projects in `avcore::sample` — big enough for
/// serialization cost and timeline math to actually show up in a profile.
fn large_project(asset_count: usize, clips_per_track: usize) -> Project {
    let media_library: Vec<MediaAsset> = (0..asset_count)
        .map(|i| MediaAsset {
            id: i as u64,
            file_name: format!("clip_{i:04}.mp4"),
            source_path: PathBuf::from(format!("/media/clip_{i:04}.mp4")),
            kind: MediaKind::Video,
            duration_secs: 30.0,
            codec: "h264".to_string(),
            source_bitrate_mbps: 42.0,
            resolution: Some((1920, 1080)),
            fps: Some(60.0),
            sample_rate_khz: None,
            loudness: Some(LoudnessMetrics {
                integrated_lufs: -16.0,
                true_peak_dbtp: -1.5,
                loudness_range_lu: 7.0,
            }),
            proxy_path: None,
            waveform_peaks: None,
        })
        .collect();

    let make_track = |id: u64, name: &str, kind: TrackKind| Track {
        id,
        name: name.to_string(),
        kind,
        clips: (0..clips_per_track)
            .map(|i| ClipInstance {
                id: i as u64,
                asset_id: (i % asset_count.max(1)) as u64,
                start_secs: i as f64 * 30.0,
                source_in_secs: 0.0,
                source_out_secs: 30.0,
            })
            .collect(),
    };

    Project {
        id: 1,
        name: "Bench Project".to_string(),
        last_edited: Recency::HoursAgo(1),
        summary: "Synthetic project for benchmarking.".to_string(),
        media_library,
        timeline: Timeline {
            tracks: vec![
                make_track(1, "V1", TrackKind::Video),
                make_track(2, "A1", TrackKind::Audio),
                make_track(3, "A2", TrackKind::Audio),
            ],
            playhead_secs: 0.0,
        },
        file_path: None,
    }
}

fn bench_parse_loudnorm_stderr(c: &mut Criterion) {
    c.bench_function("loudness::parse_loudnorm_stderr", |b| {
        b.iter(|| parse_loudnorm_stderr(black_box(LOUDNORM_STDERR_FIXTURE)).unwrap())
    });
}

fn bench_project_json_round_trip(c: &mut Criterion) {
    let mut group = c.benchmark_group("persistence");

    for clip_count in [10usize, 200, 1000] {
        let project = large_project(50, clip_count);
        let json = to_json(&project).unwrap();

        group.bench_function(format!("to_json/{clip_count}_clips_per_track"), |b| {
            b.iter(|| to_json(black_box(&project)).unwrap())
        });
        group.bench_function(format!("from_json/{clip_count}_clips_per_track"), |b| {
            b.iter(|| from_json(black_box(&json)).unwrap())
        });
    }

    group.finish();
}

fn bench_timeline_duration(c: &mut Criterion) {
    let mut group = c.benchmark_group("timeline_duration_secs");

    for clip_count in [10usize, 200, 1000] {
        let project = large_project(50, clip_count);
        group.bench_function(format!("{clip_count}_clips_per_track"), |b| {
            b.iter(|| black_box(&project.timeline).duration_secs())
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_parse_loudnorm_stderr,
    bench_project_json_round_trip,
    bench_timeline_duration,
);
criterion_main!(benches);
