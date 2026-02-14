use crate::spelling::{
    chanting::ElementQueue,
    element::Magnitudes,
    input::{CastArea, CastForward, CastImbue, CastMagick, CastSelf, SpellingInput},
    spell_resolve,
    spells::{self, Spell},
};

use bevy::prelude::*;
use bevy_enhanced_input::action::{Action, events::Start, relationship::Actions};

// TODO: Order casting systems before spell systems

pub fn plugin(app: &mut App) {
    app.add_observer(queue_cast::<CastForward>)
        .add_observer(queue_cast::<CastArea>)
        .add_observer(queue_cast::<CastSelf>)
        .add_observer(queue_cast::<CastImbue>)
        .add_observer(queue_cast::<CastMagick>);

    app.add_systems(
        FixedUpdate,
        (idle_if_spell_missing, start_cast, release_current_cast).chain(),
    );
    app.add_observer(idle_on_complete);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub enum CastType {
    Forward,
    Area,
    SelfCast,
    Weapon,
    Magick,
}

#[derive(Component, Debug, Default, Reflect)]
pub struct Caster {
    state: CasterState,
    next: Option<CastType>,
}

#[derive(Debug, Default, Reflect)]
enum CasterState {
    #[default]
    Idle,
    Holding {
        cast_type: CastType,
        spell: Entity,
    },
    Released {
        spell: Entity,
    },
}

fn start_cast(
    casters: Query<(Entity, &mut Caster, &mut ElementQueue), Changed<Caster>>,
    mut commands: Commands,
) {
    for (caster_entity, mut caster, mut element_queue) in casters {
        if !matches!(&caster.state, CasterState::Idle) {
            continue;
        }
        let Some(cast_type) = caster.next else {
            continue;
        };
        caster.next = None;

        let elements: Magnitudes = element_queue.queued_elements.drain(..).collect();

        trace!("start cast {cast_type:?} with {:?}", elements);

        let spell = match cast_type {
            CastType::Forward => spell_resolve::spell_forward(&elements),
            CastType::Area => spell_resolve::spell_area(&elements),
            CastType::SelfCast => {
                let Some(spell) = spell_resolve::spell_self(&elements) else {
                    return; // ???
                };
                spell
            }
            CastType::Weapon => {
                // If non-empty just do spell_resolve::Spell::Imbue
                // Otherwise read elements from weapon
                let Some(spell) = spell_resolve::spell_weapon(&elements) else {
                    return; // ???
                };
                spell
            }
            CastType::Magick => {
                let magick = {
                    // TODO: Resolve magick
                    return;
                };
                spell_resolve::Spell::Magick(magick)
            }
        };

        trace!("spell is {spell:?}");

        // TODO: Design a way to not define all the spell inits in one place
        match spell {
            spell_resolve::Spell::Beam => {
                let spell = commands
                    .spawn((
                        ChildOf(caster_entity),
                        Spell {
                            caster: caster_entity,
                        },
                        spells::beam::beam_spell(caster_entity, elements),
                    ))
                    .id();

                caster.state = CasterState::Holding { cast_type, spell };
            }
            spell_resolve::Spell::Spray => {
                let spell = commands
                    .spawn((
                        ChildOf(caster_entity),
                        Spell {
                            caster: caster_entity,
                        },
                        spells::spray::spray_spell(caster_entity, elements),
                    ))
                    .id();

                caster.state = CasterState::Holding { cast_type, spell };
            }
            spell_resolve::Spell::Shield(spell_resolve::RegionWithWeapon::Circle) => {
                // TODO: Cooldown etc
                commands
                    .spawn((
                        spells::shield::shield_area(),
                        Transform::from_translation(Vec3::Y * -1.),
                        ChildOf(caster_entity),
                    ))
                    .remove_parent_in_place();
            }
            spell_resolve::Spell::Shield(spell_resolve::RegionWithWeapon::Line) => {
                // TODO: Cooldown etc
                commands
                    .spawn((
                        spells::shield::shield_wall(),
                        Transform::from_translation(vec3(0., 0.25, -3.33)),
                        ChildOf(caster_entity),
                    ))
                    .remove_parent_in_place();
            }
            _ => {
                warn!("unimplemented spell: {spell:?}");
            }
        }
    }
}

// TODO: In Magicka 2, _if_ the queued cast type would produce a valid spell, it should interrupt the current channel, even if the button is still held

fn release_current_cast(
    casters: Query<(&mut Caster, Option<&Actions<SpellingInput>>)>,
    actions_forward: Query<&Action<CastForward>>,
    actions_area: Query<&Action<CastArea>>,
    actions_self: Query<&Action<CastSelf>>,
    actions_weapon: Query<&Action<CastImbue>>,
    actions_magick: Query<&Action<CastMagick>>,
    mut commands: Commands,
) {
    for (mut caster, actions) in casters {
        let CasterState::Holding { cast_type, spell } = &caster.state else {
            continue;
        };
        let holding = actions
            .and_then(|actions| match cast_type {
                CastType::Forward => actions_forward.iter_many(actions).next().map(|a| **a),
                CastType::Area => actions_area.iter_many(actions).next().map(|a| **a),
                CastType::SelfCast => actions_self.iter_many(actions).next().map(|a| **a),
                CastType::Weapon => actions_weapon.iter_many(actions).next().map(|a| **a),
                CastType::Magick => actions_magick.iter_many(actions).next().map(|a| **a),
            })
            .unwrap_or_default();
        if !holding {
            commands.trigger(spells::Release { spell: *spell });
            caster.state = CasterState::Released { spell: *spell };
        }
    }
}

fn idle_on_complete(
    event: On<spells::Complete>,
    spells: Query<&Spell>,
    mut casters: Query<&mut Caster>,
) {
    let Ok(spell) = spells.get(event.spell) else {
        return;
    };
    let Ok(mut caster) = casters.get_mut(spell.caster) else {
        return;
    };
    if let CasterState::Holding {
        spell: caster_spell,
        ..
    }
    | CasterState::Released {
        spell: caster_spell,
        ..
    } = caster.state
        && event.spell == caster_spell
    {
        caster.state = CasterState::Idle;
    }
}

fn idle_if_spell_missing(casters: Query<&mut Caster>, spells: Query<&Spell>) {
    for mut caster in casters {
        if let CasterState::Holding { spell, .. } | CasterState::Released { spell, .. } =
            caster.state
            && !spells.contains(spell)
        {
            warn!("caster is casting spell but spell does not exist, returning to idle");
            caster.state = CasterState::Idle;
        }
    }
}

fn queue_cast<Action: CastAction + bevy_enhanced_input::prelude::InputAction>(
    event: On<Start<Action>>,
    mut casters: Query<&mut Caster>,
) {
    let Ok(mut caster) = casters.get_mut(event.context) else {
        return;
    };
    caster.next = Some(Action::TYPE);
}

trait CastAction: Sized {
    const TYPE: CastType;
}
impl CastAction for CastForward {
    const TYPE: CastType = CastType::Forward;
}
impl CastAction for CastArea {
    const TYPE: CastType = CastType::Area;
}
impl CastAction for CastSelf {
    const TYPE: CastType = CastType::SelfCast;
}
impl CastAction for CastImbue {
    const TYPE: CastType = CastType::Weapon;
}
impl CastAction for CastMagick {
    const TYPE: CastType = CastType::Magick;
}
