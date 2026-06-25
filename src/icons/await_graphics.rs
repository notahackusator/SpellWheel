use std::collections::HashMap;
use hudhook::RenderContext;
use crate::icons::AtlasIcon;

pub type AwaitGraphics = Box<dyn FnOnce(&mut dyn RenderContext, &mut HashMap<u16, AtlasIcon>) -> anyhow::Result<()> + Send + Sync>;