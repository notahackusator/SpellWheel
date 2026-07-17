use std::fmt::Debug;
use crate::dynamic_icons::modded_reader;
use crate::icons::generic_loader;
use std::path::Path;
use std::time::Instant;
use crate::icons::await_graphics::AwaitGraphics;
use crate::util::AddSpan;

pub fn load_icons<P>(await_graphics: &mut Vec<AwaitGraphics>, path: P) -> anyhow::Result<()>
where P: AsRef<Path> + Debug {
    let start = Instant::now();
    let mod_path = format!("{path:?}");
    let mut read_success = modded_reader::read(path).add_span()?;
    let out = generic_loader::parse_bnd_and_tpf(mod_path, await_graphics, &mut read_success);
    let time = start.elapsed();
    if let Ok(atlas_size) = out {
        tracing::info!("Finished initializing modded icons ({atlas_size}) in {time:?}");
    } else {
        tracing::info!("Finished initializing modded icons in {time:?}");
    }
    out.map(|_| ())
}