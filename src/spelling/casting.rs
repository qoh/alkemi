use crate::spelling::{
    chanting::ElementQueue,
    element::{self, BaseElement, Element, HybridElement, Magnitudes, Reaction},
    input::{self, CastForward},
    spells::beam::Beam,
};

use bevy::prelude::*;
use bevy_enhanced_input::prelude::Start;

pub fn plugin(app: &mut App) {
    app.add_observer(spawn_test_beam);
    app.add_systems(Update, timeout_beams);
}

fn spawn_test_beam(
    event: On<Start<CastForward>>,
    mut casters: Query<&mut ElementQueue>,
    mut commands: Commands,
) {
    let Ok(mut element_queue) = casters.get_mut(event.context) else {
        return;
    };

    if element_queue.queued_elements.is_empty() {
        // M1 would push
        return;
    }
    let elements: Magnitudes = element_queue.queued_elements.drain(..).collect();

    const LIFETIME_BASE: f32 = 1.;
    const LIFETIME_ELEM: f32 = 2.;

    let num_beam_elems =
        elements.get(Element::Life) as u16 + (elements.get(Element::Arcane) as u16);
    let lifetime = LIFETIME_BASE + LIFETIME_ELEM * (num_beam_elems as f32);

    commands.entity(event.context).with_child((
        Beam {
            elements,
            ignore_entity: Some(event.context),
            ..default()
        },
        Transform::from_rotation(Quat::from_rotation_y(-0.3)),
        BeamLifetime {
            remaining: lifetime,
        },
    ));
}

#[derive(Component, Debug, Reflect)]
struct BeamLifetime {
    pub remaining: f32,
}

fn timeout_beams(
    timers: Query<(Entity, &mut BeamLifetime)>,
    time: Res<Time>,
    mut commands: Commands,
) {
    for (ent, mut timer) in timers {
        timer.remaining -= time.delta_secs();
        if timer.remaining <= 0. {
            commands.trigger(super::spells::beam::Stop(ent));
        }
    }
}
