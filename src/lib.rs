#![allow(unsafe_op_in_unsafe_fn)]

use std::fs;
use std::fs::File;
use std::path::PathBuf;
use std::ptr::NonNull;
use std::sync::OnceLock;
use std::time::Duration;
use eldenring::cs::{CSTaskGroupIndex, CSTaskImp, GameDataMan, Magic, MsbRepository, RendMan, SoloParamRepository};
use eldenring::fd4::FD4TaskData;
use eldenring::util::system::wait_for_system_init;
use fromsoftware_shared::{FromStatic, Program, SharedTaskImpExt};
use pmod::fmg::MsgRepository;
use tracing_subscriber::fmt;
use tracing_subscriber::fmt::layer;
use windows_sys::Win32::System::LibraryLoader::GetModuleFileNameW;

static GAME_PATH: OnceLock<PathBuf> = OnceLock::new();

#[unsafe(no_mangle)]
#[allow(nonstandard_style)]
/// # Safety
///
/// This is exposed this way such that windows LoadLibrary API can call it. Do not call this yourself.
pub unsafe extern "C" fn DllMain(hmodule: usize, reason: u32) -> bool {
    if reason != 1 {
        return true;
    }

    std::panic::set_hook(Box::new(move |info| {
        let msg = info.to_string();
        let _ = fs::write(get_log_path(hmodule), msg);
    }));

    fmt().with_writer(File::create(get_log_path(hmodule)).expect("Could not create log file"))
        .with_ansi(false)
        .init();
    tracing::info!("Log file created");
    GAME_PATH.set(get_game_path(hmodule)).unwrap();
    tracing::info!("Game path set");

    std::thread::spawn(main_thread);

    true
}

fn get_dll_path(hmodule: usize) -> PathBuf {
    let mut buf = vec![0; 260];
    unsafe {
        GetModuleFileNameW(
            hmodule as _,
            buf.as_mut_ptr(),
            buf.len() as u32,
        );
    }
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    PathBuf::from(String::from_utf16_lossy(&buf[..len]))
}

fn get_mods_folder_path(hmodule: usize) -> PathBuf {
    get_dll_path(hmodule).parent().unwrap()
        .to_path_buf()
}

fn get_log_path(hmodule: usize) -> PathBuf {
    get_mods_folder_path(hmodule)
        .join("spellwheel.log")
}

fn get_game_path(hmodule: usize) -> PathBuf {
    get_mods_folder_path(hmodule)
        .parent().unwrap()
        .to_path_buf()
}

fn main_thread() {
    tracing::info!("Awaiting system init");
    wait_for_system_init(&Program::current(), Duration::MAX)
        .expect("Could not await system init.");

    tracing::info!("Main function called");
    let tasks = unsafe { CSTaskImp::instance() }.unwrap();
    tasks.run_recurring(
        tick,
        CSTaskGroupIndex::GameMan
    );
}

fn tick(_fd4: &FD4TaskData) {
    let Some(game_data_man) = unsafe { GameDataMan::instance() }.ok() else {
        return;
    };

    let mut equipped_spell_ids = Vec::with_capacity(14);
    for entry in &game_data_man.main_player_game_data.equipment.equip_magic_data.entries {
        equipped_spell_ids.push(entry.param_id as u32);
    }

    let spell_names = equipped_spell_ids.into_iter()
        .map(get_spell_name)
        .collect::<Vec<_>>();

    let Some(renderer) = unsafe { RendMan::instance() }.ok() else {
        return;
    };

    tracing::info!("{spell_names:?}");
}

fn get_spell_name(spell_id: u32) -> Option<String> {
    const SPELL_NAME: u32 = 10;
    unsafe {
        read_utf16_string(MsgRepository::get_msg(
            0, SPELL_NAME, spell_id
        ))
    }
}

unsafe fn read_utf16_string(ptr: Option<NonNull<u16>>) -> Option<String> {
    ptr.map(|ptr| {
        let mut len = 0;
        let mut p = ptr.as_ptr();

        while *p != 0 {
            len += 1;
            p = p.add(1);
        }

        let slice = std::slice::from_raw_parts(ptr.as_ptr(), len);

        String::from_utf16_lossy(slice)
    })
}