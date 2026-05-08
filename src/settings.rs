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
    #[serde(default = "default_using_controller")]
    pub using_controller: bool,
    #[serde(default = "default_controller_wheel_open_delay")]
    pub controller_wheel_open_delay: f32,
    #[serde(default = "default_font_scale_multiplier")]
    pub font_scale_multiplier: f32,
    #[serde(default = "default_icon_scale_multiplier")]
    pub icon_scale_multiplier: f32,
    #[serde(default = "default_radius_multiplier")]
    pub radius_multiplier: f32,
    #[serde(default = "default_min_radius")]
    pub min_radius: f32,
    #[serde(default = "default_modded_spells")]
    pub modded_spells: Vec<String>,
    #[serde(default = "default_debugging")]
    pub debugging: bool,
    #[serde(default = "default_timing_offset")]
    pub timing_offset: f32,
}

pub fn default_key() -> String {
    "TAB".to_string()
}

pub const fn default_using_controller() -> bool {
    false
}

pub const fn default_controller_wheel_open_delay() -> f32 {
    0.5
}

pub const fn default_debugging() -> bool {
    false
}

pub const fn default_font_scale_multiplier() -> f32 {
    1.0
}

pub const fn default_icon_scale_multiplier() -> f32 {
    0.15
}

pub const fn default_radius_multiplier() -> f32 {
    0.3
}

pub fn default_modded_spells() -> Vec<String> {
    Vec::new()
}

pub const fn default_min_radius() -> f32 {
    0.3
}

pub const fn default_timing_offset() -> f32 {
    0.0
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            key: default_key(),
            using_controller: default_using_controller(),
            controller_wheel_open_delay: default_controller_wheel_open_delay(),
            debugging: default_debugging(),
            font_scale_multiplier: default_font_scale_multiplier(),
            icon_scale_multiplier: default_icon_scale_multiplier(),
            radius_multiplier: default_radius_multiplier(),
            modded_spells: default_modded_spells(),
            min_radius: default_min_radius(),
            timing_offset: default_timing_offset(),
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