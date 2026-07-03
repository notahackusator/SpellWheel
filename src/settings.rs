use crate::debugging::run_every;
use lazy_static::lazy_static;
use std::fs::read_to_string;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use crate::paths;
use serde::Deserialize;

macro_rules! settings {
    ($($name:ident: $t:ty),*$(,)?) => {
        #[derive(Clone)]
        pub struct Settings {
            $(
                pub $name: $t
            ),*
        }

        impl<'de> serde::Deserialize<'de> for Settings {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                use serde::de::{self, MapAccess, Visitor};
                use std::fmt;

                struct SettingsVisitor;

                impl<'de> Visitor<'de> for SettingsVisitor {
                    type Value = Settings;

                    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                        write!(f, "a settings map")
                    }

                    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Settings, A::Error> {
                        let mut this = Settings::default();

                        while let Some(key) = map.next_key::<String>()? {
                            match key.as_str() {
                                $(stringify!($name) => {
                                    this.$name = map.next_value()?;
                                })*
                                _ => { map.next_value::<serde::de::IgnoredAny>()?; }
                            }
                        }

                        Ok(this)
                    }
                }

                deserializer.deserialize_map(SettingsVisitor)
            }
        }

        impl Default for Settings {
            fn default() -> Self {
                Self {
                    $(
                        $name: $name()
                    ),*
                }
            }
        }
    };
}

settings!(
    spells_key: String,
    quick_items_key: String,
    spells_button: String,
    quick_items_button: String,
    using_controller: bool,
    controller_wheel_open_delay: f32,
    switch_instantly: bool,
    item_names: String,
    text_shadows: bool,
    font_scale_multiplier: f32,
    icon_scale_multiplier: f32,
    radius_multiplier: f32,
    min_radius: f32,
    modded_spells: Vec<String>,
    await_xinput_hook: bool,
    debugging: bool,
    timing_offset: f32,
);

pub fn spells_key() -> String {
    "TAB".to_string()
}

pub fn quick_items_key() -> String {
    "CAPITAL".to_string()
}

pub fn spells_button() -> String {
    "UP".to_string()
}

pub fn quick_items_button() -> String {
    "DOWN".to_string()
}

pub const fn using_controller() -> bool {
    false
}

pub const fn controller_wheel_open_delay() -> f32 {
    0.5
}

pub const fn switch_instantly() -> bool {
    true
}

pub fn item_names() -> String {
    "show".to_string()
}

pub const fn text_shadows() -> bool {
    true
}

pub const fn debugging() -> bool {
    false
}

pub const fn font_scale_multiplier() -> f32 {
    1.0
}

pub const fn icon_scale_multiplier() -> f32 {
    0.15
}

pub const fn radius_multiplier() -> f32 {
    0.3
}

pub fn modded_spells() -> Vec<String> {
    Vec::new()
}

pub const fn await_xinput_hook() -> bool {
    false
}

pub const fn min_radius() -> f32 {
    0.3
}

pub const fn timing_offset() -> f32 {
    0.0
}

#[derive(Clone, Copy, Debug)]
pub enum ItemNames {
    Show,
    Center,
    Hide
}

impl<S: AsRef<str>> From<S> for ItemNames {
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

    pub fn item_names(&self) -> ItemNames {
        self.item_names.as_str().into()
    }
}