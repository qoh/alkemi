use crate::spelling::{
    chanting::ElementQueue,
    element::Magnitudes,
    input::{CastArea, CastForward, CastImbue, CastMagick, CastSelf, SpellingInput},
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

        // TODO: Determine what spell to cast based on the queued elements
        if elements.is_empty() {
            // M1 would push
            continue;
        }

        trace!("start spell cast with {:?}", elements);

        // XXX: Just start a beam spell for now for testing
        let spell = commands
            .spawn((
                ChildOf(caster_entity),
                Spell {
                    caster: caster_entity,
                },
                spells::beam::beam_spell(caster_entity, elements),
                Transform::from_rotation(Quat::from_rotation_y(0.3)),
            ))
            .id();

        caster.state = CasterState::Holding { cast_type, spell };
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
