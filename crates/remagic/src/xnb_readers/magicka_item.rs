use winnow::{Parser, Result, combinator::cond};

use crate::{
    xnb::{Stream, TypeReaderMeta, quicklist, types::*},
    xnb_readers::{
        magicka_common::{aura, condition_collection, special_ability},
        xna_mesh::Model,
    },
};

#[derive(Debug)]
pub struct Item {
    pub name: String,
    pub display_name: String,
    pub description: String,
    /// Used to index animation sets
    pub weapon_class: u8,
    pub hide_model: bool,
    pub model: ExternalReference<Model>,
    pub scale: f32,
}

impl TypeReaderMeta for Item {
    const NAME: &'static str =
        "Magicka.ContentReaders.ItemReader, Magicka, Version=1.0.0.0, Culture=neutral";

    const VERSION: i32 = 0;
}

pub fn item(input: &mut Stream) -> Result<Item> {
    let (name, display_name, description) = (string, string, string).parse_next(input)?;
    let _sounds = quicklist((string, i32)).parse_next(input)?;
    let (
        _pickable,
        _bound,
        _block_value,
        weapon_class, // animation set index
        _cooldown_time,
        hide_model,
        _hide_effect,
        _pause_sounds,
    ) = (bool, bool, i32, u8, f32, bool, bool, bool).parse_next(input)?;

    let _resistances = quicklist((i32, f32, f32, bool)).parse_next(input)?;
    let (_passive_ability, _passive_ability_parameter) = (u8, f32).parse_next(input)?;
    let _effects = quicklist(string).parse_next(input)?;
    let _lights = quicklist((f32, vec3, vec3, f32, u8, f32, f32)).parse_next(input)?; // (radius, diffuse_color, ambient_color, specular_amount, variation_type, variation_amount, variation_speed)
    let _special_ability = bool
        .flat_map(|has| cond(has, (f32, special_ability)))
        .parse_next(input)?; // (recharge_time, ability)

    let (
        _melee_range,
        _melee_multi_hit,
        _melee_conditions,
        _ranged_range,
        _facing,
        _homing,
        _ranged_elevation_degrees,
        _ranged_danger,
        _gun_range,
        _gun_clip,
        _gun_rate,
        _gun_accuracy,
    ) = (
        f32,
        bool,
        condition_collection,
        f32,
        bool,
        f32,
        f32,
        f32,
        f32,
        i32,
        i32,
        f32,
    )
        .parse_next(input)?;
    let (
        _gun_sound_spec,
        _gun_muzzle_effect_name,
        _gun_shell_effect_name,
        _tracer_velocity,
        _sprite_spec_non_tracer,
        _sprite_spec_tracer,
        _gun_conditions,
        _projectile_model,
        _ranged_conditions,
        scale,
        model,
    ) = (
        string,
        string,
        string,
        f32,
        string,
        string,
        condition_collection,
        external_ref::<Model>,
        condition_collection,
        f32,
        external_ref,
    )
        .parse_next(input)?;

    let _auras = quicklist(aura).parse_next(input)?;

    Ok(Item {
        name: name.to_owned(),
        display_name: display_name.to_owned(),
        description: description.to_owned(),
        weapon_class,
        hide_model,
        model,
        scale,
    })
}
