//! Mock data mirroring the HTML mockup (`ui.html` / `Editor de Video - Mockups.dc.html`).
//! Fase 1 replaces this with `ffprobe` probing and a loaded/saved project JSON file.

use std::path::PathBuf;

use crate::export::{ExportJob, ExportJobStatus};
use crate::media::{LoudnessMetrics, MediaAsset, MediaKind};
use crate::project::{Project, Recency, Sequence};
use crate::timeline::{ClipInstance, ColorFilter, MaskShape, Timeline, Track, TrackKind};

fn asset(
    id: u64,
    file_name: &str,
    kind: MediaKind,
    duration_secs: f64,
    codec: &str,
    source_bitrate_mbps: f32,
    resolution: Option<(u32, u32)>,
    fps: Option<f32>,
    sample_rate_khz: Option<f32>,
    lufs: f32,
) -> MediaAsset {
    MediaAsset {
        id,
        file_name: file_name.to_string(),
        // Mock data has no real file backing it — mirrors the fake output_path strings on
        // the sample export jobs below.
        source_path: PathBuf::from(format!("/media/{file_name}")),
        kind,
        duration_secs,
        codec: codec.to_string(),
        source_bitrate_mbps,
        resolution,
        fps,
        sample_rate_khz,
        loudness: Some(LoudnessMetrics {
            integrated_lufs: lufs,
            true_peak_dbtp: -3.0,
            loudness_range_lu: 8.0,
        }),
        proxy_path: None,
        waveform_peaks: None,
    }
}

/// The media library for the "Cuphead — 50 Chefes" sample project.
pub fn cuphead_media_library() -> Vec<MediaAsset> {
    vec![
        asset(
            1,
            "boss03_ribby_croaks.mp4",
            MediaKind::Video,
            134.0,
            "H.264",
            42.0,
            Some((1920, 1080)),
            Some(60.0),
            None,
            -19.4,
        ),
        asset(
            2,
            "boss04_captain_brineybeard.mp4",
            MediaKind::Video,
            182.0,
            "H.264",
            38.0,
            Some((1920, 1080)),
            Some(60.0),
            None,
            -17.8,
        ),
        asset(
            3,
            "boss05_wally_warbles.mp4",
            MediaKind::Video,
            341.0,
            "H.264",
            45.0,
            Some((1920, 1080)),
            Some(60.0),
            None,
            -20.1,
        ),
        asset(
            4,
            "commentary_mic.wav",
            MediaKind::Audio,
            134.0,
            "PCM",
            0.0,
            None,
            None,
            Some(48.0),
            -24.0,
        ),
    ]
}

fn cuphead_timeline() -> Timeline {
    Timeline {
        playhead_secs: 42.0 * 60.0 + 10.0,
        tracks: vec![
            Track {
                id: 1,
                name: "V1".to_string(),
                kind: TrackKind::Video,
                clips: vec![
                    ClipInstance {
                        id: 1,
                        asset_id: 1,
                        start_secs: 0.0,
                        source_in_secs: 0.0,
                        source_out_secs: 30.0,
                        composite_id: None,
                        gain_db: 0.0,
                        frozen: false,
                        speed_factor: 1.0,
                        crop_x: 0.0,
                        crop_y: 0.0,
                        crop_w: 1.0,
                        crop_h: 1.0,
                        mask_shape: MaskShape::None,
                        mask_corner_radius: 0.0,
                        flipped_h: false,
                        color_filter: ColorFilter::None,
                    },
                    ClipInstance {
                        id: 2,
                        asset_id: 2,
                        start_secs: 30.0,
                        source_in_secs: 0.0,
                        source_out_secs: 44.0,
                        composite_id: None,
                        gain_db: 0.0,
                        frozen: false,
                        speed_factor: 1.0,
                        crop_x: 0.0,
                        crop_y: 0.0,
                        crop_w: 1.0,
                        crop_h: 1.0,
                        mask_shape: MaskShape::None,
                        mask_corner_radius: 0.0,
                        flipped_h: false,
                        color_filter: ColorFilter::None,
                    },
                    ClipInstance {
                        id: 3,
                        asset_id: 3,
                        start_secs: 74.0,
                        source_in_secs: 0.0,
                        source_out_secs: 24.0,
                        composite_id: None,
                        gain_db: 0.0,
                        frozen: false,
                        speed_factor: 1.0,
                        crop_x: 0.0,
                        crop_y: 0.0,
                        crop_w: 1.0,
                        crop_h: 1.0,
                        mask_shape: MaskShape::None,
                        mask_corner_radius: 0.0,
                        flipped_h: false,
                        color_filter: ColorFilter::None,
                    },
                ],
            },
            Track {
                id: 2,
                name: "A1".to_string(),
                kind: TrackKind::Audio,
                clips: vec![
                    ClipInstance {
                        id: 4,
                        asset_id: 1,
                        start_secs: 0.0,
                        source_in_secs: 0.0,
                        source_out_secs: 30.0,
                        composite_id: None,
                        gain_db: 0.0,
                        frozen: false,
                        speed_factor: 1.0,
                        crop_x: 0.0,
                        crop_y: 0.0,
                        crop_w: 1.0,
                        crop_h: 1.0,
                        mask_shape: MaskShape::None,
                        mask_corner_radius: 0.0,
                        flipped_h: false,
                        color_filter: ColorFilter::None,
                    },
                    ClipInstance {
                        id: 5,
                        asset_id: 2,
                        start_secs: 30.0,
                        source_in_secs: 0.0,
                        source_out_secs: 44.0,
                        composite_id: None,
                        gain_db: 0.0,
                        frozen: false,
                        speed_factor: 1.0,
                        crop_x: 0.0,
                        crop_y: 0.0,
                        crop_w: 1.0,
                        crop_h: 1.0,
                        mask_shape: MaskShape::None,
                        mask_corner_radius: 0.0,
                        flipped_h: false,
                        color_filter: ColorFilter::None,
                    },
                    ClipInstance {
                        id: 6,
                        asset_id: 3,
                        start_secs: 74.0,
                        source_in_secs: 0.0,
                        source_out_secs: 24.0,
                        composite_id: None,
                        gain_db: 0.0,
                        frozen: false,
                        speed_factor: 1.0,
                        crop_x: 0.0,
                        crop_y: 0.0,
                        crop_w: 1.0,
                        crop_h: 1.0,
                        mask_shape: MaskShape::None,
                        mask_corner_radius: 0.0,
                        flipped_h: false,
                        color_filter: ColorFilter::None,
                    },
                ],
            },
            Track {
                id: 3,
                name: "A2".to_string(),
                kind: TrackKind::Audio,
                clips: vec![ClipInstance {
                    id: 7,
                    asset_id: 4,
                    start_secs: 0.0,
                    source_in_secs: 0.0,
                    source_out_secs: 98.0,
                    composite_id: None,
                    gain_db: 0.0,
                    frozen: false,
                    speed_factor: 1.0,
                    crop_x: 0.0,
                    crop_y: 0.0,
                    crop_w: 1.0,
                    crop_h: 1.0,
                    mask_shape: MaskShape::None,
                    mask_corner_radius: 0.0,
                    flipped_h: false,
                    color_filter: ColorFilter::None,
                }],
            },
        ],
    }
}

/// The Início screen's project list: one project per demo scenario (a multi-clip cut
/// project, a single long recording, and an empty one) so the UI has something to render.
pub fn sample_projects() -> Vec<Project> {
    vec![
        Project {
            id: 1,
            name: "Cuphead — 50 Chefes".to_string(),
            last_edited: Recency::HoursAgo(2),
            summary: "Cortes dos boss fights, um vídeo por chefe.".to_string(),
            media_library: cuphead_media_library(),
            sequences: vec![Sequence {
                id: 1,
                name: "Sequência principal".to_string(),
                timeline: cuphead_timeline(),
            }],
            active_sequence: 0,
            file_path: None,
        },
        Project {
            id: 2,
            name: "Minecraft Long Play — Sessão 14".to_string(),
            last_edited: Recency::Yesterday,
            summary: "Gravação de 3h20 pra cortar em partes.".to_string(),
            media_library: vec![asset(
                5,
                "minecraft_ep14_raw.mp4",
                MediaKind::Video,
                3.0 * 3600.0 + 20.0 * 60.0,
                "HEVC",
                60.0,
                Some((1920, 1080)),
                Some(60.0),
                None,
                -15.2,
            )],
            sequences: vec![Sequence {
                id: 1,
                name: "Sequência principal".to_string(),
                timeline: Timeline {
                    tracks: vec![],
                    playhead_secs: 0.0,
                },
            }],
            active_sequence: 0,
            file_path: None,
        },
        Project {
            id: 3,
            name: "Shorts da semana".to_string(),
            last_edited: Recency::DaysAgo(3),
            summary: "Cortes verticais pra Shorts/Reels.".to_string(),
            media_library: vec![],
            sequences: vec![Sequence {
                id: 1,
                name: "Sequência principal".to_string(),
                timeline: Timeline {
                    tracks: vec![],
                    playhead_secs: 0.0,
                },
            }],
            active_sequence: 0,
            file_path: None,
        },
    ]
}

/// The export queue's job list, covering every [`ExportJobStatus`] so the Fila screen has an
/// example of each state (rendering, queued, paused, done, failed) to render. `source_path` is
/// left empty — like `output_path`, these don't correspond to real files on disk, they're
/// display-only demo data (a real job's dispatcher would find no source and fail immediately).
pub fn sample_export_jobs() -> Vec<ExportJob> {
    vec![
        ExportJob {
            id: 1,
            title: "Cuphead — Boss 01: Robô do Rei Dado".to_string(),
            source_path: PathBuf::new(),
            target_lufs: -14.0,
            bitrate_mbps: 42.0,
            output_path: "/export/cuphead/boss01.mp4".to_string(),
            status: ExportJobStatus::Rendering { percent: 62 },
        },
        ExportJob {
            id: 2,
            title: "Cuphead — Boss 02: Goopy Le Grande".to_string(),
            source_path: PathBuf::new(),
            target_lufs: -14.0,
            bitrate_mbps: 38.0,
            output_path: "/export/cuphead/boss02.mp4".to_string(),
            status: ExportJobStatus::Queued,
        },
        ExportJob {
            id: 3,
            title: "Cuphead — Boss 03: Ribby e Croaks".to_string(),
            source_path: PathBuf::new(),
            target_lufs: -14.0,
            bitrate_mbps: 42.0,
            output_path: "/export/cuphead/boss03.mp4".to_string(),
            status: ExportJobStatus::Queued,
        },
        ExportJob {
            id: 4,
            title: "Cuphead — Boss 00: Intro".to_string(),
            source_path: PathBuf::new(),
            target_lufs: -14.0,
            bitrate_mbps: 30.0,
            output_path: "/export/cuphead/boss00.mp4".to_string(),
            status: ExportJobStatus::Paused { percent: 18 },
        },
        ExportJob {
            id: 5,
            title: "Minecraft Long Play — Sessão 14".to_string(),
            source_path: PathBuf::new(),
            target_lufs: -14.0,
            bitrate_mbps: 60.0,
            output_path: "/export/minecraft/ep14.mp4".to_string(),
            status: ExportJobStatus::Done,
        },
        ExportJob {
            id: 6,
            title: "Shorts da semana — Corte 04".to_string(),
            source_path: PathBuf::new(),
            target_lufs: -14.0,
            bitrate_mbps: 20.0,
            output_path: "/export/shorts/".to_string(),
            status: ExportJobStatus::Failed {
                message: "espaço em disco insuficiente em /export/shorts/".to_string(),
            },
        },
    ]
}
