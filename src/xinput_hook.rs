use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock, RwLock};
use lazy_static::lazy_static;
use retour::static_detour;
use windows::core::s;
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryA};
use windows::Win32::UI::Input::XboxController::{XINPUT_GAMEPAD, XINPUT_STATE};
use crate::debugging::{add_to_screen_debug, is_debugging};

// Lazy static to prevent bullshittery
lazy_static!(
    static ref SUPPRESS_CAMERA: AtomicBool = AtomicBool::new(false);
    static ref HOOKED_STATE: Arc<RwLock<XINPUT_STATE>> = Arc::new(RwLock::default());
);

pub fn get_xinput_gamepad_state() -> XINPUT_GAMEPAD {
    HOOKED_STATE.read().unwrap().Gamepad
}

pub fn set_suppress_camera(suppress_camera: bool) {
    SUPPRESS_CAMERA.store(suppress_camera, Ordering::Relaxed);
}

static_detour! {
    static XInputHook: unsafe extern "system" fn(u32, *mut XINPUT_STATE) -> u32;
}

fn hooked_xinput_get_state(
    user_index: u32,
    state: *mut XINPUT_STATE,
) -> u32 {
    unsafe {
        let result = XInputHook.call(user_index, state);

        *HOOKED_STATE.write().unwrap() = *state;

        let suppress_camera = SUPPRESS_CAMERA.load(Ordering::Relaxed);
        if is_debugging() {
            add_to_screen_debug(format!("Camera suppressed? = {suppress_camera}"));
            add_to_screen_debug(format!("Stored state 1: {:?}", (*HOOKED_STATE.read().unwrap()).Gamepad));
            add_to_screen_debug(format!("Stored state 2: {:?}", get_xinput_gamepad_state()));
            add_to_screen_debug(format!("Original state: {:?}", (*state).Gamepad));
        }
        if suppress_camera && !state.is_null() {
            (*state).Gamepad.sThumbRX = 0;
            (*state).Gamepad.sThumbRY = 0;
        }
        if is_debugging() {
            add_to_screen_debug(format!("New state: {:?}", (*state).Gamepad));
        }

        result
    }
}

pub fn install_xinput_hook() -> bool {
    unsafe {
        let modules = [
            LoadLibraryA(s!("xinput1_4.dll")),
            LoadLibraryA(s!("xinput1_3.dll")),
            LoadLibraryA(s!("xinput9_1_0.dll")),
        ];

        for (i, module) in modules.into_iter().enumerate() {
            match module {
                Err(err) => tracing::info!("Could not load DLL {i}: {err}"),
                Ok(module) => {
                    let target = GetProcAddress(module, s!("XInputGetState"));

                    // Install hook with retour
                    unsafe {
                        XInputHook
                            .initialize(
                                std::mem::transmute(target),
                                hooked_xinput_get_state,
                            )
                            .expect("Hooked twice into XInput")
                            .enable()
                            .expect("XInput hook not initialized despite call");
                    }

                    tracing::info!("Installed XInputGetState hook from DLL {i}");
                    return true;
                }
            }
        }

        tracing::error!("Unable to locate an XInput DLL to hook");
        false
    }
}