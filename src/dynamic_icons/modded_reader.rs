use std::fs;
use std::path::{Path, PathBuf};
use crate::dynamic_icons::generic_reader;
use crate::dynamic_icons::read_success::ReadSuccess;
use crate::paths;
use crate::util::AddSpan;

pub fn search_for_mod_folder() -> Option<PathBuf> {
    let mut path = paths::dll();
    while path.pop() {
        if path.join("mod/menu/hi").exists() {
            return Some(path);
        }
    }
    None
}

pub fn read<P: AsRef<Path>>(p: P) -> anyhow::Result<ReadSuccess> {
    let bnd = p.as_ref().join("mod/menu/hi/01_common.sblytbnd.dcx");
    let bnd_bytes = fs::read(bnd).add_span()?;

    let tpf = p.as_ref().join("mod/menu/hi/01_common.tpf.dcx");
    let tpf_bytes = fs::read(tpf).add_span()?;

    generic_reader::read(&bnd_bytes, &tpf_bytes)
}