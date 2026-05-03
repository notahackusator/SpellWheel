use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;
use hudhook::windows::Win32::System::Threading::GetCurrentProcessId;
use hudhook::windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_TAB};
use hudhook::windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};
use lazy_static::lazy_static;
use windows::Win32::UI::Input::XboxController::XINPUT_GAMEPAD_DPAD_UP;
use crate::debugging::{add_to_screen_debug, is_debugging, run_every};
use crate::{gamepad_state, in_menus, PROGRAM_START};
use crate::settings::Settings;

lazy_static!(
	static ref KEYS: HashMap<&'static str, u16> = HashMap::from_iter([
		("0", 48u16),
		("1", 49u16),
		("2", 50u16),
		("3", 51u16),
		("4", 52u16),
		("5", 53u16),
		("6", 54u16),
		("7", 55u16),
		("8", 56u16),
		("9", 57u16),
		("A", 65u16),
		("ABNT_C1", 193u16),
		("ABNT_C2", 194u16),
		("ACCEPT", 30u16),
		("ADD", 107u16),
		("APPS", 93u16),
		("ATTN", 246u16),
		("B", 66u16),
		("BACK", 8u16),
		("BROWSER_BACK", 166u16),
		("BROWSER_FAVORITES", 171u16),
		("BROWSER_FORWARD", 167u16),
		("BROWSER_HOME", 172u16),
		("BROWSER_REFRESH", 168u16),
		("BROWSER_SEARCH", 170u16),
		("BROWSER_STOP", 169u16),
		("C", 67u16),
		("CANCEL", 3u16),
		("CAPITAL", 20u16),
		("CLEAR", 12u16),
		("CONTROL", 17u16),
		("CONVERT", 28u16),
		("CRSEL", 247u16),
		("D", 68u16),
		("DBE_ALPHANUMERIC", 240u16),
		("DBE_CODEINPUT", 250u16),
		("DBE_DBCSCHAR", 244u16),
		("DBE_DETERMINESTRING", 252u16),
		("DBE_ENTERDLGCONVERSIONMODE", 253u16),
		("DBE_ENTERIMECONFIGMODE", 248u16),
		("DBE_ENTERWORDREGISTERMODE", 247u16),
		("DBE_FLUSHSTRING", 249u16),
		("DBE_HIRAGANA", 242u16),
		("DBE_KATAKANA", 241u16),
		("DBE_NOCODEINPUT", 251u16),
		("DBE_NOROMAN", 246u16),
		("DBE_ROMAN", 245u16),
		("DBE_SBCSCHAR", 243u16),
		("DECIMAL", 110u16),
		("DELETE", 46u16),
		("DIVIDE", 111u16),
		("DOWN", 40u16),
		("E", 69u16),
		("END", 35u16),
		("EREOF", 249u16),
		("ESCAPE", 27u16),
		("EXECUTE", 43u16),
		("EXSEL", 248u16),
		("F", 70u16),
		("F1", 112u16),
		("F10", 121u16),
		("F11", 122u16),
		("F12", 123u16),
		("F13", 124u16),
		("F14", 125u16),
		("F15", 126u16),
		("F16", 127u16),
		("F17", 128u16),
		("F18", 129u16),
		("F19", 130u16),
		("F2", 113u16),
		("F20", 131u16),
		("F21", 132u16),
		("F22", 133u16),
		("F23", 134u16),
		("F24", 135u16),
		("F3", 114u16),
		("F4", 115u16),
		("F5", 116u16),
		("F6", 117u16),
		("F7", 118u16),
		("F8", 119u16),
		("F9", 120u16),
		("FINAL", 24u16),
		("G", 71u16),
		("GAMEPAD_A", 195u16),
		("GAMEPAD_B", 196u16),
		("GAMEPAD_DPAD_DOWN", 204u16),
		("GAMEPAD_DPAD_LEFT", 205u16),
		("GAMEPAD_DPAD_RIGHT", 206u16),
		("GAMEPAD_DPAD_UP", 203u16),
		("GAMEPAD_LEFT_SHOULDER", 200u16),
		("GAMEPAD_LEFT_THUMBSTICK_BUTTON", 209u16),
		("GAMEPAD_LEFT_THUMBSTICK_DOWN", 212u16),
		("GAMEPAD_LEFT_THUMBSTICK_LEFT", 214u16),
		("GAMEPAD_LEFT_THUMBSTICK_RIGHT", 213u16),
		("GAMEPAD_LEFT_THUMBSTICK_UP", 211u16),
		("GAMEPAD_LEFT_TRIGGER", 201u16),
		("GAMEPAD_MENU", 207u16),
		("GAMEPAD_RIGHT_SHOULDER", 199u16),
		("GAMEPAD_RIGHT_THUMBSTICK_BUTTON", 210u16),
		("GAMEPAD_RIGHT_THUMBSTICK_DOWN", 216u16),
		("GAMEPAD_RIGHT_THUMBSTICK_LEFT", 218u16),
		("GAMEPAD_RIGHT_THUMBSTICK_RIGHT", 217u16),
		("GAMEPAD_RIGHT_THUMBSTICK_UP", 215u16),
		("GAMEPAD_RIGHT_TRIGGER", 202u16),
		("GAMEPAD_VIEW", 208u16),
		("GAMEPAD_X", 197u16),
		("GAMEPAD_Y", 198u16),
		("H", 72u16),
		("HANGEUL", 21u16),
		("HANGUL", 21u16),
		("HANJA", 25u16),
		("HELP", 47u16),
		("HOME", 36u16),
		("I", 73u16),
		("ICO_00", 228u16),
		("ICO_CLEAR", 230u16),
		("ICO_HELP", 227u16),
		("IME_OFF", 26u16),
		("IME_ON", 22u16),
		("INSERT", 45u16),
		("J", 74u16),
		("JUNJA", 23u16),
		("K", 75u16),
		("KANA", 21u16),
		("KANJI", 25u16),
		("L", 76u16),
		("LAUNCH_APP1", 182u16),
		("LAUNCH_APP2", 183u16),
		("LAUNCH_MAIL", 180u16),
		("LAUNCH_MEDIA_SELECT", 181u16),
		("LBUTTON", 1u16),
		("LCONTROL", 162u16),
		("LEFT", 37u16),
		("LMENU", 164u16),
		("LSHIFT", 160u16),
		("LWIN", 91u16),
		("M", 77u16),
		("MBUTTON", 4u16),
		("MEDIA_NEXT_TRACK", 176u16),
		("MEDIA_PLAY_PAUSE", 179u16),
		("MEDIA_PREV_TRACK", 177u16),
		("MEDIA_STOP", 178u16),
		("MENU", 18u16),
		("MODECHANGE", 31u16),
		("MULTIPLY", 106u16),
		("N", 78u16),
		("NAVIGATION_ACCEPT", 142u16),
		("NAVIGATION_CANCEL", 143u16),
		("NAVIGATION_DOWN", 139u16),
		("NAVIGATION_LEFT", 140u16),
		("NAVIGATION_MENU", 137u16),
		("NAVIGATION_RIGHT", 141u16),
		("NAVIGATION_UP", 138u16),
		("NAVIGATION_VIEW", 136u16),
		("NEXT", 34u16),
		("NONAME", 252u16),
		("NONCONVERT", 29u16),
		("NUMLOCK", 144u16),
		("NUMPAD0", 96u16),
		("NUMPAD1", 97u16),
		("NUMPAD2", 98u16),
		("NUMPAD3", 99u16),
		("NUMPAD4", 100u16),
		("NUMPAD5", 101u16),
		("NUMPAD6", 102u16),
		("NUMPAD7", 103u16),
		("NUMPAD8", 104u16),
		("NUMPAD9", 105u16),
		("O", 79u16),
		("OEM_1", 186u16),
		("OEM_102", 226u16),
		("OEM_2", 191u16),
		("OEM_3", 192u16),
		("OEM_4", 219u16),
		("OEM_5", 220u16),
		("OEM_6", 221u16),
		("OEM_7", 222u16),
		("OEM_8", 223u16),
		("OEM_ATTN", 240u16),
		("OEM_AUTO", 243u16),
		("OEM_AX", 225u16),
		("OEM_BACKTAB", 245u16),
		("OEM_CLEAR", 254u16),
		("OEM_COMMA", 188u16),
		("OEM_COPY", 242u16),
		("OEM_CUSEL", 239u16),
		("OEM_ENLW", 244u16),
		("OEM_FINISH", 241u16),
		("OEM_FJ_JISHO", 146u16),
		("OEM_FJ_LOYA", 149u16),
		("OEM_FJ_MASSHOU", 147u16),
		("OEM_FJ_ROYA", 150u16),
		("OEM_FJ_TOUROKU", 148u16),
		("OEM_JUMP", 234u16),
		("OEM_MINUS", 189u16),
		("OEM_NEC_EQUAL", 146u16),
		("OEM_PA1", 235u16),
		("OEM_PA2", 236u16),
		("OEM_PA3", 237u16),
		("OEM_PERIOD", 190u16),
		("OEM_PLUS", 187u16),
		("OEM_RESET", 233u16),
		("OEM_WSCTRL", 238u16),
		("P", 80u16),
		("PA1", 253u16),
		("PACKET", 231u16),
		("PAUSE", 19u16),
		("PLAY", 250u16),
		("PRINT", 42u16),
		("PRIOR", 33u16),
		("PROCESSKEY", 229u16),
		("Q", 81u16),
		("R", 82u16),
		("RBUTTON", 2u16),
		("RCONTROL", 163u16),
		("RETURN", 13u16),
		("RIGHT", 39u16),
		("RMENU", 165u16),
		("RSHIFT", 161u16),
		("RWIN", 92u16),
		("S", 83u16),
		("SCROLL", 145u16),
		("SELECT", 41u16),
		("SEPARATOR", 108u16),
		("SHIFT", 16u16),
		("SLEEP", 95u16),
		("SNAPSHOT", 44u16),
		("SPACE", 32u16),
		("SUBTRACT", 109u16),
		("T", 84u16),
		("TAB", 9u16),
		("U", 85u16),
		("UP", 38u16),
		("V", 86u16),
		("VOLUME_DOWN", 174u16),
		("VOLUME_MUTE", 173u16),
		("VOLUME_UP", 175u16),
		("W", 87u16),
		("X", 88u16),
		("XBUTTON1", 5u16),
		("XBUTTON2", 6u16),
		("Y", 89u16),
		("Z", 90u16),
		("ZOOM", 251u16),
		("_none_", 255u16),
	]);
);

fn is_game_focused() -> bool {
    unsafe {
        let foreground = GetForegroundWindow();
        let mut pid = 0u32;
        GetWindowThreadProcessId(foreground, Some(&mut pid));
        pid == GetCurrentProcessId()
    }
}

fn is_key_pressed(vk: i32) -> bool {
	unsafe {
		GetAsyncKeyState(vk) & 0x8000u16 as i16 != 0
	}
}

pub fn is_pressed(vk: i32) -> bool {
	is_game_focused() && is_key_pressed(vk)
}

lazy_static!(
	static ref PREV_KEY: Mutex<Option<String>> = Mutex::new(None);
);
pub fn is_player_selecting_spell() -> bool {
	if in_menus() {
		if is_debugging() {
			add_to_screen_debug("Player in menus".to_string());
		}
		return false;
	}
	let settings = Settings::read_or_default();
	match settings.using_controller {
		true => {
			let pressed = gamepad_state().pressed;

			// The trailing_zeros call returns what power of 2 it is,
			// since the XINPUT_GAMEPAD constants are all powers of 2.
			let dpad_up = XINPUT_GAMEPAD_DPAD_UP.0.trailing_zeros() as usize;

			let is_pressed = pressed[dpad_up] != *PROGRAM_START;
			let pressed_for_long_enough = pressed[dpad_up].elapsed() >
				Duration::from_secs_f32(settings.controller_wheel_open_delay);

			is_pressed && pressed_for_long_enough
		}
		false => {
			let mut prev_key_mutex = PREV_KEY.lock().unwrap();
			let key = settings.key;
			let key_changed = match prev_key_mutex.as_ref() {
				Some(prev_key) => &key == prev_key,
				None => false
			};
			let key_valid = KEYS.contains_key(key.as_str());
			if key_changed && !key_valid {
				tracing::error!("Invalid key: {key}");
			}

			let key_code = *KEYS.get(key.as_str())
				.unwrap_or(&VK_TAB.0) as i32;
			let out = is_pressed(key_code);
			if is_debugging() {
				run_every!("key valid / pressed" every Duration::from_secs(1) => {
					tracing::info!("{key}: code = {key_code}, valid? = {key_valid}, pressed? = {}, focused? = {}", is_key_pressed(key_code), is_game_focused())
				});
			}
			*prev_key_mutex = Some(key);
			out
		}
	}
}