## Assets
- Configurable Magicka game path  
  It's currently hardcoded as `/data/SteamLibrary/steamapps/common/Magicka/Content/` in `src/magicka_assets.rs`.  
  Ideally it would be a Bevy asset source, so that `"magicka:Levels/Swamp.lvl"` refers to `Magicka/Content/Levels/Swamp.lvl`.
- Load each asset only once  
  Lack of this causes more delay than necessary when switching to a level that has many NPCs/enemies.
  This would be done by loading assets through the Bevy asset system, which already deduplicates.
- Read character template special abilities, events, buffs, & auras

## Gameplay
- Combat, health, statuses
- Spellcasting, Magicks
- Multiplayer
- AI
- Drowning, freezing and walking over liquid
- Mouse movement and configurable controls

## Levels
- Collision that doesn't leave you stuck in the wc_s4 spawn  
  Theorizing that the original collision is actually supposed to be one-sided
- All level scripting conditions except for trigger areas
- Most level scripting actions except for character spawns and level changes

## Experience
- Sound and music
- Dialogue, cutscenes
- Blending material layers (between dirt/grass, rock/moss, etc)
- Per-character model tint colors
- Particle effects
- Skybox

## Quality
- Ensure unsupported or malformed assets don't cause panics
- Any amount of code organization/tidying
- Any amount of optimization
- Tests
