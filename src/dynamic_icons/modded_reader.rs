use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use fstools_formats::bnd4::BND4;
use fstools_formats::dcx::DcxHeader;
use fstools_formats::tpf::TPF;
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

    let (_header, mut bnd_decoder) = DcxHeader::read(bnd_bytes.as_slice())
        .map_err(|e| anyhow::anyhow!("DCX read failed: {:?}", e)).add_span()?;
    let (_header, mut tpf_decoder) = DcxHeader::read(tpf_bytes.as_slice())
        .map_err(|e| anyhow::anyhow!("DCX read failed: {:?}", e)).add_span()?;

    let mut bnd_bytes = Vec::with_capacity(bnd_decoder.hint_size());
    bnd_decoder.read_to_end(&mut bnd_bytes).add_span()?;
    let mut tpf_bytes = Vec::with_capacity(bnd_decoder.hint_size());
    tpf_decoder.read_to_end(&mut tpf_bytes).add_span()?;

    let mut bnd_cursor = Cursor::new(bnd_bytes);
    let mut tpf_cursor = Cursor::new(tpf_bytes);

    let bnd = BND4::from_reader(&mut bnd_cursor).map_err(anyhow::Error::new).add_span()?;
    let tpf = TPF::from_reader(&mut tpf_cursor).map_err(anyhow::Error::new).add_span()?;

    Ok(ReadSuccess::new(bnd, tpf, tpf_cursor))
}