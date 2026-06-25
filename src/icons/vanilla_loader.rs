use std::time::Instant;
use crate::dynamic_icons::vanilla_reader;
use crate::icons::await_graphics::AwaitGraphics;
use crate::icons::generic_loader;
use crate::util::AddSpan;

pub fn load_spells(await_graphics: &mut Vec<AwaitGraphics>) -> anyhow::Result<()> {
    let start = Instant::now();
    let mut read_success = vanilla_reader::read().add_span()?;
    let out = generic_loader::parse_bnd_and_tpf(await_graphics, &mut read_success);
    let time = start.elapsed();
    tracing::info!("Finished initializing vanilla spells in {time:?}");
    out
}