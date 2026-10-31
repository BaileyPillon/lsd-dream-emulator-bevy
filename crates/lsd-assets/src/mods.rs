//! Opt-in, deterministic data replacements. Original simulation is not modified.
//!
//! Packs are snapshotted and hashed at startup. Later packs win; dependencies
//! must precede dependents. Executables, scripts and native plugins are excluded.
use crate::{CANONICAL_SHA256, sha256_hex};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt, fs,
    io::Read,
    path::{Path, PathBuf},
    sync::Arc,
};

const MANIFEST_LIMIT: usize = 64 * 1024;
const FILE_LIMIT: usize = 16 * 1024 * 1024;
const TOTAL_LIMIT: usize = 64 * 1024 * 1024;
const PACK_LIMIT: usize = 32;

/// An invalid, incompatible or unreadable pack; loading never silently falls back.
#[derive(Debug)]
pub struct ModError(String);
impl fmt::Display for ModError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for ModError {}
fn invalid(message: impl Into<String>) -> ModError {
    ModError(message.into())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema_version: u32,
    id: String,
    name: String,
    version: String,
    reference_sha256: String,
    #[serde(default)]
    requires: Vec<String>,
    #[serde(default)]
    conflicts: Vec<String>,
    files: Vec<Replacement>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Replacement {
    target: String,
    source: String,
    sha256: String,
}

/// Portable pack identity. No local filesystem paths or original bytes included.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct LoadedMod {
    pub id: String,
    pub name: String,
    pub version: String,
    pub manifest_sha256: String,
}
/// The winning replacement, including earlier packs it overrides in load order.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AssetOverride {
    pub target: String,
    pub mod_id: String,
    pub sha256: String,
    pub bytes: usize,
    pub replaces: Vec<String>,
}
/// A reproducible profile, separate from original-game acceptance evidence.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ModProfile {
    pub schema_version: u32,
    pub mode: &'static str,
    pub reference_sha256: String,
    pub mods: Vec<LoadedMod>,
    pub overrides: Vec<AssetOverride>,
    pub profile_sha256: String,
}
impl ModProfile {
    /// Compact JSON without local paths or game payloads.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("JSON mod profile")
    }
}

/// Remove explicit pack options while preserving the host's positional arguments.
/// `--original` documents intent and is incompatible with any `--mod` option.
pub fn split_mod_arguments(
    args: impl IntoIterator<Item = String>,
) -> Result<(Vec<String>, Vec<PathBuf>), ModError> {
    let mut args = args.into_iter();
    let mut positional = Vec::new();
    let mut packs = Vec::new();
    let mut original = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--mod" => {
                let path = args
                    .next()
                    .filter(|v| !v.is_empty() && !v.starts_with("--"))
                    .ok_or_else(|| invalid("--mod requires a pack directory"))?;
                packs.push(PathBuf::from(path));
            }
            "--original" => original = true,
            _ => positional.push(arg),
        }
    }
    if original && !packs.is_empty() {
        return Err(invalid("--original cannot be combined with --mod"));
    }
    Ok((positional, packs))
}

/// Shared immutable replacement bytes; base content is read on demand.
#[derive(Debug, Clone)]
pub struct AssetSource {
    root: PathBuf,
    replacements: BTreeMap<String, Arc<[u8]>>,
    profile: ModProfile,
}
impl AssetSource {
    /// Original-only path. No scanning, auto-discovery or pack activation occurs.
    pub fn original(root: &Path) -> Self {
        let mut source = Self {
            root: root.to_owned(),
            replacements: BTreeMap::new(),
            profile: ModProfile {
                schema_version: 1,
                mode: "original",
                reference_sha256: CANONICAL_SHA256.into(),
                mods: Vec::new(),
                overrides: Vec::new(),
                profile_sha256: String::new(),
            },
        };
        source.seal_profile();
        source
    }

    /// Validate packs in explicit low-to-high priority order, then snapshot bytes.
    /// The caller must validate the base executable before constructing gameplay.
    pub fn with_mods(root: &Path, packs: &[PathBuf]) -> Result<Self, ModError> {
        if packs.len() > PACK_LIMIT {
            return Err(invalid("At most 32 mod packs may be active"));
        }
        let mut result = Self::original(root);
        let mut ids = BTreeSet::new();
        let mut forbidden = BTreeSet::new();
        let mut overrides: BTreeMap<String, AssetOverride> = BTreeMap::new();
        let mut total = 0usize;
        for pack in packs {
            let pack = pack
                .canonicalize()
                .map_err(|e| invalid(format!("Mod directory: {e}")))?;
            let manifest_path = contained(&pack, "mod.json")?;
            let raw = bounded_read(&manifest_path, MANIFEST_LIMIT)?;
            let manifest: Manifest = serde_json::from_slice(&raw)
                .map_err(|e| invalid(format!("Invalid mod.json: {e}")))?;
            if manifest.schema_version != 1 || manifest.reference_sha256 != CANONICAL_SHA256 {
                return Err(invalid("Unsupported mod API or reference executable"));
            }
            if manifest.id.is_empty()
                || manifest.id.len() > 64
                || !manifest
                    .id
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"-_.".contains(&b))
                || manifest.name.trim().is_empty()
                || manifest.name.len() > 120
                || manifest.version.trim().is_empty()
                || manifest.version.len() > 64
            {
                return Err(invalid("Invalid mod id, name or version"));
            }
            if ids.contains(&manifest.id) {
                return Err(invalid(format!("Duplicate mod id: {}", manifest.id)));
            }
            if forbidden.contains(&manifest.id)
                || manifest
                    .conflicts
                    .iter()
                    .any(|id| ids.contains(id) || id == &manifest.id)
            {
                return Err(invalid(format!("Conflicting mod: {}", manifest.id)));
            }
            for required in &manifest.requires {
                if !ids.contains(required) {
                    return Err(invalid(format!(
                        "{} requires earlier mod {required}",
                        manifest.id
                    )));
                }
            }
            if manifest.files.len() > 128 {
                return Err(invalid("A pack may declare at most 128 replacements"));
            }
            let mut targets = BTreeSet::new();
            for replacement in manifest.files {
                let target = normalized(&replacement.target)?;
                let extension = target.rsplit('.').next().unwrap_or("");
                if !target.starts_with("CDI/")
                    || !matches!(
                        extension,
                        "TIX" | "TIM" | "TMD" | "LBD" | "MOM" | "VH" | "VB" | "SEQ"
                    )
                {
                    return Err(invalid(format!("Unsupported replacement target: {target}")));
                }
                if !targets.insert(target.clone()) {
                    return Err(invalid(format!("Duplicate replacement target: {target}")));
                }
                normalized(&replacement.source)?;
                let path = contained(&pack, &replacement.source.replace('\\', "/"))?;
                let data = bounded_read(&path, FILE_LIMIT)?;
                total = total
                    .checked_add(data.len())
                    .ok_or_else(|| invalid("Mod byte limit"))?;
                if total > TOTAL_LIMIT {
                    return Err(invalid("Active mod files exceed 64 MiB"));
                }
                let sha256 = sha256_hex(&data);
                if replacement.sha256 != sha256 {
                    return Err(invalid(format!("Replacement SHA-256 mismatch: {target}")));
                }
                let mut replaces = overrides.remove(&target).map_or_else(Vec::new, |old| {
                    let mut previous = old.replaces;
                    previous.push(old.mod_id);
                    previous
                });
                replaces.shrink_to_fit();
                overrides.insert(
                    target.clone(),
                    AssetOverride {
                        target: target.clone(),
                        mod_id: manifest.id.clone(),
                        sha256,
                        bytes: data.len(),
                        replaces,
                    },
                );
                result.replacements.insert(target, Arc::from(data));
            }
            forbidden.extend(manifest.conflicts);
            ids.insert(manifest.id.clone());
            result.profile.mods.push(LoadedMod {
                id: manifest.id,
                name: manifest.name,
                version: manifest.version,
                manifest_sha256: sha256_hex(&raw),
            });
        }
        result.profile.overrides = overrides.into_values().collect();
        if !packs.is_empty() {
            result.profile.mode = "modded";
        }
        result.seal_profile();
        Ok(result)
    }

    /// Read original data or its verified replacement. Packs cannot redirect EXE reads.
    pub fn read(&self, relative: impl AsRef<Path>) -> Result<Vec<u8>, ModError> {
        let text = relative
            .as_ref()
            .to_str()
            .ok_or_else(|| invalid("Non-UTF8 asset path"))?;
        let key = normalized(text)?;
        if let Some(bytes) = self.replacements.get(&key) {
            return Ok(bytes.to_vec());
        }
        fs::read(self.root.join(text.replace('\\', "/")))
            .map_err(|e| invalid(format!("Read asset {key}: {e}")))
    }

    /// The deterministic profile is suitable for logs and compatibility reports.
    pub fn profile(&self) -> &ModProfile {
        &self.profile
    }

    fn seal_profile(&mut self) {
        self.profile.profile_sha256.clear();
        self.profile.profile_sha256 =
            sha256_hex(&serde_json::to_vec(&self.profile).expect("JSON profile"));
    }
}

fn bounded_read(path: &Path, limit: usize) -> Result<Vec<u8>, ModError> {
    let file = fs::File::open(path).map_err(|e| invalid(format!("Read mod file: {e}")))?;
    if !file
        .metadata()
        .map_err(|e| invalid(e.to_string()))?
        .is_file()
    {
        return Err(invalid("Mod payload must be a regular file"));
    }
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| invalid(e.to_string()))?;
    if bytes.len() > limit {
        return Err(invalid("Mod file exceeds size limit"));
    }
    Ok(bytes)
}
fn contained(root: &Path, relative: &str) -> Result<PathBuf, ModError> {
    normalized(relative)?;
    let path = root
        .join(relative)
        .canonicalize()
        .map_err(|e| invalid(format!("Missing mod file: {e}")))?;
    if !path.starts_with(root) {
        return Err(invalid("Mod symlink escapes its directory"));
    }
    Ok(path)
}
fn normalized(path: &str) -> Result<String, ModError> {
    let path = path.replace('\\', "/");
    if path.len() > 240
        || path.split('/').any(|part| {
            part.is_empty()
                || part == "."
                || part == ".."
                || part.ends_with('.')
                || !part
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
                || matches!(
                    part.split('.')
                        .next()
                        .unwrap_or("")
                        .to_ascii_uppercase()
                        .as_str(),
                    "CON"
                        | "PRN"
                        | "AUX"
                        | "NUL"
                        | "COM1"
                        | "COM2"
                        | "COM3"
                        | "COM4"
                        | "COM5"
                        | "COM6"
                        | "COM7"
                        | "COM8"
                        | "COM9"
                        | "LPT1"
                        | "LPT2"
                        | "LPT3"
                        | "LPT4"
                        | "LPT5"
                        | "LPT6"
                        | "LPT7"
                        | "LPT8"
                        | "LPT9"
                )
        })
    {
        return Err(invalid(
            "Asset paths must be portable relative paths without traversal",
        ));
    }
    Ok(path.to_ascii_uppercase())
}
