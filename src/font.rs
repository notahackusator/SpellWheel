use imgui::FontGlyphRanges;
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