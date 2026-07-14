use std::collections::HashSet;
use crate::items::{read_utf16_string, Item};
use eldenring::cs::{EquipParamGoods, SoloParam, SoloParamRepository};
use eldenring::fd4::ParamFile;
use pmod::fmg::MsgRepository;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ParamFileMetadata([u32; 4]);

/// stolen from [SoloParamRepository::get_param_file]
pub unsafe fn metadata_ptr(data: &ParamFile) -> &ParamFileMetadata {
    let ptr = (data as *const ParamFile).byte_sub(size_of::<ParamFileMetadata>())
        as *const ParamFileMetadata;
    &*ptr
}

/// stolen from [ParamFile::lookup_table]
#[allow(unused)]
pub unsafe fn lookup_table(metadata: &ParamFileMetadata) -> &[[u32; 2]] {
    let stolen: ParamFileMetadata = *metadata;
    let file_size = stolen.0[0];
    let row_count = stolen.0[1];
    let aligned_file_size = file_size.next_multiple_of(0x10) as usize;

    let file_start = (metadata as *const ParamFileMetadata).add(1) as *const u8;
    std::slice::from_raw_parts(
        file_start.add(aligned_file_size) as *const [u32; 2],
        row_count as usize,
    )
}

pub unsafe fn read_text(param_repo: &SoloParamRepository) -> HashSet<char> {
    let mut chars = HashSet::new();
    let data = &param_repo.solo_param_holders[EquipParamGoods::INDEX as usize].get_res_cap(0).unwrap()
        .param_res_cap.data;
    let lookup_table = lookup_table(metadata_ptr(data));
    for &[param_id, _] in lookup_table.iter() {
        for string in [
            read_utf16_string(MsgRepository::get_msg(0, Item::BASE_GAME_ITEM_NAME, param_id)),
            read_utf16_string(MsgRepository::get_msg(0, Item::DLC_ITEM_NAME, param_id))
        ] {
            let Some(string) = string else {
                continue;
            };
            for char in string.chars() {
                chars.insert(char);
            }
        }
    }
    chars
}