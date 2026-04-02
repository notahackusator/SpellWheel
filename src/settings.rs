use std::fs::File;
use std::io::Read;
use crate::get_settings_path;

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
        let mut toml_src = String::new();
        File::open(get_settings_path()).ok()?.read_to_string(&mut toml_src).ok()?;
        toml::from_str(&toml_src).ok()
    }
}