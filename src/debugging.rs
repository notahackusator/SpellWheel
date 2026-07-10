use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use eldenring::cs::{Magic, SoloParam, SoloParamRepository};
use eldenring::fd4::ParamFile;
use lazy_static::lazy_static;
use pmod::fmg::MsgRepository;
use crate::settings::Settings;
use crate::items::{read_utf16_string, Item};

#[allow(unused)]
pub unsafe fn hacked_lookup_table_lol(metadata: &ParamFileMetadata) -> &[[u32; 2]] {
    let stolen: ParamFileMetadata = *metadata;
    let file_size = stolen.0[0];
    let row_count = stolen.0[1];

    #[allow(unused_doc_comments)]
    /// stolen from [ParamHeaderMetadata::lookup_table]
    let aligned_file_size = file_size.next_multiple_of(0x10) as usize;

    let file_start = (metadata as *const ParamFileMetadata).add(1) as *const u8;
    std::slice::from_raw_parts(
        file_start.add(aligned_file_size) as *const [u32; 2],
        row_count as usize,
    )
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ParamFileMetadata([u32; 4]);

pub unsafe fn metadata_ptr(data: &ParamFile) -> &ParamFileMetadata {
    let ptr = (data as *const ParamFile).byte_sub(size_of::<ParamFileMetadata>())
        as *const ParamFileMetadata;
    &*ptr
}

pub unsafe fn name_candidates(param_id: u32) -> Vec<String> {
    let mut out = vec![];
    for i in 0..512 {
        if let Some(str) = read_utf16_string(MsgRepository::get_msg(
            0, i, param_id
        )) {
            out.push(format!("{i}: {str}"));
        }
    }
    out
}

#[allow(unused)]
pub unsafe fn log_all<P: SoloParam>(param_repo: &mut SoloParamRepository) {
    let data = &param_repo.solo_param_holders[P::INDEX as usize].get_res_cap(0).unwrap()
        .param_res_cap.data;
    let lookup_table = hacked_lookup_table_lol(metadata_ptr(data));
    for &[param_id, _] in lookup_table.iter() {
        tracing::info!("{param_id}={:?}", name_candidates(param_id));
    }
}

#[allow(unused)]
pub unsafe fn log_all_spell_names_hopefully(param_repo: &mut SoloParamRepository) {
    let data = &param_repo.solo_param_holders[Magic::INDEX as usize].get_res_cap(0).unwrap()
        .param_res_cap.data;
    let lookup_table = hacked_lookup_table_lol(metadata_ptr(data));
    for &[param_id, _] in lookup_table.iter() {
        tracing::info!("{param_id}={:?}", Item::get_name(param_id));
    }
}

#[allow(unused)]
pub unsafe fn log_all_spell_data_hopefully(param_repo: &mut SoloParamRepository) {
    let data = &param_repo.solo_param_holders[Magic::INDEX as usize].get_res_cap(0).unwrap()
        .param_res_cap.data;
    let lookup_table = hacked_lookup_table_lol(metadata_ptr(data));
    for &[param_id, _] in lookup_table.iter() {
        let spell = param_repo.get::<Magic>(param_id)
            .expect(&format!("Could not get spell id {param_id}"));
        tracing::info!("{param_id}: name={:?} icon_id={} sort_id={}",
            Item::get_name(param_id), spell.icon_id(), spell.sort_id());
    }
}

pub struct RunEveryRegistry {
    code: HashMap<String, (Duration, Instant)>
}

lazy_static!(
    static ref RUN_EVERY_REGISTRY: Arc<RwLock<RunEveryRegistry>> = Arc::new(RwLock::new(RunEveryRegistry::new()));
);

impl RunEveryRegistry {
    fn new() -> Self {
        Self {
            code: HashMap::new(),
        }
    }

    pub fn can_run(name: &str, every: Duration) -> bool {
        RUN_EVERY_REGISTRY.write().expect("RUN_EVERY_REGISTRY owner panicked")
            .can_run_inner(name, every)
    }

    pub fn can_run_string(name: String, every: Duration) -> bool {
        RUN_EVERY_REGISTRY.write().expect("RUN_EVERY_REGISTRY owner panicked")
            .can_run_inner_string(name, every)
    }

    fn can_run_inner(&mut self, name: &str, every: Duration) -> bool {
        match self.code.get_mut(name) {
            Some((duration, start)) => {
                let now = Instant::now();
                if now - *start >= *duration {
                    *start = Instant::now();
                    return true;
                }
                false
            }
            None => {
                self.code.insert(name.to_string(), (every, Instant::now()));
                true
            }
        }
    }

    fn can_run_inner_string(&mut self, name: String, every: Duration) -> bool {
        match self.code.get_mut(&name) {
            Some((duration, start)) => {
                let now = Instant::now();
                if now - *start >= *duration {
                    *start = Instant::now();
                    return true;
                }
                false
            }
            None => {
                self.code.insert(name, (every, Instant::now()));
                true
            }
        }
    }
}

macro_rules! run_every {
    ($some_unique_string_literal:literal every $duration:expr => $code:block) => {
        if crate::debugging::RunEveryRegistry::can_run($some_unique_string_literal, $duration) $code
    };
    ($some_unique_string:expr; every $duration:expr => $code:block) => {
        if crate::debugging::RunEveryRegistry::can_run_string($some_unique_string, $duration) $code
    };
}

pub(crate) use run_every;

macro_rules! run_once {
    ($some_unique_string:literal => $code:block) => {
        crate::debugging::run_every!($some_unique_string every core::time::Duration::from_secs(u64::MAX) => $code)
    };
}

pub(crate) use run_once;

pub fn is_debugging() -> bool {
    Settings::read_or_default().debugging
}

lazy_static!(
    static ref COMMITTED_SCREEN_DEBUG: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    static ref SCREEN_DEBUG: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
);

pub fn add_to_screen_debug(add: String) {
    SCREEN_DEBUG.lock().unwrap().push(add);
}

pub fn commit_screen_debug() {
    let mut screen_debug = SCREEN_DEBUG.lock().unwrap();
    *COMMITTED_SCREEN_DEBUG.lock().unwrap() = screen_debug.clone();
    screen_debug.clear();
}

pub fn read_committed_screen_debug() -> Vec<String> {
    std::mem::take(&mut *COMMITTED_SCREEN_DEBUG.lock().unwrap())
}