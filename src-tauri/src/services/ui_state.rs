use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum UiStateError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON parse error: {0}")]
    JsonParse(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowGeometry {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug)]
pub struct UiStateService {
    state: serde_json::Value,
    config_dir: PathBuf,
}

impl UiStateService {
    pub fn load(config_dir: &Path) -> Result<Self, UiStateError> {
        let path = config_dir.join("ui_state.json");
        let state = if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            serde_json::from_str(&content)?
        } else {
            serde_json::json!({})
        };

        Ok(Self {
            state,
            config_dir: config_dir.to_path_buf(),
        })
    }

    fn save(&self) -> Result<(), UiStateError> {
        let path = self.config_dir.join("ui_state.json");
        let content = serde_json::to_string_pretty(&self.state)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn get(&self, key: &str) -> Option<serde_json::Value> {
        let keys: Vec<&str> = key.split('.').collect();
        let mut current = &self.state;

        for k in &keys {
            current = current.get(k)?;
        }

        Some(current.clone())
    }

    pub fn set(&mut self, key: &str, value: serde_json::Value) -> Result<(), UiStateError> {
        let keys: Vec<&str> = key.split('.').collect();
        let mut current = &mut self.state;

        for k in &keys[..keys.len() - 1] {
            if !current.get(k).is_some_and(|v| v.is_object()) {
                current[k] = serde_json::json!({});
            }
            current = current.get_mut(k).unwrap();
        }

        current[keys[keys.len() - 1]] = value;
        self.save()
    }

    pub fn get_geometry(&self, window_id: &str) -> Option<WindowGeometry> {
        let raw = self.get(&format!("{window_id}.geometry"))?;
        serde_json::from_value(raw).ok()
    }

    pub fn set_geometry(
        &mut self,
        window_id: &str,
        geom: WindowGeometry,
    ) -> Result<(), UiStateError> {
        let value = serde_json::to_value(&geom).map_err(UiStateError::JsonParse)?;
        self.set(&format!("{window_id}.geometry"), value)
    }
}
