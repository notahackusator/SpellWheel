pub mod display_settings;
pub mod visual_toml_parser;

use crate::debugging::{run_every, run_once};
use lazy_static::lazy_static;
use std::fs::{read_to_string, write};
use std::sync::{Arc, RwLock, RwLockReadGuard};
use std::time::Duration;
use crate::paths;

macro_rules! settings {
    ($($name:ident: $t:ty),*$(,)?) => {
        pub const KEYS: &[&str] = &[
            $(stringify!($name),)*
        ];

        #[derive(Clone)]
        pub struct Settings {
            $(
                pub $name: $t
            ),*
        }

        impl<'de> serde::Deserialize<'de> for Settings {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                use serde::de::{MapAccess, Visitor};
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
    style: String,
    highlight_time: f32,
    highlight_expansion: f32,
    controller_wheel_open_delay: f32,
    center_on_close: bool,
    switch_instantly: bool,
    item_names: String,
    text_shadows: bool,
    font_scale_multiplier: f32,
    icon_scale_multiplier: f32,
    radius_multiplier: f32,
    min_radius: f32,
    mods: Vec<String>,
    modded_spells: Vec<String>,
    await_xinput_hook: bool,
    debugging: bool,
    timing_offset: f32,
    force_rgba: bool,
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

pub const fn center_on_close() -> bool {
    true
}

pub fn style() -> String {
    "pretty".to_string()
}

pub const fn highlight_time() -> f32 {
    0.15
}

pub const fn highlight_expansion() -> f32 {
    0.15
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

pub fn mods() -> Vec<String> {
    Vec::new()
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

pub const fn force_rgba() -> bool {
    false
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Style {
    Simple,
    Pretty
}

impl<S: AsRef<str>> From<S> for Style {
    fn from(value: S) -> Self {
        match value.as_ref().to_lowercase().as_str() {
            "simple" => Self::Simple,
            "pretty" => Self::Pretty,
            _ => Self::Pretty,
        }
    }
}

pub struct SettingsContext {
    src: Arc<str>,
    settings: Settings,
}

impl Default for SettingsContext {
    fn default() -> Self {
        Self {
            src: "".into(),
            settings: Default::default(),
        }
    }
}

pub enum SCRes {
    Ok(SettingsContext),
    ParseErr(SettingsContext, anyhow::Error),
    Err(anyhow::Error)
}

impl SCRes {
    pub fn into_sc(self) -> Option<SettingsContext> {
        match self {
            SCRes::Ok(sc) => Some(sc),
            SCRes::ParseErr(sc, _) => Some(sc),
            SCRes::Err(_) => None,
        }
    }

    pub fn into_sc_or(self, sc: SettingsContext) -> SettingsContext {
        self.into_sc().unwrap_or(sc)
    }

    pub fn into_sc_or_else<F: FnOnce() -> SettingsContext>(self, f: F) -> SettingsContext {
        self.into_sc().unwrap_or_else(f)
    }

    pub fn get_err(&self) -> Option<&anyhow::Error> {
        match self {
            SCRes::Ok(_) => None,
            SCRes::ParseErr(_, err) => Some(err),
            SCRes::Err(err) => Some(err),
        }
    }
}

impl SettingsContext {
    pub fn open_toml() -> SCRes {
        let settings_path = paths::settings();
        let src = match read_to_string(&settings_path) {
            Ok(src) => src,
            Err(err) => {
                tracing::error!("Tried to look for settings in {settings_path:?}, but encountered an error");
                return SCRes::Err(err.into());
            }
        };

        match toml::from_str(&src) {
            Ok(settings) => {
                SCRes::Ok(Self {
                    src: src.into(),
                    settings,
                })
            }
            Err(err) => {
                SCRes::ParseErr(
                    Self {
                        src: src.into(),
                        settings: Settings::default(),
                    },
                    err.into()
                )
            }
        }
    }

    pub fn read_or_default() -> RwLockReadGuard<'static, Self> {
        run_every!("SettingsContext::read_or_default" every Duration::from_secs(1) => {
            Self::load();
        });

        SETTINGS_CACHE.read().expect("Could not acquire settings cache")
    }
    
    fn load() {
        let sc = SettingsContext::open_toml();
        if let Some(err) = sc.get_err() {
            tracing::error!("Could not open settings TOML, using default settings instead: {err}");
        }
        let ctx = sc.into_sc_or_else(SettingsContext::default);
        *SETTINGS_CACHE.write().expect("Could not acquire settings cache") = ctx;
    }

    pub fn save(text: &str) {
        let path = paths::settings();
        if let Err(err) = write(path, text) {
            tracing::error!("Error saving settings: {err}");
        }
        Self::load();
    }
}

lazy_static!(
    static ref SETTINGS_CACHE: Arc<RwLock<SettingsContext>> = Arc::new(RwLock::new(SettingsContext::default()));
);
impl Settings {
    pub fn read_or_default() -> Self {
        run_every!("Settings::read_or_default" every Duration::from_secs(1) => {
            let sc = SettingsContext::open_toml();
            if let Some(err) = sc.get_err() {
                tracing::error!("Could not open settings TOML, using default settings instead: {err}");
            }
            let ctx = sc.into_sc_or_else(SettingsContext::default);
            let out = ctx.settings.clone();
            *SETTINGS_CACHE.write().expect("Could not acquire settings cache") = ctx;
            return out;
        });

        SETTINGS_CACHE.read().expect("Could not acquire settings cache").settings.clone()
    }
    
    pub fn mods(&self) -> &[String] {
        match (self.mods.is_empty(), self.modded_spells.is_empty()) {
            (true, true) => &[],
            (true, false) => &self.modded_spells,
            (false, true) => &self.mods,
            (false, false) => {
                run_once!("Settings::mods" => {
                    tracing::warn!(
                        "Both 'mods' and 'modded_spells' aren't empty. Defaulting to 'mods'.\n\
                        'modded_spells' is old and 'mods' should be used instead"
                    );
                });
                &self.mods
            }
        }
    }

    pub fn item_names(&self) -> ItemNames {
        self.item_names.as_str().into()
    }

    pub fn style(&self) -> Style {
        self.style.as_str().into()
    }
    
    // Returns the highlight time t normalized. Takes into account self.highlight_time = 0.
    pub fn normalized_highlight_time(&self, t: f32) -> f32 {
        if self.highlight_time == 0.0 {
            if t > 0.0 {
                1.0
            } else {
                0.0
            }
        } else {
            t / self.highlight_time
        }
    }
}