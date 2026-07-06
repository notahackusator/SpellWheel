use crate::hmodule;
use cached::proc_macro::cached;
use std::path::PathBuf;
use std::ptr::null_mut;
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
pub fn dll_folder() -> PathBuf {
    dll().parent().unwrap()
        .to_path_buf()
}

#[cached]
pub fn game() -> PathBuf {
    let mut buf = vec![0u16; 260];
    unsafe {
        GetModuleFileNameW(
            null_mut(),
            buf.as_mut_ptr(),
            buf.len() as u32,
        )
    };
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    PathBuf::from(String::from_utf16_lossy(&buf[..len]))
}

#[cached]
pub fn game_directory() -> PathBuf {
    game().parent().unwrap()
        .to_path_buf()
}

#[cached]
pub fn spellwheel() -> PathBuf {
    dll_folder()
        .join("spellwheel")
}

#[cached]
pub fn log() -> PathBuf {
    spellwheel()
        .join("spellwheel.log")
}

#[cached]
pub fn settings() -> PathBuf {
    dll_folder()
        .join("spellwheel.toml")
}