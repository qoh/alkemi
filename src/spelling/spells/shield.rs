use avian3d::prelude::{Collider, CollisionLayers, LayerMask, PhysicsLayer, RigidBody};
pub use bevy::prelude::*;
use bevy::{
    ecs::{lifecycle::HookContext, world::DeferredWorld},
    light::NotShadowCaster,
};

use crate::{magicka_level_model::Layers, spelling::element::Element};

const DIAMETER: f32 = 5.5;

// TODO: Determine what to set for WALL_LENGTH and WALL_HEIGHT
const WALL_LENGTH: f32 = 5.5;
const WALL_HEIGHT: f32 = 2.25;

pub fn plugin(app: &mut App) {
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
        RigidBody::Static,
        CollisionLayers::new(
            Layers::Shield,
            LayerMask::ALL & !(Layers::Trigger.to_bits()),
        ),
        Collider::sphere(DIAMETER / 2.),
    )
}

pub fn shield_wall() -> impl Bundle {
    (
        Shield { shape: Shape::Wall },
        RigidBody::Static,
        CollisionLayers::new(
            Layers::Shield,
            LayerMask::ALL & !(Layers::Trigger.to_bits()),
        ),
        Collider::cuboid(WALL_LENGTH, WALL_HEIGHT, 0.05),
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
