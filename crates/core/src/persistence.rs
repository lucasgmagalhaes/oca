//! Saves/loads a [`Project`] as `.ocproj` — a custom binary format: a small magic/version
//! header, then gzip-compressed MessagePack (struct-map mode). Struct-map mode keeps field
//! names in the encoded bytes (unlike MessagePack's default compact/array mode), so
//! `#[serde(default)]` on a field added after a project was last saved still works the same
//! way it did for JSON — nearly every past feature in this codebase relies on that to let
//! older saved projects keep loading. The actual (de)serialization is a pure function
//! ([`to_ocproj_bytes`]/[`from_ocproj_bytes`]), and the file-touching wrappers
//! ([`save_project_to_file`]/[`load_project_from_file`]) are thin shells around it.

use std::fs;
use std::io::{Read, Write};
use std::path::Path;

use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use serde::Serialize;

use crate::project::Project;

/// Identifies an `.ocproj` file before we even try to gzip/msgpack-decode it, so a
/// wrong-file/corrupt-file error says exactly that instead of surfacing an opaque gzip or
/// msgpack failure.
const MAGIC: &[u8; 4] = b"OCPJ";
/// Bumped only if the on-disk layout (header framing, not the `Project` schema itself —
/// that's handled by `#[serde(default)]`) ever needs to change.
const FORMAT_VERSION: u8 = 1;

#[derive(Debug)]
pub enum PersistError {
    Io(std::io::Error),
    Encode(rmp_serde::encode::Error),
    Decode(rmp_serde::decode::Error),
    /// Bytes that don't start with [`MAGIC`], or whose version byte isn't [`FORMAT_VERSION`].
    Corrupt(&'static str),
}

impl std::fmt::Display for PersistError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PersistError::Io(e) => write!(f, "failed to access the project file: {e}"),
            PersistError::Encode(e) => write!(f, "failed to serialize the project: {e}"),
            PersistError::Decode(e) => write!(f, "failed to parse the project: {e}"),
            PersistError::Corrupt(msg) => write!(f, "not a valid .ocproj file: {msg}"),
        }
    }
}

impl std::error::Error for PersistError {}

/// Serializes `project` to the `.ocproj` byte layout: `MAGIC` + version byte, then a gzip
/// stream wrapping MessagePack (struct-map) bytes.
pub fn to_ocproj_bytes(project: &Project) -> Result<Vec<u8>, PersistError> {
    let mut msgpack = Vec::new();
    project
        .serialize(&mut rmp_serde::Serializer::new(&mut msgpack).with_struct_map())
        .map_err(PersistError::Encode)?;

    let mut out = Vec::with_capacity(msgpack.len() / 2 + 5);
    out.extend_from_slice(MAGIC);
    out.push(FORMAT_VERSION);
    let mut encoder = GzEncoder::new(&mut out, Compression::fast());
    encoder.write_all(&msgpack).map_err(PersistError::Io)?;
    encoder.finish().map_err(PersistError::Io)?;
    Ok(out)
}

/// Parses a project previously produced by [`to_ocproj_bytes`].
pub fn from_ocproj_bytes(bytes: &[u8]) -> Result<Project, PersistError> {
    if bytes.len() < MAGIC.len() + 1 || &bytes[..MAGIC.len()] != MAGIC {
        return Err(PersistError::Corrupt("bad magic bytes"));
    }
    let version = bytes[MAGIC.len()];
    if version != FORMAT_VERSION {
        return Err(PersistError::Corrupt("unsupported format version"));
    }
    let mut msgpack = Vec::new();
    GzDecoder::new(&bytes[MAGIC.len() + 1..])
        .read_to_end(&mut msgpack)
        .map_err(PersistError::Io)?;
    rmp_serde::from_slice(&msgpack).map_err(PersistError::Decode)
}

/// Writes `project` to `path` as `.ocproj`, overwriting any existing file.
pub fn save_project_to_file(project: &Project, path: &Path) -> Result<(), PersistError> {
    let bytes = to_ocproj_bytes(project)?;
    fs::write(path, bytes).map_err(PersistError::Io)
}

/// Reads and parses a project from `path`. The returned [`Project::file_path`] is `None` —
/// callers that want it populated (so subsequent saves go back to the same file) should set
/// it themselves, since only the caller knows whether `path` should be remembered.
pub fn load_project_from_file(path: &Path) -> Result<Project, PersistError> {
    let bytes = fs::read(path).map_err(PersistError::Io)?;
    from_ocproj_bytes(&bytes)
}
