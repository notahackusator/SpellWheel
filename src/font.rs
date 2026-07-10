use imgui::FontGlyphRanges;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::mem;
use std::num::NonZeroUsize;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FontId {
    ptr: Option<NonZeroUsize>,
}

impl FontId {
    pub fn none() -> FontId {
        Self {
            ptr: None
        }
    }
}

impl From<imgui::FontId> for FontId {
    fn from(value: imgui::FontId) -> Self {
        unsafe {
            mem::transmute(value)
        }
    }
}

impl From<FontId> for imgui::FontId {
    fn from(value: FontId) -> Self {
        unsafe {
            mem::transmute(value)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FontChars(u128);

impl From<FontGlyphRanges> for FontChars {
    fn from(value: FontGlyphRanges) -> Self {
        unsafe {
            mem::transmute(value)
        }
    }
}

impl From<FontChars> for FontGlyphRanges {
    fn from(value: FontChars) -> Self {
        unsafe {
            mem::transmute(value)
        }
    }
}

pub const FONT_LATIN_BYTES: &'static [u8] = include_bytes!("../assets/font_latin.ttf");

lazy_static!(
    // Don't ask me why the FontGlyphRanges functions aren't const
    pub static ref OTHER_FONTS: HashMap<&'static str, FontChars> = HashMap::from_iter(
        [
            ("cyrillic.ttf", FontGlyphRanges::cyrillic().into()),
            ("japanese.ttf", FontGlyphRanges::japanese().into()),
            ("korean.ttf", FontGlyphRanges::korean().into()),
            ("simplified_chinese.ttf", FontGlyphRanges::chinese_simplified_common().into()),
            ("traditional_chinese.ttf", FontGlyphRanges::chinese_full().into()),
            ("thai.ttf", FontGlyphRanges::thai().into()),
        ]
    );
);

pub const DEFAULT_FONT_HEIGHT: f32 = 54.0;

macro_rules! create_font_sources {
    ($font_bytes:ident, $font_data:ident; then: $($t:tt)*) => {
        {
            let mut $font_data: Vec<imgui::FontSource> = vec![
                imgui::FontSource::TtfData {
                    data: crate::font::FONT_LATIN_BYTES,
                    size_pixels: crate::font::DEFAULT_FONT_HEIGHT,
                    config: Some(imgui::FontConfig {
                        glyph_ranges: imgui::FontGlyphRanges::default(),
                        ..imgui::FontConfig::default()
                    })
                }
            ];

            let mut $font_bytes = vec![];

            let mut fonts: Vec<(&'static str, imgui::FontGlyphRanges)> = crate::font::OTHER_FONTS.iter()
                .filter_map(|(font_name, font_ranges)| {
                    let path = crate::paths::spellwheel().join(font_name);
                    if !path.exists() {
                        tracing::info!("Skipping font {font_name}");
                        return None;
                    }
                    Some((*font_name, (*font_ranges).into()))
                })
                .collect();

            for (i, (font_name, _)) in fonts.iter().enumerate() {
                let path = crate::paths::spellwheel().join(font_name);
                tracing::info!("Adding font {font_name}");
                let bytes = match std::fs::read(path) {
                    Ok(bytes) => bytes,
                    Err(err) => {
                        tracing::error!("Couldn't load font {font_name}: {err}");
                        continue;
                    }
                };
                $font_bytes.push(bytes);
            }

            for (i, (_, font_ranges)) in fonts.iter().enumerate() {
                $font_data.push(
                    imgui::FontSource::TtfData {
                        data: &$font_bytes[i],
                        size_pixels: crate::font::DEFAULT_FONT_HEIGHT,
                        config: Some(imgui::FontConfig {
                            glyph_ranges: font_ranges.clone(),
                            ..imgui::FontConfig::default()
                        })
                    }
                );
            }

            $($t)*
        }
    };
}

pub(crate) use create_font_sources;