use crate::glyphs::fmg::Fmg;
use fstools_formats::bnd4::BND4;
use std::collections::HashSet;
use std::io::Cursor;

pub fn read(bytes: &[u8], out: &mut HashSet<char>) -> anyhow::Result<()> {
    let bnd4 = BND4::from_reader(Cursor::new(bytes))?;

    for layout_entry in bnd4.files.iter() {
        if !layout_entry.path.contains("GoodsName") {
            continue;
        }

        let bytes = bnd4.file_bytes(layout_entry);
        let fmg = Fmg::read(bytes)?;

        for entry in fmg.entries {
            let Some(text) = entry.text else {
                continue;
            };
            for char in text.chars() {
                out.insert(char);
            }
        }
    }

    Ok(())
}