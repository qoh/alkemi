use num_enum::TryFromPrimitive;
use winnow::{
    Parser, Result,
    combinator::seq,
    error::{StrContext, StrContextValue},
};

use crate::xnb::{Stream, quicklist, types::*};

pub type ConditionCollection = Vec<EventCollection>;

#[derive(Debug)]
pub struct EventCollection {
    pub condition: EventCondition,
    pub repeat: bool,
    pub event_storages: Vec<EventStorage>,
}

#[derive(Debug)]
pub struct EventCondition {
    pub condition_type: u8,
    pub hitpoints: i32,
    pub element_set: i32,
    pub threshold: f32,
    pub time: f32,
}

#[derive(Debug)]
pub struct EventStorage {}

pub fn condition_collection(input: &mut Stream) -> Result<ConditionCollection> {
    quicklist(event_collection).parse_next(input)
}

fn event_collection(input: &mut Stream) -> Result<EventCollection> {
    seq!(EventCollection {
        condition: event_condition,
        repeat: bool,
        event_storages: quicklist(event_storage)
    })
    .parse_next(input)
}

fn event_condition(input: &mut Stream) -> Result<EventCondition> {
    seq!(EventCondition {
        condition_type: u8,
        hitpoints: i32,
        element_set: i32,
        threshold: f32,
        time: f32,
    })
    .parse_next(input)
}

fn event_storage(input: &mut Stream) -> Result<EventStorage> {
    let event_type = u8
        .try_map(EventType::try_from)
        .context(StrContext::Expected(StrContextValue::Description(
            "a valid event storage type",
        )))
        .parse_next(input)?;
    match event_type {
        EventType::Damage => {
            let (_damage, _use_velocity) = (damage, bool).parse_next(input)?;
            Ok(EventStorage {})
        }
        EventType::Splash => {
            let (_damage, _radius) = (damage, f32).parse_next(input)?;
            Ok(EventStorage {})
        }
        EventType::Sound => {
            let (_sound_bank, _sound_name, _magnitude, _stop_on_remove) =
                (i32, string, f32, bool).parse_next(input)?;
            Ok(EventStorage {})
        }
        EventType::Effect => {
            let (_follow, _world_align, _effect_name) = (bool, bool, string).parse_next(input)?;
            Ok(EventStorage {})
        }
        EventType::Remove => {
            let _bounce = bool.parse_next(input)?;
            Ok(EventStorage {})
        }
        EventType::CameraShake => {
            let (_duration, _magnitude, _positional) = (f32, f32, bool).parse_next(input)?;
            Ok(EventStorage {})
        }
        EventType::SpawnDecal => {
            let (_i, _j, _scale) = (i32, i32, i32).parse_next(input)?;
            Ok(EventStorage {})
        }
        EventType::Blast => unimplemented!("item blast event"),
        EventType::SpawnCharacter => {
            let (
                _character_template_name,
                _animation_idle,
                _animation_spawn,
                _health,
                _order,
                _react_to,
                _reaction,
                _rotation,
                _offset,
            ) = (string, string, string, f32, u8, u8, u8, f32, vec3).parse_next(input)?;
            Ok(EventStorage {})
        }
        EventType::Overkill => Ok(EventStorage {}),
        EventType::SpawnGibs => {
            let (_start_index, _end_index) = (i32, i32).parse_next(input)?;
            Ok(EventStorage {})
        }
        EventType::SpawnItem => {
            let _wizard_item_name = string.parse_next(input)?;
            Ok(EventStorage {})
        }
        EventType::SpawnMagick => {
            let _magick_name = string.parse_next(input)?;
            Ok(EventStorage {})
        }
        EventType::SpawnMissile => {
            let (_weapon_type_name, _velocity, _directional) =
                (string, vec3, bool).parse_next(input)?;
            Ok(EventStorage {})
        }
        EventType::Light => {
            let (
                _radius,
                _diffuse_color,
                _ambient_color,
                _specular_amount,
                _variation_type,
                _variation_amount,
                _variation_speed,
            ) = (f32, vec3, vec3, f32, u8, f32, f32).parse_next(input)?;
            Ok(EventStorage {})
        }
        EventType::CastMagick => {
            let (_magick_type_name, _element_sets) = (string, quicklist(i32)).parse_next(input)?;
            Ok(EventStorage {})
        }
        EventType::DamageOwner => {
            let (_damage, _use_velocity) = (damage, bool).parse_next(input)?;
            Ok(EventStorage {})
        }
        EventType::Callback => unimplemented!("deserialize item callback event"),
    }
}

fn damage(input: &mut Stream) -> Result<()> {
    let (_attack_properties, _elements, _amount, _magnitude) =
        (i32, i32, f32, f32).parse_next(input)?;
    Ok(())
}

#[derive(Debug, Clone, Copy, TryFromPrimitive)]
#[repr(u8)]
enum EventType {
    Damage,
    Splash,
    Sound,
    Effect,
    Remove,
    CameraShake,
    SpawnDecal,
    Blast,
    SpawnCharacter,
    Overkill,
    SpawnGibs,
    SpawnItem,
    SpawnMagick,
    SpawnMissile,
    Light,
    CastMagick,
    DamageOwner,
    Callback,
}

pub fn special_ability(input: &mut Stream) -> Result<()> {
    let _type_name = string.parse_next(input)?;
    let _s1 = string.parse_next(input)?;
    let _s2 = string.parse_next(input)?;
    let _element_sets = quicklist(i32).parse_next(input)?;
    Ok(())
}

pub fn aura(_input: &mut Stream) -> Result<()> {
    todo!("parse aura")
}
