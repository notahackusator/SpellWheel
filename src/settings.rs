use crate::get_settings_path;
use std::fs::read_to_string;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use lazy_static::lazy_static;
use crate::debugging::run_every;

#[derive(serde::Deserialize, Clone)]
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

lazy_static!(
    static ref SETTINGS_CACHE: Arc<RwLock<Settings>> = Arc::new(RwLock::new(Settings::default()));
);
impl Settings {
    pub fn open_toml() -> Option<Self> {
        toml::from_str(&read_to_string(get_settings_path()).ok()?).ok()
    }

    pub fn read_or_default() -> Self {
        run_every!("Settings::read_or_default" every Duration::from_secs(1) => {
            let settings = Self::open_toml().unwrap_or(Settings::default());
            *SETTINGS_CACHE.write().expect("Could not acquire settings cache") = settings.clone();
            return settings;
        });

        SETTINGS_CACHE.read().expect("Could not acquire settings cache").clone()
    }
}