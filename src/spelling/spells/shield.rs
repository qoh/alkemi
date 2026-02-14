use std::time::Duration;

use avian3d::prelude::{
    Collider, CollisionEventsEnabled, CollisionLayers, CollisionStart, Collisions, LayerMask,
    PhysicsLayer, RigidBody,
};
pub use bevy::prelude::*;
use bevy::{
    ecs::{lifecycle::HookContext, world::DeferredWorld},
    light::NotShadowCaster,
};

use crate::{
    gameplay::{
        damage::{Damage, DamagePayload, DamageType, Die, Health},
        damage_over_time::{self, DamageOverTime},
    },
    magicka_level_model::Layers,
    spelling::element::Element,
};

const HEALTH_MAGICKA1_FORWARD: f32 = 800.;
const HEALTH_MAGICKA1_AREA: f32 = 1000.;
const HEALTH_MAGICKA2_FORWARD_PER_ELEMENT: f32 = 2000.;
const HEALTH_MAGICKA2_AREA_PER_ELEMENT: f32 = 2000.;
const HEALTH_MAGICKA2_LINE_PER_ELEMENT: f32 = 1000.;
const HEALTH_MAGICKA2_SELF_PER_ELEMENT: f32 = 1500.;

const DECAY_MAGICKA2_AMOUNT_PER_RATE: f32 = 25.;
const DECAY_MAGICKA2_RATE: f32 = 0.1;

const SPHERE_DIAMETER_MAGICKA1: f32 = 5.5;
const DIAMETER: f32 = SPHERE_DIAMETER_MAGICKA1;

// TODO: Determine what to set for WALL_LENGTH and WALL_HEIGHT
const WALL_LENGTH: f32 = 5.5;
const WALL_HEIGHT: f32 = 2.25;

pub fn plugin(app: &mut App) {
    app.add_observer(despawn_dead_shield);
    app.add_observer(collapse_intersecting_shields);

    app.add_observer(spawn_collapse_effect);
    app.add_systems(Update, fade_lights);

    let world = app.world_mut();

    let mesh_sphere = world
        .resource_mut::<Assets<Mesh>>()
        .add(Sphere::new(DIAMETER / 2.).mesh().ico(7).unwrap());
    let mesh_wall =
        world
            .resource_mut::<Assets<Mesh>>()
            .add(Cuboid::new(WALL_LENGTH, WALL_HEIGHT, 0.05));

    let mut color = crate::spelling::color::element_color(Element::Shield);
    color.blue *= 0.5;
    let material = world
        .resource_mut::<Assets<StandardMaterial>>()
        .add(StandardMaterial {
            base_color: color.with_alpha(0.076).into(),
            emissive: color,
            emissive_exposure_weight: -100.,
            perceptual_roughness: 1.,
            metallic: 1.,
            clearcoat: 1.,
            clearcoat_perceptual_roughness: 1.,
            alpha_mode: AlphaMode::Blend,
            double_sided: true,
            cull_mode: None,
            ..default()
        });

    world.insert_resource(ShieldAssets {
        mesh_sphere,
        mesh_wall,
        material,
    });
}

#[derive(Resource, Reflect)]
pub struct ShieldAssets {
    mesh_sphere: Handle<Mesh>,
    // mesh_segment: Handle<Mesh>,
    mesh_wall: Handle<Mesh>,
    material: Handle<StandardMaterial>,
}

pub fn shield_area() -> impl Bundle {
    (
        Shield {
            shape: Shape::Sphere,
        },
        RigidBody::Kinematic,
        CollisionLayers::new(
            Layers::Shield,
            LayerMask::ALL & !(Layers::Trigger.to_bits()),
        ),
        Collider::sphere(DIAMETER / 2.),
        health_and_decay(HEALTH_MAGICKA1_AREA),
    )
}

pub fn shield_wall() -> impl Bundle {
    (
        Shield { shape: Shape::Wall },
        RigidBody::Kinematic,
        CollisionLayers::new(
            Layers::Shield,
            LayerMask::ALL & !(Layers::Trigger.to_bits()),
        ),
        Collider::cuboid(WALL_LENGTH, WALL_HEIGHT, 0.05),
        health_and_decay(
            HEALTH_MAGICKA2_LINE_PER_ELEMENT
                * 1.
                * (HEALTH_MAGICKA1_AREA / HEALTH_MAGICKA2_AREA_PER_ELEMENT),
        ),
    )
}

fn health_and_decay(health: f32) -> impl Bundle {
    (
        Health::full(health),
        DamageOverTime::every(
            Duration::from_secs_f32(DECAY_MAGICKA2_RATE),
            damage_over_time::Target::This,
            DamagePayload {
                damage_type: DamageType::True,
                amount: DECAY_MAGICKA2_AMOUNT_PER_RATE * DECAY_MAGICKA2_RATE,
                source: None,
                silent: true,
            },
        ),
        CollisionEventsEnabled,
    )
}

#[derive(Component)]
#[component(on_add = Self::on_add)]
pub struct Shield {
    shape: Shape,
}

#[derive(Debug, Clone, Copy)]
enum Shape {
    Section,
    Sphere,
    Wall,
}

#[derive(Event, Debug)]
pub struct ShieldCollapse {
    shield1: Entity,
    shield2: Entity,
    collider1: Entity,
    collider2: Entity,
}

fn collapse_intersecting_shields(
    collision: On<CollisionStart>,
    shields: Query<Option<&Health>, With<Shield>>,
    mut commands: Commands,
) {
    let CollisionStart {
        body1: Some(body1),
        body2: Some(body2),
        ..
    } = *collision.event()
    else {
        return;
    };

    // Ensure only one shield collapse occurs per contact pair regardless of CollisionStart order
    if body1.index() >= body2.index() {
        return;
    }

    let Ok(health1) = shields.get(body1) else {
        return;
    };
    let Ok(health2) = shields.get(body2) else {
        return;
    };

    let alive1 = health1.is_some_and(|h| h.alive());
    let alive2 = health2.is_some_and(|h| h.alive());

    if alive1 || alive2 {
        commands.trigger(ShieldCollapse {
            shield1: body1,
            shield2: body2,
            collider1: collision.collider1,
            collider2: collision.collider2,
        });
    }

    if alive1 {
        commands.trigger(Damage {
            recipient: body1,
            damage: DamagePayload {
                damage_type: DamageType::True,
                amount: 10_000.,
                source: Some(body2),
                silent: false,
            },
        });
    }

    if alive2 {
        commands.trigger(Damage {
            recipient: body2,
            damage: DamagePayload {
                damage_type: DamageType::True,
                amount: 10_000.,
                source: Some(body1),
                silent: false,
            },
        });
    }
}

fn despawn_dead_shield(death: On<Die>, shields: Query<(), With<Shield>>, mut commands: Commands) {
    if shields.contains(death.subject) {
        commands.entity(death.subject).try_despawn();
    }
}

// VFX

impl Shield {
    fn on_add(mut deferred_world: DeferredWorld, ctx: HookContext) {
        let shape = deferred_world
            .get::<Shield>(ctx.entity)
            .expect("Shield component to exist in Shield::on_add")
            .shape;
        let assets = deferred_world.resource::<ShieldAssets>();
        let mesh = match shape {
            Shape::Section => todo!(),
            Shape::Sphere => &assets.mesh_sphere,
            Shape::Wall => &assets.mesh_wall,
        };
        let (color, _) = crate::spelling::color::normalize_color(
            crate::spelling::color::element_color(Element::Shield),
        );
        let bundle = (
            Mesh3d(mesh.clone()),
            MeshMaterial3d(assets.material.clone()),
            NotShadowCaster,
            Children::spawn_one((
                Transform::from_translation(Vec3::Y * 0.25 * DIAMETER),
                PointLight {
                    color: color.into(),
                    intensity: 0.25 * light_consts::lumens::VERY_LARGE_CINEMA_LIGHT,
                    radius: DIAMETER / 2. / 2.,
                    // Perf: Would like to cast shadows, but it's too laggy with many shields until we use deferred rendering
                    //shadows_enabled: true,
                    ..default()
                },
            )),
        );
        deferred_world
            .commands()
            .spawn((ChildOf(ctx.entity), bundle));
    }
}

fn spawn_collapse_effect(
    event: On<ShieldCollapse>,
    collisions: Collisions,
    transform_helper: TransformHelper,
    mut commands: Commands,
) {
    // Try to use the actual collision point as the place to spawn the effect
    let epicenter = if let Some(contact) = collisions
        .get(event.collider1, event.collider2)
        // FIXME: This seems to be in the wrong place when spawning line shields onto sphere shields
        .and_then(|c| c.find_deepest_contact())
        // Disabled for now
        && false
    {
        contact.point
    } else {
        match (
            transform_helper.compute_global_transform(event.collider1),
            transform_helper.compute_global_transform(event.collider2),
        ) {
            (Ok(transform1), Ok(transform2)) => {
                info!(
                    "Shield collapse effect can't access collision, falling back to average translation"
                );
                (transform1.translation() + transform2.translation()) / 2.
            }
            (r1, r2) => {
                warn!(
                    "Failed to spawn shield collapse effect: epicenter transform not available\n  first: {r1:?}\n  second: {r2:?}"
                );
                return;
            }
        }
    };

    commands.spawn((
        Transform::from_translation(epicenter + 0.8 * Vec3::Y),
        PointLight {
            color: LinearRgba::rgb(1., 0.9, 0.7).into(),
            intensity: 0.,
            shadows_enabled: true,
            ..default()
        },
        FadePointLight {
            curve: std::sync::Arc::new(SmoothStepCurve.reverse().unwrap().map(|y| y * 10_000_000.)),
            timer: Timer::from_seconds(0.4, TimerMode::Once),
        },
    ));
}

#[derive(Component)]
#[require(PointLight)]
struct FadePointLight {
    pub curve: std::sync::Arc<dyn Curve<f32> + Send + Sync>,
    pub timer: Timer,
}

fn fade_lights(
    faders: Query<(Entity, &mut FadePointLight, &mut PointLight)>,
    time: Res<Time>,
    mut commands: Commands,
) {
    for (entity, mut fader, mut light) in faders {
        fader.timer.tick(time.delta());
        if fader.timer.is_finished() {
            commands.entity(entity).try_despawn();
        } else {
            let t = fader.timer.elapsed_secs() / fader.timer.duration().as_secs_f32();
            light.intensity = fader.curve.sample_clamped(t);
        }
    }
}
