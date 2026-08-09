use std::collections::HashSet;

use nivela_core::sample::{sample_export_jobs, sample_projects};

#[test]
fn sample_projects_is_non_empty() {
    assert!(!sample_projects().is_empty());
}

#[test]
fn media_asset_ids_are_unique_within_each_project() {
    for project in sample_projects() {
        let mut seen = HashSet::new();
        for asset in &project.media_library {
            assert!(
                seen.insert(asset.id),
                "duplicate MediaAsset id {} in project {:?}",
                asset.id,
                project.name
            );
        }
    }
}

#[test]
fn every_timeline_clip_references_an_asset_in_the_same_project() {
    for project in sample_projects() {
        let asset_ids: HashSet<u64> = project.media_library.iter().map(|a| a.id).collect();
        for track in &project.timeline.tracks {
            for clip in &track.clips {
                assert!(
                    asset_ids.contains(&clip.asset_id),
                    "clip {} on track {:?} references missing asset {} in project {:?}",
                    clip.id,
                    track.name,
                    clip.asset_id,
                    project.name
                );
            }
        }
    }
}

#[test]
fn sample_export_jobs_have_non_empty_titles_and_output_paths() {
    let jobs = sample_export_jobs();
    assert!(!jobs.is_empty());
    for job in &jobs {
        assert!(!job.title.is_empty());
        assert!(!job.output_path.is_empty());
    }
}
