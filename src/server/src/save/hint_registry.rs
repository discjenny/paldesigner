use crate::save::paltypes::{DISABLED_PROPERTIES, PALWORLD_TYPE_HINTS};
use std::collections::{HashMap, HashSet};
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use tracing::warn;

pub fn normalize_hint_path(path: &str) -> String {
    path.strip_prefix('.').unwrap_or(path).to_string()
}

pub fn mirrored_hint_map() -> HashMap<String, String> {
    let disabled: std::collections::HashSet<String> = DISABLED_PROPERTIES
        .iter()
        .map(|path| normalize_hint_path(path))
        .collect();

    PALWORLD_TYPE_HINTS
        .iter()
        .map(|(path, ty)| (normalize_hint_path(path), (*ty).to_string()))
        .filter(|(path, _)| !disabled.contains(path))
        .collect()
}

pub fn hint_cache() -> &'static Mutex<HashMap<String, String>> {
    static CACHE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn merged_hints_with_cache() -> HashMap<String, String> {
    let mut hints = mirrored_hint_map();
    hints.extend(load_persisted_hints());
    if let Ok(cache) = hint_cache().lock() {
        hints.extend(cache.clone());
    }
    hints
}

pub fn cache_discovered_hint(path: String, ty: String) {
    let normalized_path = normalize_hint_path(&path);
    if let Ok(mut cache) = hint_cache().lock() {
        cache.insert(normalized_path.clone(), ty.clone());
    }
    if let Err(error) = persist_discovered_hint(&normalized_path, &ty) {
        warn!(
            path = %normalized_path,
            "failed to persist discovered hint: {error}"
        );
    }
}

fn discovery_file_path() -> PathBuf {
    if let Ok(path) = std::env::var("PALDESIGNER_HINT_DISCOVERY_FILE") {
        return PathBuf::from(path);
    }

    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("data")
        .join("discovered_hint_paths.txt")
}

fn load_persisted_hints() -> HashMap<String, String> {
    static PERSISTED_HINTS: OnceLock<HashMap<String, String>> = OnceLock::new();
    PERSISTED_HINTS
        .get_or_init(|| {
            let path = discovery_file_path();
            let Ok(file) = std::fs::File::open(&path) else {
                return HashMap::new();
            };
            let reader = BufReader::new(file);
            let mut hints = HashMap::<String, String>::new();

            for line in reader.lines().map_while(Result::ok) {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }

                let Some((raw_path, raw_ty)) = trimmed.split_once('|') else {
                    continue;
                };
                let path = normalize_hint_path(raw_path.trim());
                let ty = raw_ty.trim();
                if path.is_empty() || ty.is_empty() {
                    continue;
                }
                hints.insert(path, ty.to_string());
            }
            hints
        })
        .clone()
}

fn persisted_hint_set() -> &'static Mutex<HashSet<String>> {
    static SET: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    SET.get_or_init(|| {
        let mut keys = HashSet::<String>::new();
        for (path, ty) in load_persisted_hints() {
            keys.insert(format!("{path}|{ty}"));
        }
        Mutex::new(keys)
    })
}

fn persist_discovered_hint(path: &str, ty: &str) -> Result<(), String> {
    let key = format!("{path}|{ty}");
    {
        let mut set = persisted_hint_set()
            .lock()
            .map_err(|error| format!("persisted hint set lock poisoned: {error}"))?;
        if !set.insert(key.clone()) {
            return Ok(());
        }
    }

    let file_path = discovery_file_path();
    if let Some(parent) = file_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create hint discovery dir: {error}"))?;
    }

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&file_path)
        .map_err(|error| format!("failed to open hint discovery file: {error}"))?;
    writeln!(file, "{key}").map_err(|error| format!("failed to append discovered hint: {error}"))
}
