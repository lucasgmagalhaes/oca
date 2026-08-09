//! Saves/loads a [`Project`] as JSON — the format Fase 1 calls for. Same split as
//! [`crate::probe`]/[`crate::loudness`]: the actual (de)serialization is a pure function
//! ([`to_json`]/[`from_json`]), and the file-touching wrappers ([`save_project_to_file`]/
//! [`load_project_from_file`]) are thin shells around it.

use std::fs;
use std::path::Path;

use crate::project::Project;

#[derive(Debug)]
pub enum PersistError {
    Io(std::io::Error),
    Json(serde_json::Error),
}

impl std::fmt::Display for PersistError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PersistError::Io(e) => write!(f, "failed to access the project file: {e}"),
            PersistError::Json(e) => write!(f, "failed to (de)serialize the project: {e}"),
        }
    }
}

impl std::error::Error for PersistError {}

/// Serializes `project` to pretty-printed JSON.
pub fn to_json(project: &Project) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(project)
}

/// Parses a project previously produced by [`to_json`] (or hand-edited — it's plain JSON).
pub fn from_json(json: &str) -> Result<Project, serde_json::Error> {
    serde_json::from_str(json)
}

/// Writes `project` to `path` as JSON, overwriting any existing file.
pub fn save_project_to_file(project: &Project, path: &Path) -> Result<(), PersistError> {
    let json = to_json(project).map_err(PersistError::Json)?;
    fs::write(path, json).map_err(PersistError::Io)
}

/// Reads and parses a project from `path`. The returned [`Project::file_path`] is `None` —
/// callers that want it populated (so subsequent saves go back to the same file) should set
/// it themselves, since only the caller knows whether `path` should be remembered.
pub fn load_project_from_file(path: &Path) -> Result<Project, PersistError> {
    let json = fs::read_to_string(path).map_err(PersistError::Io)?;
    from_json(&json).map_err(PersistError::Json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sample::sample_projects;

    #[test]
    fn round_trips_a_project_through_json() {
        let original = sample_projects().into_iter().next().unwrap();
        let json = to_json(&original).unwrap();
        let restored = from_json(&json).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn round_trips_a_project_with_an_empty_timeline_and_library() {
        let original = sample_projects().into_iter().nth(2).unwrap();
        assert!(original.media_library.is_empty());
        assert!(original.timeline.tracks.is_empty());
        let json = to_json(&original).unwrap();
        let restored = from_json(&json).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn from_json_rejects_malformed_input() {
        assert!(from_json("not json").is_err());
    }

    #[test]
    fn file_path_is_not_part_of_the_serialized_json() {
        let mut project = sample_projects().into_iter().next().unwrap();
        project.file_path = Some("/tmp/whatever.json".into());
        let json = to_json(&project).unwrap();
        assert!(!json.contains("whatever.json"));
        assert!(!json.contains("file_path"));
    }

    #[test]
    fn save_then_load_round_trips_through_a_real_file() {
        let original = sample_projects().into_iter().next().unwrap();
        let path = std::env::temp_dir().join(format!("nivela_persist_test_{}.json", original.id));

        save_project_to_file(&original, &path).unwrap();
        let loaded = load_project_from_file(&path).unwrap();
        let _ = fs::remove_file(&path);

        assert_eq!(original, loaded);
    }

    #[test]
    fn load_project_from_file_errors_on_a_missing_file() {
        let path = std::env::temp_dir().join("nivela_persist_test_does_not_exist.json");
        assert!(matches!(load_project_from_file(&path), Err(PersistError::Io(_))));
    }
}
