use std::cell::Cell;
use std::ffi::c_void;
use std::mem;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use hudhook::windows::core::Interface;
use hudhook::windows::Win32::Devices::HumanInterfaceDevice::{DirectInput8Create, GUID_SysKeyboard, IDirectInput8W, IDirectInputDevice8W};
use lazy_static::lazy_static;
use retour::{static_detour, GenericDetour};
use windows::core::s;
use windows::Win32::Foundation::HINSTANCE;
use windows::Win32::System::LibraryLoader::{GetModuleHandleA, GetModuleHandleW, GetProcAddress};
use crate::settings::display_settings::display_settings_open;

type GetDeviceStateFn = unsafe extern "system" fn(*mut c_void, u32, *mut c_void) -> i32;
static STATE_HOOK: OnceLock<GenericDetour<GetDeviceStateFn>> = OnceLock::new();
unsafe extern "system" fn get_device_state_detour(this: *mut c_void, cb: u32, data: *mut c_void) -> i32 {
    let hr = STATE_HOOK.get().unwrap().call(this, cb, data);
    if cb == 256 && !data.is_null() && hr >= 0 && display_settings_open() {
        std::ptr::write_bytes(data as *mut u8, 0, 256);
    }
    hr
}

pub unsafe fn install_dinput_hook() -> anyhow::Result<()> {
    let hinst = GetModuleHandleW(None)?;
    let mut di: Option<IDirectInput8W> = None;
    DirectInput8Create(mem::transmute(hinst), 0x0800, &IDirectInput8W::IID, &mut di as *mut _ as *mut *mut c_void, None)?;
    let di = di.unwrap();

    let mut dev: Option<IDirectInputDevice8W> = None;
    di.CreateDevice(&GUID_SysKeyboard, &mut dev, None)?;
    let dev = dev.unwrap();

    // COM object -> vtable pointer -> function pointers
    let vtbl = *(dev.as_raw() as *const *const usize);
    let target: GetDeviceStateFn = mem::transmute(*vtbl.add(9));

    let hook = GenericDetour::new(target, get_device_state_detour)?;
    hook.enable()?;
    let _ = STATE_HOOK.set(hook);
    Ok(())
}

lazy_static!(
    static ref INSTALLED: AtomicBool = AtomicBool::new(false);
);

pub fn install_keyboard_hooks() {
    if INSTALLED.load(Ordering::Relaxed) {
        return;
    }
    if let Err(err) = unsafe { install_dinput_hook() } {
        tracing::error!("Error installing keyboard hook: {err}");
    }
    INSTALLED.store(true, Ordering::Relaxed);
}