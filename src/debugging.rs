use eldenring::cs::{ActionButtonParam, Magic, SoloParam, SoloParamRepository};
use eldenring::fd4::ParamHeaderMetadata;
use crate::get_spell_name;

#[allow(unused)]
unsafe fn hacked_lookup_table_lol(metadata: &ParamHeaderMetadata) -> &[[u32; 2]] {
    let stolen: [u32; 4] = *(metadata as *const _ as *const [u32; 4]);
    let file_size = stolen[0];
    let row_count = stolen[1];

    #[allow(unused_doc_comments)]
    /// stolen from [ParamHeaderMetadata::lookup_table]
    let aligned_file_size = file_size.next_multiple_of(0x10) as usize;

    let file_start = (metadata as *const ParamHeaderMetadata).add(1) as *const u8;
    std::slice::from_raw_parts(
        file_start.add(aligned_file_size) as *const [u32; 2],
        row_count as usize,
    )
}

#[allow(unused)]
unsafe fn log_all_spell_names_hopefully(param_repo: &mut SoloParamRepository) {
    let data = &param_repo.solo_param_holders[Magic::INDEX as usize].get_res_cap(0).unwrap()
        .param_res_cap.data;
    let lookup_table = hacked_lookup_table_lol(data.metadata());
    for &[param_id, _] in lookup_table.iter() {
        tracing::info!("{param_id}={:?}", get_spell_name(param_id));
    }
}

#[allow(unused)]
unsafe fn log_all_spell_data_hopefully(param_repo: &mut SoloParamRepository) {
    let data = &param_repo.solo_param_holders[Magic::INDEX as usize].get_res_cap(0).unwrap()
        .param_res_cap.data;
    let lookup_table = hacked_lookup_table_lol(data.metadata());
    for &[param_id, _] in lookup_table.iter() {
        let spell = param_repo.get::<Magic>(param_id)
            .expect(&format!("Could not get spell id {param_id}"));
        tracing::info!("{param_id}: name={:?} icon_id={} sort_id={}",
            get_spell_name(param_id), spell.icon_id(), spell.sort_id());
    }
}