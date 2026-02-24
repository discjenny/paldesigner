use anyhow::{Context, Result, bail};
use std::collections::BTreeSet;
use std::io::Cursor;
use std::path::{Component, Path, PathBuf};
use zip::ZipArchive;

#[derive(Debug, Clone)]
pub struct ZipEntry {
    pub path: String,
    pub bytes: Vec<u8>,
}

pub fn parse_zip_entries(zip_bytes: &[u8]) -> Result<Vec<ZipEntry>> {
    let reader = Cursor::new(zip_bytes);
    let mut archive =
        ZipArchive::new(reader).with_context(|| "uploaded file is not a readable ZIP archive")?;
    let mut entries = Vec::new();

    for index in 0..archive.len() {
        let mut file = archive
            .by_index(index)
            .with_context(|| format!("failed to read ZIP entry at index {}", index))?;
        if file.is_dir() {
            continue;
        }

        let path = sanitize_zip_path(file.name())?;
        let mut bytes = Vec::new();
        std::io::Read::read_to_end(&mut file, &mut bytes)
            .with_context(|| format!("failed to read ZIP entry bytes for {}", path))?;
        entries.push(ZipEntry { path, bytes });
    }

    if entries.is_empty() {
        bail!("ZIP archive contains no files");
    }

    Ok(entries)
}

pub fn detect_world_root(entries: &[ZipEntry]) -> Result<String> {
    let paths: BTreeSet<String> = entries.iter().map(|entry| entry.path.clone()).collect();
    let mut candidates = BTreeSet::new();

    for path in &paths {
        if path.ends_with("Level.sav") {
            let root = Path::new(path)
                .parent()
                .map(|value| value.to_string_lossy().replace('\\', "/"))
                .unwrap_or_default();
            candidates.insert(root);
        }
    }

    if candidates.is_empty() {
        bail!("ZIP does not contain Level.sav in any path");
    }

    let mut ordered: Vec<String> = candidates.into_iter().collect();
    ordered.sort_by(|a, b| {
        let depth_a = a.matches('/').count();
        let depth_b = b.matches('/').count();
        depth_a.cmp(&depth_b).then_with(|| a.cmp(b))
    });

    let mut found_player_singular_only = false;
    for root in ordered {
        let level_path = prefixed(&root, "Level.sav");
        let players_prefix = prefixed(&root, "Players/");
        let player_prefix = prefixed(&root, "Player/");

        let has_level = paths.contains(&level_path);
        let has_players = paths
            .iter()
            .any(|path| path.starts_with(&players_prefix) && path.ends_with(".sav"));
        let has_player_singular = paths
            .iter()
            .any(|path| path.starts_with(&player_prefix) && path.ends_with(".sav"));

        if has_level && has_players && !has_player_singular {
            return Ok(root);
        }
        if has_level && has_player_singular && !has_players {
            found_player_singular_only = true;
        }
    }

    if found_player_singular_only {
        bail!("ZIP uses Player/ directory; only Players/ is supported");
    }
    bail!("ZIP does not contain a valid world root with Level.sav and Players/*.sav");
}

pub fn strip_root_prefix(root: &str, full_path: &str) -> Option<String> {
    if root.is_empty() {
        return Some(full_path.to_string());
    }

    let prefix = format!("{}/", root);
    full_path
        .strip_prefix(&prefix)
        .map(std::string::ToString::to_string)
}

pub fn is_supported_world_file(relative_path: &str) -> bool {
    matches!(
        relative_path,
        "Level.sav" | "LevelMeta.sav" | "LocalData.sav" | "WorldOption.sav"
    ) || (relative_path.starts_with("Players/") && relative_path.ends_with(".sav"))
}

fn sanitize_zip_path(raw: &str) -> Result<String> {
    let normalized = raw.replace('\\', "/");
    let mut cleaned = PathBuf::new();

    for component in Path::new(&normalized).components() {
        match component {
            Component::Normal(value) => cleaned.push(value),
            Component::CurDir => {}
            Component::Prefix(_) | Component::RootDir | Component::ParentDir => {
                bail!("ZIP entry path is unsafe: {}", raw)
            }
        }
    }

    if cleaned.as_os_str().is_empty() {
        bail!("ZIP entry path is empty: {}", raw);
    }

    Ok(cleaned.to_string_lossy().replace('\\', "/"))
}

fn prefixed(root: &str, suffix: &str) -> String {
    if root.is_empty() {
        suffix.to_string()
    } else {
        format!("{}/{}", root, suffix)
    }
}
