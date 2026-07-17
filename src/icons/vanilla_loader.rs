use std::time::Instant;
use crate::dynamic_icons::vanilla_reader;
use crate::icons::await_graphics::AwaitGraphics;
use crate::icons::generic_loader;
use crate::util::AddSpan;

pub const BASE_GAME_SOURCE: &str = "Base Game";

pub fn load_icons(await_graphics: &mut Vec<AwaitGraphics>) -> anyhow::Result<()> {
    let start = Instant::now();
    let mut read_success = vanilla_reader::read().add_span()?;
    let out = generic_loader::parse_bnd_and_tpf(BASE_GAME_SOURCE.to_string(), await_graphics, &mut read_success);
    let time = start.elapsed();
    if let Ok(atlas_size) = out {
        tracing::info!("Finished initializing vanilla icons ({atlas_size}) in {time:?}");
    } else {
        tracing::info!("Finished initializing vanilla icons in {time:?}");
    }
    out.map(|_| ())
}