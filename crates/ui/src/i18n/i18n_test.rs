// Copyright (C) 2026 by Lucas Gomes <lucasgsm88@gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 2 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use super::*;

#[test]
fn every_text_variant_is_non_empty_in_every_locale() {
    for &text in Text::ALL {
        for &locale in &Locale::ALL {
            assert!(
                !text.tr(locale).is_empty(),
                "{text:?} is empty for {locale:?}"
            );
        }
    }
}

#[test]
fn each_locale_lists_its_own_name() {
    assert_eq!(Locale::PtBr.native_name(), "Português");
    assert_eq!(Locale::En.native_name(), "English");
}

#[test]
fn recency_uses_singular_for_exactly_one_unit() {
    assert_eq!(
        recency_label(Locale::PtBr, Recency::HoursAgo(1)),
        "Editado há 1 hora"
    );
    assert_eq!(
        recency_label(Locale::En, Recency::HoursAgo(1)),
        "Edited 1 hour ago"
    );
    assert_eq!(
        recency_label(Locale::PtBr, Recency::DaysAgo(1)),
        "Editado há 1 dia"
    );
    assert_eq!(
        recency_label(Locale::En, Recency::DaysAgo(1)),
        "Edited 1 day ago"
    );
}

#[test]
fn recency_uses_plural_beyond_one_unit() {
    assert_eq!(
        recency_label(Locale::PtBr, Recency::HoursAgo(2)),
        "Editado há 2 horas"
    );
    assert_eq!(
        recency_label(Locale::En, Recency::HoursAgo(5)),
        "Edited 5 hours ago"
    );
    assert_eq!(
        recency_label(Locale::PtBr, Recency::DaysAgo(3)),
        "Editado há 3 dias"
    );
}

#[test]
fn recency_yesterday_has_a_fixed_label() {
    assert_eq!(
        recency_label(Locale::PtBr, Recency::Yesterday),
        "Editado ontem"
    );
    assert_eq!(
        recency_label(Locale::En, Recency::Yesterday),
        "Edited yesterday"
    );
}

#[test]
fn track_summary_uses_singular_clip_for_exactly_one() {
    let tracks = vec!["V1".to_string(), "A1".to_string()];
    assert_eq!(track_summary(Locale::PtBr, 1, &tracks), "1 clipe · V1/A1");
    assert_eq!(track_summary(Locale::En, 1, &tracks), "1 clip · V1/A1");
}

#[test]
fn track_summary_uses_plural_clips_otherwise() {
    let tracks = vec!["V1".to_string(), "A1".to_string(), "A2".to_string()];
    assert_eq!(
        track_summary(Locale::PtBr, 4, &tracks),
        "4 clipes · V1/A1/A2"
    );
    assert_eq!(track_summary(Locale::En, 0, &tracks), "0 clips · V1/A1/A2");
}

fn job(status: ExportJobStatus) -> ExportJob {
    ExportJob {
        id: 1,
        title: "Test job".to_string(),
        segments: Vec::new(),

        text_segments: vec![],
        shape_segments: vec![],

        track_segments: Vec::new(),
        audio_segments: Vec::new(),

        canvas: avcore::Canvas {
            width: 1920,
            height: 1080,
            fps_num: 30,
            fps_den: 1,
            bit_rate_bps: 42_000_000,
        },
        target_lufs: -14.0,
        output_path: "/export/test.mp4".to_string(),
        status,
    }
}

#[test]
fn job_status_label_covers_every_status() {
    assert_eq!(
        job_status_label(Locale::PtBr, &ExportJobStatus::Queued),
        "Na fila"
    );
    assert_eq!(
        job_status_label(Locale::PtBr, &ExportJobStatus::Rendering { percent: 10 }),
        "Renderizando"
    );
    assert_eq!(
        job_status_label(Locale::PtBr, &ExportJobStatus::Paused { percent: 10 }),
        "Pausado"
    );
    assert_eq!(
        job_status_label(Locale::PtBr, &ExportJobStatus::Done),
        "Concluído"
    );
    assert_eq!(
        job_status_label(
            Locale::PtBr,
            &ExportJobStatus::Failed {
                message: "x".to_string()
            }
        ),
        "Falhou"
    );
}

#[test]
fn job_detail_line_flips_the_negative_lufs_sign_for_display() {
    let line = job_detail_line(Locale::PtBr, &job(ExportJobStatus::Queued));
    assert_eq!(line, "-14 LUFS · 42 Mbps · /export/test.mp4");
}

#[test]
fn job_detail_line_shows_translated_error_prefix_on_failure() {
    let failed = job(ExportJobStatus::Failed {
        message: "disk full".to_string(),
    });
    assert_eq!(job_detail_line(Locale::PtBr, &failed), "Erro: disk full");
    assert_eq!(job_detail_line(Locale::En, &failed), "Error: disk full");
}
