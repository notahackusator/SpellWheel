use crate::dynamic_icons::keys::KeyProvider;
use crate::paths;
use crate::util::AddSpan;
use fstools_dvdbnd::DvdBnd;
use std::collections::HashSet;
use std::fmt::Display;
use std::io::Read;
use std::path::Path;
use fstools_formats::dcx::DcxHeader;
use crate::glyphs::generic_reader;

pub fn read<P: AsRef<Path> + Display>(lang: P, out: &mut HashSet<char>) -> anyhow::Result<()> {
    let paths = [
        paths::game_directory().join("Data0.bhd"),
        paths::game_directory().join("Data0.bdt"),
    ];

    let key_provider = KeyProvider;
    let dvd_bnd = DvdBnd::create(paths, &key_provider).add_span()?;
    
    let mut bytes = [
        dvd_bnd.open(format!("msg/{lang}/item.msgbnd.dcx")).add_span()?,
        dvd_bnd.open(format!("msg/{lang}/item_dlc01.msgbnd.dcx")).add_span()?,
        dvd_bnd.open(format!("msg/{lang}/item_dlc02.msgbnd.dcx")).add_span()?,
    ];

    for reader in bytes.iter_mut() {
        let (_header, mut decoder) = DcxHeader::read(reader)
            .map_err(|e| anyhow::anyhow!("DCX read failed: {:?}", e)).add_span()?;

        let mut bytes = Vec::with_capacity(decoder.hint_size());
        decoder.read_to_end(&mut bytes).add_span()?;

        if let Err(err) = generic_reader::read(&bytes, out) {
            tracing::error!("Error loading language file {lang}: {err:?}");
        }
    }

    Ok(())
}