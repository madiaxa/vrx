use crate::{Result, config::Config};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::{
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

const SCHEMA: u32 = 1;
static SERIAL: AtomicU64 = AtomicU64::new(0);
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recent {
    pub path: PathBuf,
    pub fingerprint: u64,
    pub index: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Saved {
    pub schema: u32,
    pub config: Config,
    pub recent: Vec<Recent>,
}
impl Default for Saved {
    fn default() -> Self {
        Self {
            schema: SCHEMA,
            config: Config::default(),
            recent: Vec::new(),
        }
    }
}
impl Saved {
    pub fn remember(&mut self, path: PathBuf, fingerprint: u64, index: usize) {
        self.recent.retain(|r| r.path != path);
        self.recent.insert(
            0,
            Recent {
                path,
                fingerprint,
                index,
            },
        );
        self.recent.truncate(16);
    }
    pub fn position(&self, path: &Path, fingerprint: u64) -> Option<usize> {
        self.recent
            .iter()
            .find(|r| r.path == path && r.fingerprint == fingerprint)
            .map(|r| r.index)
    }
}
pub fn config_path() -> Result<PathBuf> {
    if let Some(directory) = std::env::var_os("VRX_CONFIG_DIR") {
        return Ok(PathBuf::from(directory).join("settings.json"));
    }
    ProjectDirs::from("app", "VRX", "VRX")
        .map(|p| p.config_dir().join("settings.json"))
        .ok_or_else(|| "Cannot locate the application configuration directory".into())
}
pub fn load(path: &Path) -> (Saved, Option<String>) {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return (Saved::default(), None),
        Err(e) => {
            return (
                Saved::default(),
                Some(format!("Settings could not be read: {e}")),
            );
        }
    };
    match serde_json::from_slice::<Saved>(&bytes) {
        Ok(mut saved) if saved.schema == SCHEMA => { saved.config.sanitize(); saved.recent.truncate(16); (saved, None) }
        Ok(_) => (Saved::default(), Some("Settings use an unsupported schema. Defaults loaded; original file retained until you save.".into())),
        Err(e) => (Saved::default(), Some(format!("Settings are corrupt. Defaults loaded: {e}"))),
    }
}
pub fn save(path: &Path, saved: &Saved) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(saved).map_err(|e| e.to_string())?;
    atomic_write(path, &bytes)
}
/// Same-directory replacement, with fsync before rename. Incomplete temporary
/// files never replace a valid destination. Caller serializes concurrent saves.
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("vrx");
    let temporary = parent.join(format!(
        ".{name}.{}.{}.tmp",
        std::process::id(),
        SERIAL.fetch_add(1, Ordering::Relaxed)
    ));
    let mut created = false;
    let result = (|| {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|e| e.to_string())?;
        created = true;
        file.write_all(bytes).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        drop(file);
        std::fs::rename(&temporary, path).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            if let Ok(directory) = std::fs::File::open(parent) {
                let _ = directory.sync_all();
            }
        }
        Ok(())
    })();
    if result.is_err() && created {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}
