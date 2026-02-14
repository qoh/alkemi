use std::collections::HashMap;

use winnow::{
    Parser, Result,
    binary::length_repeat,
    combinator::{cond, repeat, seq},
    error::{ContextError, StrContext, StrContextValue},
};

use crate::{
    xnb::{Stream, TypeReaderMeta, quicklist, types::*},
    xnb_readers::{skinning::SkinnedModel, xna_mesh::Model},
};

// External reference types
#[derive(Debug, Clone)]
pub struct Item;

#[derive(Debug)]
pub struct CharacterTemplate {
    pub id: String,
    pub display_id: String,
    /*
    pub faction: i32,
    pub blood: i32,
    pub is_ethereal: bool,
    pub looks_ethereal: bool,
    pub fearless: bool,
    pub uncharmable: bool,
    pub nonslippery: bool,
    pub has_fairy: bool,
    pub can_see_invisible: bool,
    pub attached_sounds: Vec<(String, i32)>, // (key, banks)
    pub gibs: Vec<(ExternalReference<Model>, f32, f32)>, // (model, mass, scale)
    pub lights: Vec<(String, String, Vector3, Vector3, f32, u8, f32, f32)>, // (joint_name, radius, diffuse_color, ambient_color, specular_amount, variation_type, variation_amount, variation_speed)
    */
    pub max_hitpoints: f32,
    /*
    pub number_of_healthbars: i32,
    pub undying: bool,
    pub undie_time: f32,
    pub undie_hit_points: f32,
    pub hit_tolerance: i32,
    pub knockdown_tolerance: f32,
    pub score_value: i32,
    pub experience_value: i32,
    pub reward_on_kill: bool,
    pub reward_on_overkill: bool,
    pub regeneration: i32,
    pub max_panic: f32,
    pub zap_modifier: f32,
    */
    pub length: f32,
    pub radius: f32,
    pub mass: f32,
    pub speed: f32,
    pub turn_speed: f32,
    /*
    pub bleed_rate: f32,
    pub stun_time: f32,
    pub summon_element_bank: i32,
    pub summon_element_cue_string: String,
    pub resistances: Vec<(i32, f32, f32, bool)>, // (elements, multiplier, modifier, status_resistance)
    */
    pub skinned_models: (
        Vec<(ExternalReference<SkinnedModel>, f32, Vector3)>, // (model, scale, tint)
        ExternalReference<SkinnedModel>,                      // skeleton
    ),
    // attached_effects: Vec<(String, String)>,
    pub animation_sets: Vec<AnimationSet>,
    /*
    pub equipment: Vec<(i32, (String, Vector3, ExternalReference<Item>))>,
    pub event_conditions: Vec<(
        (u8, i32, i32, f32, f32), // event_condition
        bool,
        Vec<()>, // event_storage
    )>,
    pub alert_radius: f32,
    pub group_chase: f32,
    pub group_separation: f32,
    pub group_cohesion: f32,
    pub group_alignment: f32,
    pub group_wander: f32,
    pub friendly_avoidance: f32,
    pub enemy_avoidance: f32,
    pub sight_avoidance: f32,
    pub danger_avoidance: f32,
    pub anger_weight: f32,
    pub distance_weight: f32,
    pub health_weight: f32,
    pub flocking: bool,
    pub break_free_strength: f32,
    pub abilities: Vec<Ability>,
    pub move_animations: Vec<(u8, Vec<String>)>, // map<movement_properties, vec<animation>>
    pub buffs: Vec<()>,
    pub auras: Vec<()>,
    */
}

impl TypeReaderMeta for CharacterTemplate {
    const NAME: &'static str =
        "Magicka.ContentReaders.CharacterTemplateReader, Magicka, Version=1.0.0.0, Culture=neutral";

    const VERSION: i32 = 0;
}

pub fn character_template(input: &mut Stream) -> Result<CharacterTemplate> {
    let (id, display_id) = (
        string.map(ToOwned::to_owned), // id
        string.map(ToOwned::to_owned), // display_id
    )
        .parse_next(input)?;

    let a = (
        (
            i32,  // faction
            i32,  // blood type
            bool, // is_ethereal
            bool, // looks_ethereal
            bool, // fearless
            bool, // uncharmable
            bool, // nonslippery
            bool, // has_fairy
            bool, // can_see_invisible
        ),
        length_repeat::<_, _, Vec<_>, _, _, _, _>(
            i32.try_map(usize::try_from).map(|n| n.min(4)),
            (string.map(ToOwned::to_owned), i32),
        ), // attached_sounds(key, banks)
        quicklist((external_ref::<Model>, f32, f32)), // gibs(model, mass, scale)
        // lights(joint_name, radius, diffuse_color, ambient_color, specular_amount, variation_type, variation_amount, variation_speed)
        length_repeat::<_, _, Vec<_>, _, _, _, _>(
            i32.try_map(usize::try_from).verify(|n| *n <= 4),
            (
                string.map(ToOwned::to_owned),
                string.map(ToOwned::to_owned),
                vec3,
                vec3,
                f32,
                u8,
                f32,
                f32,
            ),
        ),
    )
        .parse_next(input)?;

    let max_hitpoints = f32.parse_next(input)?;

    let _ = (
        i32,  // number_of_healthbars
        bool, // undying
        f32,  // undie_time
        f32,  // undie_hit_points
        i32,  // hit_tolerance
        f32,  // knockdown_tolerance
    )
        .parse_next(input)?;
    let _ = (
        i32,  // score_value
        i32,  // experience_value
        bool, // reward_on_kill
        bool, // reward_on_overkill
        i32,  // regeneration
        f32,  // max_panic
        f32,  // zap_modifier
    )
        .parse_next(input)?;
    let (length, radius, mass, speed, turn_speed) = (
        f32, // length
        f32, // radius
        f32, // mass
        f32, // speed
        f32, // turn_speed
    )
        .parse_next(input)?;
    let _ = (
        f32,                              // bleed_rate
        f32,                              // stun_time
        i32,                              // summon_element_bank
        string.map(ToOwned::to_owned),    // summon_element_cue_string
        quicklist((i32, f32, f32, bool)), // resistances(elements, multiplier, modifier, status_resistance)
    )
        .parse_next(input)?;

    let skinned_models: (
        Vec<(ExternalReference<SkinnedModel>, _, _)>, // (model, scale, tint)
        ExternalReference<SkinnedModel>,              // skeleton
    ) = (quicklist((external_ref, f32, vec3)), external_ref).parse_next(input)?;

    let attached_effects =
        quicklist((string.map(ToOwned::to_owned), string.map(ToOwned::to_owned)))
            .parse_next(input)?;

    let animation_clip_action = seq!(AnimationEntry {
        clip_name: string.map(ToOwned::to_owned),
        speed: f32,
        blend_time: f32,
        repeat: bool,
        actions: quicklist(animation_action),
    });
    let animation_sets = repeat::<_, _, Vec<_>, _, _>(
        27,
        length_repeat(
            i32.try_map(usize::try_from),
            (string.map(ToOwned::to_owned), animation_clip_action),
        )
        .map(|animations| AnimationSet { animations }),
    )
    .parse_next(input)?;

    let equipment = quicklist((
        i32,
        (string.map(ToOwned::to_owned), vec3, external_ref::<Item>),
    ))
    .parse_next(input)?;

    let event_condition = (u8, i32, i32, f32, f32);
    let event_collection = (event_condition, bool, quicklist(event_storage));
    let event_conditions = quicklist(event_collection).parse_next(input);

    let d = (
        f32, // alert_radius
        f32, // group_chase
        f32, // group_separation
        f32, // group_cohesion
        f32, // group_alignment
        f32, // group_wander
        (
            f32, // friendly_avoidance
            f32, // enemy_avoidance
            f32, // sight_avoidance
            f32, // danger_avoidance
        ),
        f32,  // anger_weight
        f32,  // distance_weight
        f32,  // health_weight
        bool, // flocking
        f32,  // break_free_strength
    )
        .parse_next(input)?;

    let abilities = quicklist(ability).parse_next(input)?;
    let move_animations = quicklist((u8, quicklist(string))).parse_next(input)?; // map<movement_properties, vec<animation>>
    let buffs = quicklist(buff).parse_next(input)?;
    let auras = quicklist(aura).parse_next(input)?;

    Ok(CharacterTemplate {
        id,
        display_id,
        max_hitpoints,
        length,
        radius,
        mass,
        speed,
        turn_speed,
        skinned_models,
        animation_sets,
    })
}

/// A moveset for the character, such as while wielding a staff or while unarmed.
#[derive(Debug, Clone)]
pub struct AnimationSet {
    /// For each general animation name, what a character should do for it.
    /// For example, it might map the general animation "move_fall"
    /// to a specific clip "common_move_panic" in the character's skinned model,
    /// with footsteps events.
    pub animations: HashMap<String, AnimationEntry>,
}

/// What a character should when to play a particular common animation.
#[derive(Debug, Clone)]
pub struct AnimationEntry {
    /// The name of the clip in the character's skeleton skinned model.
    pub clip_name: String,
    /// The speed factor at which [`clip_name`] should be played.
    pub speed: f32,
    pub blend_time: f32,
    /// Should [`clip_name`] be played looping?
    pub repeat: bool,
    /// Events to trigger at specific points in time in [`clip_name`].
    pub actions: Vec<AnimationAction>,
}

#[derive(Debug, Clone)]
pub struct AnimationAction {
    pub start_time: f32,
    pub end_time: f32,
    pub data: AnimationActionData,
}
#[derive(Debug, Clone)]
pub enum AnimationActionData {
    Block {
        weapon: i32,
    },
    BreakFree {
        magnitude: f32,
        weapon: i32,
    },
    CameraShake {
        duration: f32,
        magnitude: f32,
    },
    CastSpell {
        source_not_from_staff: Option<String>,
    },
    Crouch {
        radius: f32,
        length: f32,
    },
    DamageGrip {
        damage_to_owner: bool,
        /// (attack_property, element, amount, magnitude)
        damages: Vec<(i32, i32, f32, f32)>,
    },
    DealDamage {
        weapon: i32,
        target: u8,
    },
    DetachItem {
        item: i32,
        velocity: Vector3,
    },
    Ethereal {
        ethereal: bool,
        alpha: f32,
        speed: f32,
    },
    Footstep,
    Grip {
        grip_type: u8,
        radius: f32,
        break_free_tolerance: f32,
        grip_attach_skeleton_bone_name: String,
        target_attach_skeleton_bone_name: String,
        finish_on_grip: bool,
    },
    Gunfire {
        weapon: i32,
        accuracy: f32,
    },
    Immortal {
        collide: bool,
    },
    Invisible {
        no_effect: bool,
    },
    Jump {
        elevation: f32,
        min_range: Option<f32>,
        max_range: Option<f32>,
    },
    Move {
        velocity: Vector3,
    },
    OverkillGrip,
    PlayEffect {
        skeleton_bone_name: String,
        attach: bool,
        effect: String,
    },
    PlaySound {
        sound: String,
        bank: i32,
    },
    ReleaseGrip,
    RemoveStatus {
        status_effect_name: String,
    },
    SetItemAttach {
        item: i32,
        joint_name: String,
    },
    SpawnMissile {
        weapon: i32,
        velocity: Vector3,
        item_aligned: bool,
    },
    SpecialAbility {},
    Suicide {
        overkill: bool,
    },
    ThrowGrip,
    Tongue {
        max_length: f32,
    },
    WeaponVisibility {
        weapon: i32,
        visible: bool,
    },
}
fn animation_action(input: &mut Stream) -> Result<AnimationAction> {
    let (type_name, start_time, end_time) = (string, f32, f32).parse_next(input)?;
    let data: AnimationActionData = match type_name {
        "Block" => seq!(AnimationActionData::Block { weapon: i32 }).parse_next(input)?,
        "BreakFree" => seq!(AnimationActionData::BreakFree {
            magnitude: f32,
            weapon: i32
        })
        .parse_next(input)?,
        "CameraShake" => seq!(AnimationActionData::CameraShake {
            duration: f32,
            magnitude: f32
        })
        .parse_next(input)?,
        "CastSpell" => bool
            .flat_map(|from_staff| cond(!from_staff, string.map(|s| s.to_owned())))
            .map(|o| AnimationActionData::CastSpell {
                source_not_from_staff: o,
            })
            .parse_next(input)?,
        "Crouch" => seq!(AnimationActionData::Crouch {
            radius: f32,
            length: f32
        })
        .parse_next(input)?,
        "DamageGrip" => seq!(AnimationActionData::DamageGrip {
            damage_to_owner: bool,
            damages: length_repeat::<_, _, Vec<_>, _, _, _, _>(
                i32.try_map(usize::try_from).verify(|n| *n <= 5),
                (i32, i32, f32, f32),
            )
        })
        .parse_next(input)?,
        "DealDamage" => seq!(AnimationActionData::DealDamage {
            weapon: i32,
            target: u8
        })
        .parse_next(input)?,
        "DetachItem" => seq!(AnimationActionData::DetachItem {
            item: i32,
            velocity: vec3
        })
        .parse_next(input)?,
        "Ethereal" => seq!(AnimationActionData::Ethereal {
            ethereal: bool,
            alpha: f32,
            speed: f32
        })
        .parse_next(input)?,
        "Footstep" => AnimationActionData::Footstep,
        "Grip" => seq!(AnimationActionData::Grip {
            grip_type: u8,
            radius: f32,
            break_free_tolerance: f32,
            grip_attach_skeleton_bone_name: string.map(ToOwned::to_owned),
            target_attach_skeleton_bone_name: string.map(ToOwned::to_owned),
            finish_on_grip: bool
        })
        .parse_next(input)?,
        "Gunfire" => seq!(AnimationActionData::Gunfire {
            weapon: i32,
            accuracy: f32
        })
        .parse_next(input)?,
        "Immortal" => seq!(AnimationActionData::Immortal { collide: bool }).parse_next(input)?,
        "Invisible" => {
            seq!(AnimationActionData::Invisible { no_effect: bool }).parse_next(input)?
        }
        "Jump" => seq!(AnimationActionData::Jump {
            elevation: f32,
            min_range: bool.flat_map(|has| cond(has, f32)),
            max_range: bool.flat_map(|has| cond(has, f32)),
        })
        .parse_next(input)?,
        "Move" => seq!(AnimationActionData::Move { velocity: vec3 }).parse_next(input)?,
        "OverkillGrip" => AnimationActionData::OverkillGrip,
        "PlayEffect" => seq!(AnimationActionData::PlayEffect { skeleton_bone_name: string.map(ToOwned::to_owned), attach: bool, effect: string.map(ToOwned::to_owned), _: f32 }).parse_next(input)?,
        "PlaySound" => seq!(AnimationActionData::PlaySound { sound: string.map(ToOwned::to_owned), bank: i32 }).parse_next(input)?,
        "ReleaseGrip" => AnimationActionData::ReleaseGrip,
        "RemoveStatus" => seq!(AnimationActionData::RemoveStatus { status_effect_name: string.map(ToOwned::to_owned) }).parse_next(input)?,
        "SetItemAttach" => seq!(AnimationActionData::SetItemAttach { item: i32, joint_name: string.map(ToOwned::to_owned) }).parse_next(input)?,
        "SpawnMissile" => seq!(AnimationActionData::SpawnMissile {
            weapon: i32,
            velocity: vec3,
            item_aligned: bool
        })
        .parse_next(input)?,
        "SpecialAbility" => todo!("parse character template special ability"),
        "Suicide" => seq!(AnimationActionData::Suicide { overkill: bool }).parse_next(input)?,
        "ThrowGrip" => AnimationActionData::ThrowGrip,
        "Tongue" => seq!(AnimationActionData::Tongue { max_length: f32 }).parse_next(input)?,
        "WeaponVisibility" => seq!(AnimationActionData::WeaponVisibility { weapon: i32, visible: bool }).parse_next(input)?,
        _t => winnow::combinator::fail
            .context(StrContext::Expected(StrContextValue::Description(
                "a valid character template animation type",
            )))
            .parse_next(input)?,
    };
    Ok(AnimationAction {
        start_time,
        end_time,
        data,
    })
}

fn event_storage(input: &mut Stream) -> Result<()> {
    let event_type = u8.parse_next(input)?;
    todo!("read event storage for event type {}", event_type)
}

#[derive(Debug, Clone)]
pub struct Ability {
    pub cooldown: f32,
    pub target: u8,
    pub fuzzy_expression: Option<String>,
    pub animation_keys: Vec<String>,
    pub data: AbilityData,
}
#[derive(Debug, Clone)]
pub enum AbilityData {
    Block {
        arc: f32,
        shield: i32,
    },
    CastSpell {
        min_range: f32,
        max_range: f32,
        arc: f32,
        chant_speed: f32,
        power: f32,
        cast_type: i32,
        elements: Vec<i32>,
    },
    ConfuseGrip,
    DamageGrip,
    Dash {
        min_range: f32,
        max_range: f32,
        arc: f32,
    },
    ElementSteal {
        range: f32,
        angle: f32,
    },
    GripCharacterFromBehind {
        max_range: f32,
        min_range: f32,
        angle: f32,
        max_weight: f32,
    },
    Jump {
        max_range: f32,
        min_range: f32,
        angle: f32,
        elevation: f32,
    },
    Melee {
        min_range: f32,
        max_range: f32,
        arc: f32,
        weapons: Vec<i32>,
        rotate: bool,
    },
    PickUpCharacter {
        max_range: f32,
        min_range: f32,
        angle: f32,
        max_weight: f32,
        drop_animation: String,
    },
    Ranged {
        min_range: f32,
        max_range: f32,
        elevation: f32,
        accuracy: f32,
        weapons: Vec<i32>,
    },
    RemoveStatus,
    SpecialAbilityAbility {
        max_range: f32,
        min_range: f32,
        angle: f32,
        weapon: i32,
    },
    ThrowGrip {
        max_range: f32,
        min_range: f32,
        elevation: f32,
        damages: Vec<(i32, i32, i32, f32)>,
    },
    ZombieGrip {
        max_range: f32,
        min_range: f32,
        angle: f32,
        max_weight: f32,
        drop_animation: String,
    },
}
fn ability(input: &mut Stream) -> Result<Ability> {
    let (type_name, cooldown, target, fuzzy_expression, animation_keys) = (
        string,
        f32,
        u8,
        bool.flat_map(|has| cond(has, string)),
        quicklist(string.map(ToOwned::to_owned)),
    )
        .parse_next(input)?;
    let data = match type_name {
        "Block" => seq!(AbilityData::Block {
            arc: f32,
            shield: i32,
        })
        .parse_next(input)?,
        "CastSpell" => seq!(AbilityData::CastSpell {
            min_range: f32,
            max_range: f32,
            arc: f32,
            chant_speed: f32,
            power: f32,
            cast_type: i32,
            elements: quicklist(i32),
        })
        .parse_next(input)?,
        "ConfuseGrip" => AbilityData::ConfuseGrip,
        "DamageGrip" => AbilityData::DamageGrip,
        "Dash" => seq!(AbilityData::Dash {
            min_range: f32,
            max_range: f32,
            arc: f32,
            _: vec3,
        })
        .parse_next(input)?,
        "ElementSteal" => seq!(AbilityData::ElementSteal {
            range: f32,
            angle: f32,
        })
        .parse_next(input)?,
        "GripCharacterFromBehind" => seq!(AbilityData::GripCharacterFromBehind {
            max_range: f32,
            min_range: f32,
            angle: f32,
            max_weight: f32,
        })
        .parse_next(input)?,
        "Jump" => seq!(AbilityData::Jump {
            max_range: f32,
            min_range: f32,
            angle: f32,
            elevation: f32,
        })
        .parse_next(input)?,
        "Melee" => seq!(AbilityData::Melee {
            min_range: f32,
            max_range: f32,
            arc: f32,
            weapons: quicklist(i32),
            rotate: bool
        })
        .parse_next(input)?,
        "PickUpCharacter" => seq!(AbilityData::PickUpCharacter {
            max_range: f32,
            min_range: f32,
            angle: f32,
            max_weight: f32,
            drop_animation: string.map(ToOwned::to_owned),
        })
        .parse_next(input)?,
        "Ranged" => seq!(AbilityData::Ranged {
            min_range: f32,
            max_range: f32,
            elevation: f32,
            _: f32,
            accuracy: f32,
            weapons: quicklist(i32),
        })
        .parse_next(input)?,
        "RemoveStatus" => AbilityData::RemoveStatus,
        "SpecialAbilityAbility" => seq!(AbilityData::SpecialAbilityAbility {
            max_range: f32,
            min_range: f32,
            angle: f32,
            weapon: i32,
        })
        .parse_next(input)?,
        "ThrowGrip" => seq!(AbilityData::ThrowGrip {
            max_range: f32,
            min_range: f32,
            elevation: f32,
            damages: length_repeat(
                i32.try_map(usize::try_from).map(|n| n.min(4)),
                (i32, i32, i32, f32),
            ),
        })
        .parse_next(input)?,
        "ZombieGrip" => seq!(AbilityData::ZombieGrip {
            max_range: f32,
            min_range: f32,
            angle: f32,
            max_weight: f32,
            drop_animation: string.map(ToOwned::to_owned),
        })
        .parse_next(input)?,
        _t => winnow::combinator::fail
            .context(StrContext::Expected(StrContextValue::Description(
                "a valid character template ability type",
            )))
            .parse_next(input)?,
    };
    Ok(Ability {
        cooldown,
        target,
        fuzzy_expression: fuzzy_expression.map(ToOwned::to_owned),
        animation_keys,
        data,
    })
}

pub fn buff(input: &mut Stream) -> Result<()> {
    todo!()
}

pub fn aura(input: &mut Stream) -> Result<()> {
    todo!()
}

#[cfg(test)]
mod tests {
    #[test]
    fn read_template() {
        let bytes = std::fs::read(
            "/data/SteamLibrary/steamapps/common/Magicka/Content/Data/Characters/Wizard_Purple.xnb",
        )
        .unwrap();
        let template = crate::parse_character(&bytes)
            .map_err(|e| e.into_inner())
            .unwrap();
        dbg!(template.inner());
    }
}
