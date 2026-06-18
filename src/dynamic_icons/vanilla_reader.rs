use crate::dynamic_icons::keys::KeyProvider;
use crate::paths;
use fstools_dvdbnd::DvdBnd;
use fstools_formats::dcx::DcxHeader;
use fstools_formats::tpf::TPF;
use std::io::{Cursor, Read};

pub fn read() -> anyhow::Result<TPF> {
    todo!();
    let paths = [
        paths::game().join("Data0").with_extension("bhd"),
        paths::game().join("Data0").with_extension("bdt")
    ];

    let key_provider = KeyProvider;
    let dvd_bnd = DvdBnd::create(paths, &key_provider)?;

    let mut reader = dvd_bnd.open("menu/hi/01_common.sblytbnd.dcx")?;
    let (_header, mut decoder) = DcxHeader::read(&mut reader)
        .map_err(|e| anyhow::anyhow!("DCX read failed: {:?}", e))?;

    let mut bnd_bytes = Vec::with_capacity(decoder.hint_size());
    decoder.read_to_end(&mut bnd_bytes)?;

    TPF::from_reader(&mut Cursor::new(bnd_bytes.clone())).map_err(anyhow::Error::new)
}