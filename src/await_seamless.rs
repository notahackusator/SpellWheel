use eldenring::cs::CSMenuManImp;
use fromsoftware_shared::FromStatic;
use std::time::Duration;
use windows::core::s;
use windows::Win32::System::LibraryLoader::GetModuleHandleA;

pub fn is_seamless_coop_active() -> bool {
    unsafe {
        !GetModuleHandleA(s!("elden_ring_seamless_coop.dll")).is_ok()
    }
}

pub fn await_seamless() {
    while unsafe { CSMenuManImp::instance() }.is_err() {
        std::thread::sleep(Duration::from_millis(50));
    }
}