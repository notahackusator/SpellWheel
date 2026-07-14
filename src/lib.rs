#![allow(unsafe_op_in_unsafe_fn)]

mod rendering;
mod debugging;
mod keyboard;
mod icons;
mod settings;
pub mod items;
pub mod paths;
pub mod gamepad;
pub mod xinput_hook;
pub mod await_seamless;
pub mod display_item;
pub mod mouse;
pub mod hwindow;
pub mod dynamic_icons;
pub mod util;
pub mod io;
pub mod font;
pub mod glyphs;

use std::fs::File;
use std::mem;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use eldenring::cs::{CSFeManHudState, CSFeManImp, CSMenuManImp, CSTaskGroupIndex, CSTaskImp, GameDataMan, Magic, SoloParam, SoloParamRepository, WorldChrManDbg};
use eldenring::fd4::FD4TaskData;
use eldenring::util::system::wait_for_system_init;
use fromsoftware_shared::{FromStatic, Program, SharedTaskImpExt};
use hudhook::windows::Win32::System::SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH};
use lazy_static::lazy_static;
use tracing_subscriber::fmt;
use crate::await_seamless::{await_seamless, is_seamless_coop_active};
use crate::debugging::{add_to_screen_debug, commit_screen_debug, is_debugging, run_every, run_once};
use crate::gamepad::{set_gamepad_state, update_gamepad_state, GamepadState};
use crate::icons::icon_manager::IconManager;
use io::selected_wheel_type;
use crate::glyphs::font_manager::FontManager;
use crate::rendering::{try_init_rendering, remove_hudhook, ItemWheelData, WheelType};
use crate::settings::Settings;
use crate::items::Item;
use crate::xinput_hook::{install_xinput_hook, remove_xinput_hook, set_suppress_camera};

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
    match reason {
        DLL_PROCESS_ATTACH => { std::thread::spawn(move || init(hmodule)); },
        DLL_PROCESS_DETACH => deinit(),
        _ => {}
    }

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

fn deinit() {
    tracing::info!("DLL detach called");
    remove_hudhook();
    remove_xinput_hook();
}

lazy_static!(
    pub static ref PROGRAM_START: Instant = Instant::now();
);

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
    tracing::info!("Game Path: {:?}", paths::game());
    tracing::info!("Awaiting system init");
    wait_for_system_init(&Program::current(), Duration::MAX)
        .expect("Could not await system init.");

    if !Settings::read_or_default().await_xinput_hook {
        install_xinput_hook();
    }

    tracing::info!("Initializing IconManager and FontManager asynchronously");
    std::thread::spawn(IconManager::init);
    std::thread::spawn(FontManager::init);

    tracing::info!("Init complete");
    let tasks = unsafe { CSTaskImp::instance() }.expect("Could not get CSTaskImp");
    tracing::info!("Creating gamepad state");
    let gamepad_state = Arc::new(Mutex::new(GamepadState::new()));
    if set_gamepad_state(gamepad_state).is_err() {
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
    static ref SELECTED_QUICK_ITEM_INDEX: AtomicI32 = AtomicI32::new(-1);
);

pub fn set_selected_spell_index(idx: i32) {
    SELECTED_SPELL_INDEX.store(idx, Ordering::Relaxed);
}

pub fn set_selected_quick_item_index(idx: i32) {
    SELECTED_QUICK_ITEM_INDEX.store(idx, Ordering::Relaxed);
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

lazy_static!(
    static ref PREV_WHEEL_TYPE: Arc<Mutex<WheelType>> = Arc::new(Mutex::new(WheelType::None));
);
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
        let selected_quick_item_index = SELECTED_QUICK_ITEM_INDEX.load(Ordering::Relaxed);
        if selected_quick_item_index != -1 {
            game_data_man.main_player_game_data.equipment.equip_item_data.selected_quick_slot = selected_quick_item_index;
            SELECTED_QUICK_ITEM_INDEX.store(-1, Ordering::Relaxed);
        }

        let mut equipped_spells = Vec::with_capacity(14);
        let mut equipped_quick_items = Vec::with_capacity(10);

        let spell_data: Vec<_> = game_data_man.main_player_game_data
            .equipment
            .equip_magic_data
            .entries
            .iter()
            .map(|entry| entry.param_id as u32)
            .enumerate()
            .collect();
        let quick_item_data: Vec<_> = game_data_man.main_player_game_data
            .equipment
            .equipment_entries
            .quick_tems
            .iter()
            .enumerate()
            .filter_map(|(idx, entry)| entry.param_id().map(|param_id| (idx, param_id)))
            .collect();

        for (equipped, data) in [
            (&mut equipped_spells, spell_data), (&mut equipped_quick_items, quick_item_data)
        ] {
            for (idx, id) in data {
                if let Some(item) = Item::try_new(param_repo, idx as i32, id) {
                    equipped.push(item);
                }
            }
        }

        if is_debugging() {
            add_to_screen_debug(format!("Equipped spells: {equipped_spells:?}"));
            add_to_screen_debug(format!("Equipped quick items: {equipped_quick_items:?}"));

            run_every!("D equipped" every Duration::from_secs(1) => {
                tracing::info!("Equipped spells: {equipped_spells:?}");
                tracing::info!("Equipped quick items: {equipped_quick_items:?}");
            });
        }

        ItemWheelData::mutate(|data| {
            data.spells = equipped_spells;
            data.quick_items = equipped_quick_items;
        });
        let selected_wheel_type = selected_wheel_type();
        if *PREV_WHEEL_TYPE.lock().unwrap() != selected_wheel_type {
            *PREV_WHEEL_TYPE.lock().unwrap() = selected_wheel_type;
            let is_wheel_open = selected_wheel_type != WheelType::None;
            menu_man.disable_mouse_cursor = !is_wheel_open;
            set_suppress_camera(is_wheel_open);
        }
        ItemWheelData::mutate(|data| {
            data.wheel_type = selected_wheel_type;
        });

        commit_screen_debug();
    );
}