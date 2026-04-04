use crate::hmodule;
use cached::proc_macro::cached;
use std::path::PathBuf;
use windows_sys::Win32::System::LibraryLoader::GetModuleFileNameW;

#[cached]
pub fn dll() -> PathBuf {
    let mut buf = vec![0; 260];
    unsafe {
        GetModuleFileNameW(
            hmodule() as _,
            buf.as_mut_ptr(),
            buf.len() as u32,
        );
    }
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    PathBuf::from(String::from_utf16_lossy(&buf[..len]))
}

#[cached]
pub fn mods() -> PathBuf {
    dll().parent().unwrap()
        .to_path_buf()
}
#[cached]
pub fn spellwheel() -> PathBuf {
    mods()
        .join("spellwheel")
}

#[cached]
pub fn spell_icons() -> PathBuf {
    spellwheel()
        .join("icons")
}

#[cached]
pub fn log() -> PathBuf {
    spellwheel()
        .join("spellwheel.log")
}

#[cached]
pub fn font() -> PathBuf {
    spellwheel()
        .join("font.ttf")
}

#[cached]
pub fn settings() -> PathBuf {
    mods()
        .join("spellwheel.toml")
}