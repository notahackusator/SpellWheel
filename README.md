<h1>Elden Ring Spell Wheel</h1>
Source code for the Spell Wheel mod, which allows you to quickly
switch spells without having to cycle through them one by one.

Nexus page: https://www.nexusmods.com/eldenring/mods/9636

<h2>Modded Spell Icons</h2>
<h3>Introduction</h3>
<p>
In order to show icons of modded spells,
the user must specify a path to a JSON file in the
<code>spellwheel.toml</code> settings file,
in the <code>modded_spells</code> array.<br>
The paths specified are relative to
<em>ELDEN RING/Game/mods/spellwheel/icons</em>,
so for example if you wanted to reference a JSON file in
<em>ELDEN RING/Game/mods/spellwheel/icons/ModdedSpells.json</em>,
you'd simply write "ModdedSpells.json" in the array.
Just like standard file system navigation,
you can use ../ to go up a directory, and you can
enter directories with a slash.
</p>
<h3>Creating modded spell JSON files</h3>
<p>
The JSON files must be written like this:

```json
[
    {
        // Icon ID here:
        "id": 1337
        // Path to icon image, relative to spellwheel/icons:
        "path_to_icon": "path/to/icon.png"
    }
]
```
</p>
