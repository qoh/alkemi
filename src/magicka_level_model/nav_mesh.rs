use std::sync::Arc;

use crate::magicka_level_model::map_vec3;
use bevy::{ecs::relationship::RelatedSpawnerCommands, prelude::*};
use bevy_landmass::{
    Archipelago3d, ArchipelagoOptions, ArchipelagoRef3d, FromAgentRadius as _, Island,
    Island3dBundle, Landmass3dPlugin, NavMesh3d, NavMeshHandle, NavigationMesh3d,
    PointSampleDistance3d, ValidationError, Velocity3d, coords::ThreeD,
};
use remagic::xnb_readers::magicka_content::{NavMesh as MagickaNavMesh, PathFindingTriangle};

pub fn plugin(app: &mut App) {
    app.add_plugins(Landmass3dPlugin::default());
    app.add_systems(
        FixedPostUpdate,
        propagate_physics_velocities_to_nav.after(avian3d::prelude::PhysicsSystems::Last),
    );

    #[cfg(feature = "dev")]
    {
        use bevy::input::common_conditions::input_just_pressed;
        use bevy_landmass::debug::{EnableLandmassDebug, Landmass3dDebugPlugin};

        app.add_plugins(Landmass3dDebugPlugin {
            draw_on_start: false,
            ..default()
        });
        app.insert_gizmo_config(
            bevy_landmass::debug::LandmassGizmos::default(),
            GizmoConfig {
                depth_bias: 0.,
                ..default()
            },
        );
        app.add_systems(
            PostUpdate,
            toggle_debug.run_if(input_just_pressed(KeyCode::F6)),
        );
        fn toggle_debug(mut debug: ResMut<EnableLandmassDebug>) {
            debug.0 ^= true;
        }
    }
}

#[derive(bevy::ecs::system::SystemParam)]
pub struct SpawnParams<'w> {
    assets: ResMut<'w, Assets<NavMesh3d>>,
}

pub struct NavMeshSetup {
    archipelago: Entity,
}

pub fn setup_for_level(level: &mut RelatedSpawnerCommands<ChildOf>) -> NavMeshSetup {
    let archipelago = level
        .spawn((
            Name::new("Navigation Mesh Archipelago"),
            Archipelago3d::new({
                let (length, radius, _mass, _speed, _turn_speed) = (0.5, 0.75, 70.0, 5.0, 7.0);
                let total_height = length + radius * 2.;
                let agent_radius = total_height * 0.5;
                let mut opts = ArchipelagoOptions::<ThreeD>::from_agent_radius(agent_radius);
                opts.point_sample_distance.horizontal_distance *= 50.;
                opts.point_sample_distance.distance_below *= 10.;
                opts.point_sample_distance.distance_above *= 20.;
                opts.point_sample_distance.vertical_preference_ratio = 8.0;
                opts
            }),
        ))
        .id();
    NavMeshSetup { archipelago }
}

pub fn spawn(
    parent: &mut RelatedSpawnerCommands<ChildOf>,
    source_nav_mesh: &MagickaNavMesh,
    params: &mut SpawnParams,
    setup: &NavMeshSetup,
) {
    let bevy_nav_mesh = match convert_nav_mesh(source_nav_mesh) {
        Ok(x) => x,
        Err(e) => {
            error!("Failed to create level navigation mesh: {}", e);
            return;
        }
    };

    let archipelago_id = setup.archipelago;

    let nav_mesh_handle = params.assets.add(bevy_nav_mesh);

    parent.spawn((
        Name::new("Navigation Mesh Island"),
        // TODO: Remove this hack, configure agent sampling better?
        Transform::from_translation(Vec3::Y * -0.6), // Magicka nav mesh is floating above ground
        Island3dBundle {
            island: Island,
            archipelago_ref: ArchipelagoRef3d::new(archipelago_id),
            nav_mesh: NavMeshHandle(nav_mesh_handle.clone()),
        },
    ));
}

fn propagate_physics_velocities_to_nav(
    q: Query<(&avian3d::prelude::LinearVelocity, &mut Velocity3d)>,
) {
    for (phys_vel, mut nav_vel) in q {
        nav_vel.velocity = **phys_vel;
    }
}

fn convert_nav_mesh(magicka: &MagickaNavMesh) -> Result<NavMesh3d, ValidationError> {
    let nav_mesh = NavigationMesh3d {
        vertices: magicka.vertices.iter().copied().map(map_vec3).collect(),
        polygons: magicka
            .triangles
            .iter()
            .map(
                |PathFindingTriangle {
                     vertices,
                     neighbors,
                     costs: _,
                     properties: _,
                 }| {
                    vec![
                        usize::from(vertices.0),
                        usize::from(vertices.1),
                        usize::from(vertices.2),
                    ]
                },
            )
            .collect(),
        polygon_type_indices: magicka
            .triangles
            .iter()
            .map(
                |PathFindingTriangle {
                     vertices: _,
                     neighbors: _,
                     costs,
                     properties,
                 }| { 0 },
            )
            .collect(),
        height_mesh: None,
    };

    nav_mesh.validate().map(|nav_mesh| NavMesh3d {
        nav_mesh: Arc::new(nav_mesh),
    })
}
