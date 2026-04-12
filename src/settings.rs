use crate::debugging::run_every;
use lazy_static::lazy_static;
use std::fs::read_to_string;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use crate::paths;

#[derive(serde::Deserialize, Clone)]
pub struct Settings {
    #[serde(default = "default_key")]
    pub key: String,
    #[serde(default = "default_debugging")]
    pub debugging: bool,
    #[serde(default = "default_font_scale_multiplier")]
    pub font_scale_multiplier: f32,
    #[serde(default = "default_icon_scale_multiplier")]
    pub icon_scale_multiplier: f32,
    #[serde(default = "default_radius_multiplier")]
    pub radius_multiplier: f32,
    #[serde(default = "default_timing_offset")]
    pub timing_offset: f32,
}

pub fn default_key() -> String {
    "TAB".to_string()
}

pub fn default_debugging() -> bool {
    false
}

pub fn default_font_scale_multiplier() -> f32 {
    1.0
}

pub fn default_icon_scale_multiplier() -> f32 {
    0.15
}

pub fn default_radius_multiplier() -> f32 {
    0.25
}

pub fn default_timing_offset() -> f32 {
    0.0
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            key: "TAB".to_string(),
            debugging: false,
            font_scale_multiplier: 1.0,
            icon_scale_multiplier: 0.15,
            radius_multiplier: 0.25,
            timing_offset: 0.0,
        }
    }
}

lazy_static!(
    static ref SETTINGS_CACHE: Arc<RwLock<Settings>> = Arc::new(RwLock::new(Settings::default()));
);
impl Settings {
    pub fn open_toml() -> Option<Self> {
        toml::from_str(&read_to_string(paths::settings()).ok()?).ok()
    }

    pub fn read_or_default() -> Self {
        run_every!("Settings::read_or_default" every Duration::from_secs(1) => {
            let settings = Self::open_toml().unwrap_or_else(|| {
                tracing::error!("Could not open settings TOML, using default settings instead");
                Settings::default()
            });
            *SETTINGS_CACHE.write().expect("Could not acquire settings cache") = settings.clone();
            return settings;
        });

        SETTINGS_CACHE.read().expect("Could not acquire settings cache").clone()
    }
}