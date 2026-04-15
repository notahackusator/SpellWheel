#![allow(unsafe_op_in_unsafe_fn)]

mod rendering;
mod debugging;
mod keyboard;
mod icons;
mod settings;
pub mod spells;
pub mod paths;

use std::fs::File;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::OnceLock;
use std::time::Duration;
use eldenring::cs::{CSMenuManImp, CSTaskGroupIndex, CSTaskImp, GameDataMan, Magic, SoloParam, SoloParamRepository};
use eldenring::fd4::FD4TaskData;
use eldenring::util::system::wait_for_system_init;
use fromsoftware_shared::{FromStatic, Program, SharedTaskImpExt};
use lazy_static::lazy_static;
use tracing_subscriber::fmt;
use crate::debugging::{is_debugging, run_every, run_once};
use crate::keyboard::is_player_selecting_spell;
use crate::rendering::{try_init_rendering, SpellWheelData};
use crate::settings::Settings;
use crate::spells::Spell;

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

    std::thread::spawn(move || init(hmodule));

    true
}

fn init(hmodule: usize) {
    HMODULE.set(hmodule).expect("Could not set HMODULE");
    // Fix for Seamless crash
    std::thread::sleep(Duration::from_secs_f32(Settings::read_or_default().timing_offset));

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

fn start() {
    tracing::info!("Awaiting system init");
    wait_for_system_init(&Program::current(), Duration::MAX)
        .expect("Could not await system init.");

    tracing::info!("Init complete");
    let tasks = unsafe { CSTaskImp::instance() }.expect("Could not get CSTaskImp");
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

pub(crate) use guard;

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
        run_once!("passed all checks" => {
            tracing::info!("Passed all checks");
        });
        if is_debugging() {
            run_every!("D passed all checks" every Duration::from_secs(1) => {
                tracing::info!("Passed all checks");
            });
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
            if let Some(spell) = Spell::try_new(index as i32, id) {
                equipped_spells.push(spell);
            }
        }
        if is_debugging() {
            run_every!("D equipped spells" every Duration::from_secs(1) => {
                tracing::info!("spells: {equipped_spells:?}");
                tracing::info!("player selecting spell? (before update) = {}", is_player_selecting_spell());
            });
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
            }
            SpellWheelData::mutate(|data| {
                data.do_render = is_player_selecting_spell;
            });
        }
        if is_debugging() {
            run_every!("D player selecting spell" every Duration::from_secs(1) => {
                tracing::info!("player selecting spell? (after update) = {}", is_player_selecting_spell());
            });
        }
    );
}