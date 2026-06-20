#![allow(unsafe_op_in_unsafe_fn)]

mod rendering;
mod debugging;
mod keyboard;
mod icons;
mod settings;
pub mod spells;
pub mod paths;
pub mod gamepad;
pub mod xinput_hook;
pub mod await_seamless;
pub mod display_spell;
pub mod mouse;
pub mod hwindow;
pub mod dynamic_icons;
pub mod util;

use std::fs::File;
use std::mem;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use eldenring::cs::{CSFeManHudState, CSFeManImp, CSMenuManImp, CSTaskGroupIndex, CSTaskImp, GameDataMan, Magic, SoloParam, SoloParamRepository, WorldChrManDbg};
use eldenring::fd4::FD4TaskData;
use eldenring::util::system::wait_for_system_init;
use fromsoftware_shared::{FromStatic, Program, SharedTaskImpExt};
use lazy_static::lazy_static;
use tracing_subscriber::fmt;
use crate::await_seamless::{await_seamless, is_seamless_coop_active};
use crate::debugging::{add_to_screen_debug, commit_screen_debug, is_debugging, run_every, run_once};
use crate::gamepad::GamepadState;
use crate::keyboard::is_player_selecting_spell;
use crate::rendering::{try_init_rendering, SpellWheelData};
use crate::settings::Settings;
use crate::spells::Spell;
use crate::xinput_hook::{install_xinput_hook, set_suppress_camera};

static HMODULE: OnceLock<usize> = OnceLock::new();

pub fn hmodule() -> usize {
    *HMODULE.get().expect("Could not get HMODULE")
}

static HWND: OnceLock<usize> = OnceLock::new();

pub fn hwnd() -> windows::Win32::Foundation::HWND {
    unsafe {
        mem::transmute(*HWND.get().expect("Could not get HWND"))
    }
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

    std::thread::spawn(move || init(hmodule));

    true
}

fn init(hmodule: usize) {
    HMODULE.set(hmodule).expect("Could not set HMODULE");
    // Fix for Seamless crash
    std::thread::sleep(Duration::from_secs_f32(Settings::read_or_default().timing_offset));
    if is_seamless_coop_active() {
        await_seamless();
    }

    fmt().with_writer(File::create(paths::log()).expect("Could not create log file"))
        .with_ansi(false)
        .init();
    tracing::info!("Log file created");
    std::panic::set_hook(Box::new(move |info| {
        let msg = info.to_string();
        tracing::error!("Encountered an error:\n{msg}");
    }));

    guard!(
        start();
    );
}

lazy_static!(
    pub static ref PROGRAM_START: Instant = Instant::now();
);

lazy_static!(
    static ref GAMEPAD_STATE: OnceLock<Arc<Mutex<GamepadState>>> = OnceLock::new();
);
fn update_gamepad_state() {
    match GAMEPAD_STATE.get() {
        Some(gamepad_state) => gamepad_state.lock().unwrap().update(),
        None => tracing::error!("update_gamepad_data called before GAMEPAD_STATE was initialized"),
    }
}

pub fn gamepad_state() -> GamepadState {
    match GAMEPAD_STATE.get() {
        Some(gamepad_state) => gamepad_state.lock().unwrap().clone(),
        None => {
            tracing::error!("gamepad_data called before GAMEPAD_STATE was initialized");
            GamepadState::new()
        }
    }
}

#[macro_export]
macro_rules! guard {
    ($($t:tt)*) => {
        let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            $($t)*
        }));
        if let Err(err) = out {
            tracing::error!("Encountered an error:\n{}", panic_message::panic_message(&err));
        }
    };
}

fn start() {
    tracing::info!("DLL Path: {:?}", paths::dll());
    tracing::info!("Awaiting system init");
    wait_for_system_init(&Program::current(), Duration::MAX)
        .expect("Could not await system init.");

    if !Settings::read_or_default().await_xinput_hook {
        install_xinput_hook();
    }

    tracing::info!("Init complete");
    let tasks = unsafe { CSTaskImp::instance() }.expect("Could not get CSTaskImp");
    tracing::info!("Creating gamepad state");
    let gamepad_state = Arc::new(Mutex::new(GamepadState::new()));
    if GAMEPAD_STATE.set(gamepad_state).is_err() {
        tracing::error!("GAMEPAD_STATE should not be set before start");
    }
    tracing::info!("Running task");
    tasks.run_recurring(
        tick,
        CSTaskGroupIndex::MenuMan
    );
}

lazy_static!(
    static ref SELECTED_SPELL_INDEX: AtomicI32 = AtomicI32::new(-1);
);

pub fn set_selected_spell_index(idx: i32) {
    SELECTED_SPELL_INDEX.store(idx, Ordering::Relaxed);
}

// One may ask why I'm creating atomic structs in lazy statics.
// Thour't a cunning fella, I assure ye! Thy suspicions are not misplaced.
// Though alas, thou art mistaken.
// For ye see, were I not to do this, one would find that:
//
// IN_MENUS.store(false, Ordering::Release);
// assert!(!in_menus());
//
// fails.
lazy_static!(
    static ref IN_MENUS: AtomicBool = AtomicBool::new(true);
);

pub fn in_menus() -> bool {
    IN_MENUS.load(Ordering::Acquire)
}

pub fn set_in_menus(world_chr_man_dbg: &WorldChrManDbg, fe_man: &CSFeManImp) {
    let not_in_game = world_chr_man_dbg.player_session_holder.is_none();
    let paused = !matches!(fe_man.hud_state, CSFeManHudState::Default);
    if is_debugging() {
        add_to_screen_debug(format!("pre-game menu: {not_in_game}"));
        add_to_screen_debug(format!("in-game menu: {paused}"));
    }
    IN_MENUS.store(
        not_in_game || paused,
        Ordering::Release
    );
}

static mut WAS_PLAYER_SELECTING_SPELL: bool = false;
fn tick(_fd4: &FD4TaskData) {
    guard!(
        run_once!("entered tick function" => {
            tracing::info!("Entered tick function");
        });
        if is_debugging() {
            run_every!("D tick" every Duration::from_secs(1) => {
                tracing::info!("In tick function");
            });
        }
        let Some(world_chr_man_dbg) = unsafe { WorldChrManDbg::instance() }.ok() else {
            return;
        };

        let Some(fe_man) = unsafe { CSFeManImp::instance() }.ok() else {
            return;
        };
        set_in_menus(world_chr_man_dbg, fe_man);

        let Some(game_data_man) = unsafe { GameDataMan::instance_mut() }.ok() else {
            return;
        };

        let Some(param_repo) = unsafe { SoloParamRepository::instance_mut() }.ok() else {
            return;
        };

        let Some(menu_man) = unsafe { CSMenuManImp::instance_mut() }.ok() else {
            return;
        };

        if param_repo.solo_param_holders[Magic::INDEX as usize].get_res_cap(0).is_none() {
            return;
        }
        run_once!("passed all checks" => {
            tracing::info!("Passed all checks");
        });
        if is_debugging() {
            run_every!("D passed all checks" every Duration::from_secs(1) => {
                tracing::info!("Passed all checks");
            });
        }
        if Settings::read_or_default().await_xinput_hook {
            install_xinput_hook();
        }
        update_gamepad_state();
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
            if let Some(spell) = Spell::try_new(param_repo, index as i32, id) {
                equipped_spells.push(spell);
            }
        }
        if is_debugging() {
            add_to_screen_debug(format!("Equipped spells: {equipped_spells:?}"));
        }

        if equipped_spells.is_empty() {
            return;
        }

        SpellWheelData::mutate(|data| {
            data.spells = equipped_spells;
        });
        unsafe {
            let is_player_selecting_spell = is_player_selecting_spell();
            if WAS_PLAYER_SELECTING_SPELL != is_player_selecting_spell {
                WAS_PLAYER_SELECTING_SPELL = is_player_selecting_spell;
                menu_man.disable_mouse_cursor = !is_player_selecting_spell;
                set_suppress_camera(is_player_selecting_spell);
            }
            SpellWheelData::mutate(|data| {
                data.do_render = is_player_selecting_spell;
            });
        }

        commit_screen_debug();
    );
}