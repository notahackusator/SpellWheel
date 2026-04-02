#![allow(unsafe_op_in_unsafe_fn)]

mod rendering;
mod debugging;
mod keyboard;
mod icons;
mod settings;

use std::fs::File;
use std::panic::catch_unwind;
use std::path::PathBuf;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::OnceLock;
use std::time::Duration;
use eldenring::cs::{CSMenuManImp, CSTaskGroupIndex, CSTaskImp, GameDataMan, Magic, SoloParam, SoloParamRepository};
use eldenring::fd4::FD4TaskData;
use eldenring::util::system::wait_for_system_init;
use fromsoftware_shared::{FromStatic, Program, SharedTaskImpExt};
use lazy_static::lazy_static;
use pmod::fmg::MsgRepository;
use tracing_subscriber::fmt;
use windows_sys::Win32::System::LibraryLoader::GetModuleFileNameW;
use crate::debugging::{is_debugging, run_every};
use crate::keyboard::is_player_selecting_spell;
use crate::rendering::{try_init_rendering, SpellWheelData};

static HMODULE: OnceLock<usize> = OnceLock::new();

pub fn hmodule() -> usize {
    *HMODULE.get().expect("Could not get HMODULE")
}

#[unsafe(no_mangle)]
#[allow(nonstandard_style)]
/// # Safety
///
/// This is exposed this way such that windows LoadLibrary API can call it. Do not call this yourself.
pub unsafe extern "C" fn DllMain(hmodule: usize, reason: u32) -> bool {
    if reason != 1 {
        return true;
    }

    HMODULE.set(hmodule).expect("Could not set HMODULE");

    fmt().with_writer(File::create(get_log_path()).expect("Could not create log file"))
        .with_ansi(false)
        .init();
    tracing::info!("Log file created");
    std::panic::set_hook(Box::new(move |info| {
        let msg = info.to_string();
        tracing::error!("Encountered an error:\n{msg}");
    }));
    std::thread::spawn(|| {
        let code = catch_unwind(|| {
            start();
        });
        if code.is_err() {
            tracing::error!("Encountered an error:\n{}", panic_message::panic_message(&code.unwrap_err()));
        }
    });

    true
}

fn start() {
    tracing::info!("Awaiting system init");
    wait_for_system_init(&Program::current(), Duration::MAX)
        .expect("Could not await system init.");

    main_thread();
}

fn main_thread() {
    tracing::info!("Main function called");
    let tasks = unsafe { CSTaskImp::instance() }.unwrap();
    tasks.run_recurring(
        tick,
        CSTaskGroupIndex::GameMan
    );
}

lazy_static!(
    static ref SELECTED_SPELL_INDEX: AtomicI32 = AtomicI32::new(-1);
);

pub fn set_selected_spell_index(idx: i32) {
    SELECTED_SPELL_INDEX.store(idx, Ordering::Relaxed);
}

#[derive(Clone, Debug, PartialEq)]
struct Spell {
    index: usize,
    id: u32,
    name: String,
}

impl Spell {
    fn try_new(index: usize, id: u32, name: Option<String>) -> Option<Self> {
        name.map(|name| Self { index, id, name })
    }
}

fn tick(_fd4: &FD4TaskData) {
    if is_debugging() {
        run_every!("tick info" every Duration::from_secs(1) => {
            tracing::info!("In tick function");
        });
    }
    let Some(game_data_man) = unsafe { GameDataMan::instance() }.ok() else {
        return;
    };

    let Some(param_repo) = unsafe { SoloParamRepository::instance() }.ok() else {
        return;
    };

    let Some(menu_man) = unsafe { CSMenuManImp::instance() }.ok() else {
        return;
    };

    if param_repo.solo_param_holders[Magic::INDEX as usize].get_res_cap(0).is_none() {
        return;
    }
    try_init_rendering();

    let selected_spell_index = SELECTED_SPELL_INDEX.load(Ordering::Relaxed);
    if selected_spell_index != -1 {
        game_data_man.main_player_game_data.equipment.equip_magic_data.selected_slot = selected_spell_index;
        SELECTED_SPELL_INDEX.store(-1, Ordering::Relaxed);
    }

    let mut equipped_spells = Vec::with_capacity(14);
    let data = &game_data_man.main_player_game_data.equipment.equip_magic_data;
    for (index, spell) in data.entries.iter().enumerate() {
        let id = spell.param_id as u32;
        if let Some(spell) = Spell::try_new(index, id, get_spell_name(id)) {
            equipped_spells.push(spell);
        }
    }
    if is_debugging() {
        run_every!("spell info" every Duration::from_secs(1) => {
            tracing::info!("Equipped spells: {equipped_spells:?}");
        });
    }

    if equipped_spells.is_empty() {
        return;
    }

    SpellWheelData::mutate(|data| {
        data.spells = equipped_spells;
        data.do_render = is_player_selecting_spell();
    });
    menu_man.disable_mouse_cursor = !is_player_selecting_spell();
}

fn get_dll_path() -> PathBuf {
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

fn get_mods_folder_path() -> PathBuf {
    get_dll_path().parent().unwrap()
        .to_path_buf()
}

fn get_log_path() -> PathBuf {
    get_mods_folder_path()
        .join("spellwheel")
        .join("spellwheel.log")
}

fn get_spell_icons_path() -> PathBuf {
    get_mods_folder_path()
        .join("spellwheel")
        .join("icons")
}

fn get_font_path() -> PathBuf {
    get_mods_folder_path()
        .join("spellwheel")
        .join("font.ttf")
}

fn get_settings_path() -> PathBuf {
    get_mods_folder_path()
        .join("spellwheel.toml")
}

fn get_spell_name(spell_id: u32) -> Option<String> {
    const BASE_GAME_SPELL_NAME: u32 = 10;
    const DLC_SPELL_NAME: u32 = 319;

    unsafe {
        read_utf16_string(MsgRepository::get_msg(
            0, BASE_GAME_SPELL_NAME, spell_id
        )).or(read_utf16_string(MsgRepository::get_msg(
            0, DLC_SPELL_NAME, spell_id
        )))
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