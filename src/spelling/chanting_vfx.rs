use bevy::{mesh::SphereMeshBuilder, prelude::*};

use crate::spelling::{
    chanting::ElementQueue,
    color::{element_color, normalize_color},
    element::Element,
};

pub fn plugin(app: &mut App) {
    app.add_observer(spawn_mote_source)
        .add_observer(despawn_mote_source);
    // TODO: Does apply_queue_to_motes need a manual apply deferred commands after it?
    app.add_systems(
        Update,
        (
            apply_queue_to_motes,
            (change_mote_light, change_mote_scale, move_mote),
        )
            .chain(),
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
    assets: Res<AssetServer>,
    mut commands: Commands,
    mut buffer_magnitudes: Local<std::collections::HashMap<Element, u8>>,
) {
    for (queue, source_ref) in changed_queues {
        let Ok(source_motes) = mote_sources.get(source_ref.0) else {
            continue;
        };

        // Determine how much there should be of each element
        // TODO: Optimize this? It also duplicates elements::Magnitudes
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
            let new_mote = QueuedElementMote { element, magnitude };
            commands
                .entity(source_ref.0)
                .with_child(mote_bundle(new_mote, &assets));
        }
    }
}

// XXX: The mesh/material does not update if QueuedElementMote.element changes
fn mote_bundle(init: QueuedElementMote, assets: &AssetServer) -> impl Bundle {
    let mesh: Mesh = match init.element {
        Element::Earth => Sphere::new(0.15).mesh().uv(5, 3),
        Element::Ice => Sphere::new(0.15).mesh().uv(4, 2),
        _ => Sphere::new(0.15).mesh().ico(2).unwrap(),
    };
    let color = element_color(init.element);
    let is_emissive = !matches!(init.element, Element::Earth | Element::Steam | Element::Ice);
    let color = match init.element {
        Element::Lightning => color * 1.5,
        Element::Ice => color.lighter(0.2),
        _ => color,
    };
    let mat = StandardMaterial {
        base_color: if is_emissive {
            Color::BLACK
        } else {
            color.into()
        },
        emissive: if is_emissive {
            color
        } else {
            LinearRgba::BLACK
        },
        emissive_exposure_weight: match init.element {
            Element::Lightning => -30.,
            Element::Arcane => -20.,
            Element::Life | Element::Fire => -10.,
            _ => -5.,
        },
        perceptual_roughness: match init.element {
            Element::Earth => 0.97,
            Element::Water => 0.2,
            _ => 0.5,
        },
        alpha_mode: match init.element {
            Element::Water | Element::Steam | Element::Poison => AlphaMode::Add,
            _ => AlphaMode::default(),
        },
        ..default()
    };
    (
        Mesh3d(assets.add(mesh)),
        MeshMaterial3d(assets.add(mat)),
        init,
    )
}

fn change_mote_light(
    motes: Query<(&mut PointLight, &QueuedElementMote), Changed<QueuedElementMote>>,
) {
    for (mut light, mote) in motes {
        let magnitude_fac = 0.2 * (mote.magnitude as f32);
        let (color, color_strength) = normalize_color(element_color(mote.element));
        light.color = color.into();
        light.intensity = magnitude_fac * color_strength * 100_000.0;
    }
}

fn change_mote_scale(
    motes: Query<(&mut Transform, &QueuedElementMote), Changed<QueuedElementMote>>,
) {
    for (mut trans, mote) in motes {
        let scale = match mote.element {
            Element::Shield => 1., // Shield is its own opposite, so won't have meaningful magnitude
            _ => 0.6 + 0.2 * (mote.magnitude as f32),
        };
        trans.scale = Vec3::splat(scale);
    }
}

fn move_mote(
    motes: Query<(Entity, &mut Transform, Option<&ChildOf>), With<QueuedElementMote>>,
    parents: Query<&Children>,
    time: Res<Time>,
) {
    let distance = 1.;
    let orbit_period = 3.;
    let bob_period = 1.;
    let bob_variance = 0.3;
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
