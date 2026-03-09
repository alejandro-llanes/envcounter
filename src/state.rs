use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::pool::Pool;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct AppState {
    pub pools: HashMap<String, Pool>,
}

/// Returns the path to the state file: `$XDG_STATE_HOME/envcounter/state.json`
/// (typically `~/.local/state/envcounter/state.json`).
pub fn state_file_path() -> Result<PathBuf, io::Error> {
    let base = directories::BaseDirs::new()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "cannot determine home directory"))?;

    let state_dir = base
        .state_dir()
        .unwrap_or_else(|| base.data_local_dir())
        .join("envcounter");

    Ok(state_dir.join("state.json"))
}

/// Persist state to disk using atomic write (write tmp, then rename).
pub fn save(state: &AppState) -> Result<(), io::Error> {
    let path = state_file_path()?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let tmp_path = path.with_extension("json.tmp");
    let data = serde_json::to_string_pretty(state)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    fs::write(&tmp_path, data)?;
    fs::rename(&tmp_path, &path)?;

    Ok(())
}

/// Load state from disk. Returns default empty state if file does not exist.
pub fn load() -> Result<AppState, io::Error> {
    let path = state_file_path()?;

    match fs::read_to_string(&path) {
        Ok(data) => {
            let state: AppState = serde_json::from_str(&data)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            Ok(state)
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(AppState::default()),
        Err(e) => Err(e),
    }
}
