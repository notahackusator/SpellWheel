use crate::get_settings_path;
use std::fs::read_to_string;

#[derive(serde::Deserialize)]
pub struct Settings {
    pub key: String,
    pub debugging: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            key: "TAB".to_string(),
            debugging: false,
        }
    }
}

impl Settings {
    pub fn open_toml() -> Option<Self> {
        toml::from_str(&read_to_string(get_settings_path()).ok()?).ok()
    }
}