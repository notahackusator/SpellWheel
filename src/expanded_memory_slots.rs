use std::ffi::c_void;
use std::mem::{size_of, transmute};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows::core::{s, w};

pub const MAX_SPELL_RECORDS: usize = 24;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct SpellLayoutV1 {
    struct_size: u32,
    api_version: u32,
    spell_record_count: u32,
    spell_record_stride: u32,
    spell_records_offset: u32,
    selected_spell_index_offset: u32,
    equip_magic_data_size: u32,
}

type QuerySpellLayout = unsafe extern "C" fn(*mut SpellLayoutV1, u32) -> u32;
type CopySpellIds = unsafe extern "C" fn(*const c_void, *mut i32, u32) -> u32;
type SetSelectedSpellIndex = unsafe extern "C" fn(*mut c_void, i32) -> u32;

#[derive(Clone, Copy)]
struct Api {
    query_spell_layout: QuerySpellLayout,
    copy_spell_ids: CopySpellIds,
    set_selected_spell_index: SetSelectedSpellIndex,
}

static API: OnceLock<Api> = OnceLock::new();
static LOGGED_ACTIVE: AtomicBool = AtomicBool::new(false);

fn api() -> Option<&'static Api> {
    if let Some(api) = API.get() {
        return Some(api);
    }

    let module = unsafe { GetModuleHandleW(w!("ExpandedMemorySlots.dll")) }.ok()?;
    let query_spell_layout =
        unsafe { GetProcAddress(module, s!("ExpandedMemorySlots_QuerySpellLayout")) }?;
    let copy_spell_ids = unsafe { GetProcAddress(module, s!("ExpandedMemorySlots_CopySpellIds")) }?;
    let set_selected_spell_index =
        unsafe { GetProcAddress(module, s!("ExpandedMemorySlots_SetSelectedSpellIndex")) }?;

    let discovered = Api {
        query_spell_layout: unsafe { transmute(query_spell_layout) },
        copy_spell_ids: unsafe { transmute(copy_spell_ids) },
        set_selected_spell_index: unsafe { transmute(set_selected_spell_index) },
    };
    let _ = API.set(discovered);
    API.get()
}

fn active_layout(api: &Api) -> Result<Option<SpellLayoutV1>, ()> {
    let mut layout = SpellLayoutV1::default();
    let available =
        unsafe { (api.query_spell_layout)(&mut layout, size_of::<SpellLayoutV1>() as u32) };
    if available == 0 {
        return Ok(None);
    }
    if layout.struct_size < size_of::<SpellLayoutV1>() as u32
        || layout.api_version != 1
        || layout.spell_record_count == 0
        || layout.spell_record_count as usize > MAX_SPELL_RECORDS
        || layout.spell_record_stride != 8
    {
        return Err(());
    }

    if !LOGGED_ACTIVE.swap(true, Ordering::Relaxed) {
        tracing::info!(
            "Expanded Memory Slots compatibility active: records={}, selected_offset={:#x}",
            layout.spell_record_count,
            layout.selected_spell_index_offset,
        );
    }
    Ok(Some(layout))
}

pub fn spell_ids(equip_magic_data: *const c_void) -> Option<Vec<(usize, u32)>> {
    let api = api()?;
    let layout = active_layout(api).ok().flatten()?;
    let record_count = layout.spell_record_count as usize;
    let mut ids = vec![-1i32; record_count];
    let copied =
        unsafe { (api.copy_spell_ids)(equip_magic_data, ids.as_mut_ptr(), ids.len() as u32) };
    if copied as usize != record_count {
        return None;
    }

    Some(
        ids.into_iter()
            .enumerate()
            .filter_map(|(index, id)| (id >= 0).then_some((index, id as u32)))
            .collect(),
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionResult {
    Unavailable,
    Applied,
    Rejected,
}

pub fn select_spell(equip_magic_data: *mut c_void, selected_index: i32) -> SelectionResult {
    let Some(api) = api() else {
        return SelectionResult::Unavailable;
    };
    match active_layout(api) {
        Ok(Some(_)) => {}
        Ok(None) => return SelectionResult::Unavailable,
        Err(()) => return SelectionResult::Rejected,
    }

    if unsafe { (api.set_selected_spell_index)(equip_magic_data, selected_index) } != 0 {
        SelectionResult::Applied
    } else {
        SelectionResult::Rejected
    }
}
