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
    #[serde(default = "default_spell_names")]
    pub spell_names: String,
    #[serde(default = "default_text_shadows")]
    pub text_shadows: bool,
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
    #[serde(default = "default_await_xinput_hook")]
    pub await_xinput_hook: bool,
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

pub fn default_spell_names() -> String {
    "show".to_string()
}

pub const fn default_text_shadows() -> bool {
    true
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

pub const fn default_await_xinput_hook() -> bool {
    false
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
            spell_names: default_spell_names(),
            text_shadows: default_text_shadows(),
            debugging: default_debugging(),
            font_scale_multiplier: default_font_scale_multiplier(),
            icon_scale_multiplier: default_icon_scale_multiplier(),
            radius_multiplier: default_radius_multiplier(),
            modded_spells: default_modded_spells(),
            await_xinput_hook: default_await_xinput_hook(),
            min_radius: default_min_radius(),
            timing_offset: default_timing_offset(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum SpellNames {
    Show,
    Center,
    Hide
}

impl<S: AsRef<str>> From<S> for SpellNames {
    fn from(value: S) -> Self {
        match value.as_ref().to_lowercase().as_str() {
            "show" => Self::Show,
            "center" => Self::Center,
            "hide" => Self::Hide,
            _ => Self::Show,
        }
    }
}

lazy_static!(
    static ref SETTINGS_CACHE: Arc<RwLock<Settings>> = Arc::new(RwLock::new(Settings::default()));
);
impl Settings {
    pub fn open_toml() -> anyhow::Result<Self> {
        toml::from_str(&read_to_string(paths::settings())?).map_err(anyhow::Error::new)
    }

    pub fn read_or_default() -> Self {
        run_every!("Settings::read_or_default" every Duration::from_secs(1) => {
            let settings = Self::open_toml().unwrap_or_else(|err| {
                tracing::error!("Could not open settings TOML, using default settings instead: {err}");
                Settings::default()
            });
            *SETTINGS_CACHE.write().expect("Could not acquire settings cache") = settings.clone();
            return settings;
        });

        SETTINGS_CACHE.read().expect("Could not acquire settings cache").clone()
    }

    pub fn spell_names(&self) -> SpellNames {
        self.spell_names.as_str().into()
    }
}