use std::collections::HashSet;
use std::fmt::Display;
use std::fs;
use std::path::Path;
use crate::glyphs::generic_reader;

pub fn read<P: AsRef<Path>, L: AsRef<Path> + Display>(p: P, lang: L, out: &mut HashSet<char>) -> anyhow::Result<()> {
    let bytes = [
        fs::read(p.as_ref().join(format!("msg/{lang}/item.msgbnd.dcx"))),
        fs::read(p.as_ref().join(format!("msg/{lang}/item_dlc01.msgbnd.dcx"))),
        fs::read(p.as_ref().join(format!("msg/{lang}/item_dlc02.msgbnd.dcx"))),
    ];

    for bytes in bytes.iter() {
        let Ok(bytes) = bytes else {
            continue;
        };
        if let Err(err) = generic_reader::read(bytes, out) {
            tracing::error!("Error loading language file {lang}: {err:?}");
        }
    }

    Ok(())
}