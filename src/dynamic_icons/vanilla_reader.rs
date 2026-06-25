use crate::dynamic_icons::generic_reader;
use crate::dynamic_icons::keys::KeyProvider;
use crate::dynamic_icons::read_success::ReadSuccess;
use crate::paths;
use fstools_dvdbnd::DvdBnd;
use crate::util::AddSpan;

pub fn read() -> anyhow::Result<ReadSuccess> {
    let paths = [
        paths::game_directory().join("Data0.bhd"),
        paths::game_directory().join("Data0.bdt")
    ];

    let key_provider = KeyProvider;
    let dvd_bnd = DvdBnd::create(paths, &key_provider).add_span()?;

    let bnd_bytes = dvd_bnd.open("menu/hi/01_common.sblytbnd.dcx").add_span()?;
    let tpf_bytes = dvd_bnd.open("menu/hi/01_common.tpf.dcx").add_span()?;

    let bnd_bytes = bnd_bytes.data();
    let tpf_bytes = tpf_bytes.data();

    generic_reader::read(bnd_bytes, tpf_bytes)
}