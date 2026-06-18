use std::collections::HashMap;
use hudhook::RenderContext;
use imgui::TextureId;
use crate::icons::ModdedSpell;
use crate::paths;

pub fn load_modded_spell(all_modded_spells: &mut HashMap<u32, TextureId>,
                         render_context: &mut dyn RenderContext, modded_spell: &ModdedSpell) -> anyhow::Result<()> {
    let icon = image::open(paths::spell_icons().join(&modded_spell.path_to_icon))?;
    let texture_id = render_context.load_texture(icon.as_bytes(), icon.width(), icon.height())?;
    all_modded_spells.insert(modded_spell.id, texture_id);
    Ok(())
}

pub fn load_modded_spells(all_modded_spells: &mut HashMap<u32, TextureId>,
                          render_context: &mut dyn RenderContext, json: &str) -> anyhow::Result<()> {
    let json: Vec<ModdedSpell> = serde_json::from_str(json)?;
    for modded_spell in json {
        if let Err(err) = load_modded_spell(all_modded_spells, render_context, &modded_spell) {
            tracing::error!("Error loading modded spell {}: {}", modded_spell.id, err)
        }
    }
    Ok(())
}