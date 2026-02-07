use bevy::prelude::*;

use crate::spelling::{chanting::ElementQueue, element::Element};

pub fn plugin(app: &mut App) {
    app.add_observer(spawn_mote_source)
        .add_observer(despawn_mote_source);
    // TODO: Does apply_queue_to_motes need a manual apply deferred commands after it?
    app.add_systems(
        Update,
        (apply_queue_to_motes, (change_mote_light, move_mote)).chain(),
    );
}

#[derive(Component, Debug, Reflect)]
#[relationship(relationship_target = ElementQueueMoteSource)]
#[require(Transform, Visibility)]
struct ElementQueueMoteSourceOf(Entity);

#[derive(Component, Debug, Reflect)]
#[relationship_target(relationship = ElementQueueMoteSourceOf, linked_spawn)]
struct ElementQueueMoteSource(Entity);

#[derive(Component, Debug, Reflect)]
#[require(PointLight {
    shadows_enabled: true,
    ..default()
})]
struct QueuedElementMote {
    pub element: Element,
    pub magnitude: u8,
}

fn spawn_mote_source(event: On<Add, ElementQueue>, mut commands: Commands) {
    let chanter = event.entity;
    commands.queue(move |world: &mut World| {
        if !world.entity(chanter).contains::<ElementQueueMoteSource>() {
            world.spawn((ChildOf(chanter), ElementQueueMoteSourceOf(chanter)));
        }
    });
}

fn despawn_mote_source(event: On<Remove, ElementQueue>, mut commands: Commands) {
    let chanter = event.entity;
    commands.queue(move |world: &mut World| {
        if let Some(source_ref) = world.get::<ElementQueueMoteSource>(chanter) {
            world.despawn(source_ref.0);
        }
    });
}

fn apply_queue_to_motes(
    changed_queues: Query<(&ElementQueue, &ElementQueueMoteSource), Changed<ElementQueue>>,
    mote_sources: Query<Option<&Children>, With<ElementQueueMoteSourceOf>>,
    mut motes: Query<&mut QueuedElementMote>,
    mut commands: Commands,
    mut buffer_magnitudes: Local<std::collections::HashMap<Element, u8>>,
) {
    for (queue, source_ref) in changed_queues {
        let Ok(source_motes) = mote_sources.get(source_ref.0) else {
            continue;
        };

        // Determine how much there should be of each element
        // TODO: Optimize this?
        buffer_magnitudes.clear();
        for queued in queue.queued_elements.iter().copied() {
            let magnitude = buffer_magnitudes.entry(queued).or_default();
            *magnitude = magnitude.saturating_add(1);
        }

        // Remove motes no longer present in queue
        if let Some(source_motes) = source_motes {
            for mote_ent in source_motes {
                let Ok(mote) = motes.get(*mote_ent) else {
                    continue;
                };
                if !buffer_magnitudes.contains_key(&mote.element) {
                    commands.entity(*mote_ent).try_despawn();
                }
            }
        }

        // TODO: This should use the queue order, not hashmap order
        'elems: for (element, magnitude) in buffer_magnitudes.drain() {
            // Try to find an existing mote for the element
            if let Some(source_motes) = source_motes {
                for mote_ent in source_motes {
                    let Ok(mote) = motes.get_mut(*mote_ent) else {
                        continue;
                    };
                    if mote.element == element {
                        mote.map_unchanged(|m| &mut m.magnitude)
                            .set_if_neq(magnitude);
                        continue 'elems;
                    }
                }
            }
            // If there is none, spawn one
            commands
                .entity(source_ref.0)
                .with_child(QueuedElementMote { element, magnitude });
        }
    }
}

fn change_mote_light(
    motes: Query<(&mut PointLight, &QueuedElementMote), Changed<QueuedElementMote>>,
) {
    for (mut light, mote) in motes {
        let magnitude_fac = 0.2 * (mote.magnitude as f32);
        let (color, color_strength) = element_color(mote.element);
        light.color = color.into();
        light.intensity = magnitude_fac * color_strength * 100_000.0;
    }
}

fn move_mote(
    motes: Query<(Entity, &mut Transform, Option<&ChildOf>), With<QueuedElementMote>>,
    parents: Query<&Children>,
    time: Res<Time>,
) {
    let distance = 1.;
    let orbit_period = 3.;
    let bob_period = 2.;
    let bob_variance = 0.25;
    let index_phase_offset = orbit_period * (1. / 5.);
    for (mote, mut mote_trans, mote_parent) in motes {
        let index_in_parent = mote_parent
            .and_then(|p| parents.get(p.parent()).ok())
            .and_then(|parent_children| parent_children.iter().position(|p| p == mote))
            .unwrap_or(0);
        // TODO: Use .elapsed_wrapped() if possible
        let t = time.elapsed_secs() + index_phase_offset * (index_in_parent as f32);
        let angle = std::f32::consts::TAU * (t / orbit_period).fract();
        let height = bob_variance * (t / bob_period).cos();
        // Polar to cartesian
        let offset = (Vec2::from_angle(angle) * distance).extend(height).xzy();
        mote_trans.translation = offset;
    }
}

fn element_color(element: Element) -> (LinearRgba, f32) {
    use Element::*;
    let rgb = Vec3::from(match element {
        Water => (0., 0.7, 1.3),
        Life => (0.2, 1.6, 0.2),
        Shield => (2., 1.5, 1.),
        Cold => (1., 1., 1.4),
        Lightning => (0.75, 0.5, 1.),
        Arcane => (2., 0.4, 0.6),
        Earth => (0.3, 0.2, 0.1),
        Fire => (1.8, 0.6, 0.4),
        Steam => (1., 1., 1.),
        Ice => (0.8, 0.9, 1.4),
        Poison => (1., 1.2, 0.),
        Lok => (0.2, 0.3, 0.3),
    });
    let (rgb, magnitude) = rgb.normalize_and_length();
    (LinearRgba::from_vec3(rgb), magnitude)
}
