use std::fs;
use std::io::{Cursor, Read};
use std::path::Path;
use fstools_formats::bnd4::BND4;
use fstools_formats::dcx::DcxHeader;
use fstools_formats::tpf::TPF;
use crate::dynamic_icons::read_success::ReadSuccess;

pub fn read<P: AsRef<Path>>(p: P) -> anyhow::Result<ReadSuccess> {
    let icons = p.as_ref().join("mod/menu/hi/01_common.sblytbnd.dcx");
    let mut icon_bytes = fs::read(icons)?;

    let (_header, mut decoder) = DcxHeader::read(icon_bytes.as_mut_slice())
        .map_err(|e| anyhow::anyhow!("DCX read failed: {:?}", e))?;

    let mut bnd_bytes = Vec::with_capacity(decoder.hint_size());
    decoder.read_to_end(&mut bnd_bytes)?;
    let tpf_bytes = bnd_bytes.clone();
    let mut tpf_cursor = Cursor::new(tpf_bytes);

    let bnd = BND4::from_reader(&mut Cursor::new(bnd_bytes)).map_err(anyhow::Error::new)?;
    let tpf = TPF::from_reader(&mut tpf_cursor).map_err(anyhow::Error::new)?;

    Ok(ReadSuccess::new(bnd, tpf, tpf_cursor))
}