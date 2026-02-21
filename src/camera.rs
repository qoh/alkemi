use crate::PlayerControlled;
use crate::magicka_level_model::CameraMesh;
use bevy::{
    camera::RenderTarget,
    core_pipeline::tonemapping::Tonemapping,
    input::InputSystems,
    post_process::bloom::Bloom,
    prelude::*,
    render::view::Hdr,
    window::{PrimaryWindow, WindowRef},
};
use bevy_seedling::prelude::*;

pub fn plugin(app: &mut App) {
    app.add_systems(Startup, spawn_camera)
        .add_systems(PostUpdate, camera_follow_group);
    app.add_systems(PreUpdate, project_pointer_camera_rays.after(InputSystems));
    app.add_plugins(free_camera::plugin);
}

#[derive(Component, Reflect)]
pub struct CameraGroupFollower {
    pub position: Vec3,
    pub magnify: f32,
}

/// A camera with [`CameraGroupFollower`] includes this entity in its view.
#[derive(Component, Default)]
pub struct CameraGroupMember;

/// The window cursor projected into the world with this camera at the start of the frame.
#[derive(Component, Default, Debug)]
pub struct PointerRay {
    pub current: Option<Ray3d>,
}

/// This camera is a primary view into the world and will be swapped out for different camera modes.
#[derive(Component, Default, Debug, Clone, Copy)]
pub struct PrimaryView;

pub(crate) fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Name::new("Camera"),
        world_view_camera(),
        SpatialListener3D,
        Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Dir3::Y),
        CameraGroupFollower {
            position: default(),
            magnify: 1.,
        },
        PointerRay::default(),
    ));
}

fn world_view_camera() -> impl Bundle {
    (
        Camera3d::default(),
        Hdr,
        Tonemapping::None, // Magicka would not
        bevy::camera::Exposure {
            ev100: crate::magicka_level_model::light::MAGICKA_EXPOSURE,
        },
        Bloom::default(),
        PrimaryView,
    )
}

pub fn project_pointer_camera_rays(
    cameras: Query<(&mut PointerRay, &Camera, &RenderTarget, &GlobalTransform)>,
    windows: Query<&Window>,
    primary_windows: Query<&Window, With<PrimaryWindow>>,
) {
    for (pointer_ray, camera, render_target, camera_trans) in cameras {
        let window = match *render_target {
            RenderTarget::Window(WindowRef::Primary) => primary_windows.single().ok(),
            RenderTarget::Window(WindowRef::Entity(window)) => windows.get(window).ok(),
            _ => None,
        };

        let maybe_ray = window
            .and_then(|w| w.cursor_position())
            .and_then(|cursor| camera.viewport_to_world(camera_trans, cursor).ok());

        pointer_ray
            .map_unchanged(|r| &mut r.current)
            .set_if_neq(maybe_ray);
    }
}

fn camera_follow_group(
    camera: Single<
        (&mut Transform, &mut Projection, &mut CameraGroupFollower),
        Without<PlayerControlled>,
    >,
    characters: Query<&Transform, (With<CameraGroupMember>, Without<CameraGroupFollower>)>,
    camera_mesh: Option<Single<&CameraMesh>>,
    time: Res<Time>,
) {
    let (mut camera_transform, mut camera_projection, mut camera_state) = camera.into_inner();
    let players_center = {
        let mut sum = Vec3::ZERO;
        let mut count = 0;
        let mut sum_y = 0.;
        let mut count_y = 0;
        for transform in characters {
            let is_dead = false;
            if is_dead {
                continue;
            }
            sum += transform.translation;
            count += 1;
            let is_touching_ground_and_not_leaving_ground = true;
            if is_touching_ground_and_not_leaving_ground {
                sum_y += transform.translation.y; // TODO: - (capsule.length * 0.5 + capsule.radius)
                count_y += 1;
            }
        }
        if count != 0 {
            sum /= count as f32;
            if sum_y.abs() > 0.000000001 {
                sum.y = sum_y / (count_y as f32);
            }
            Some(sum)
        } else {
            None
        }
    };
    let mut target_position = players_center
        .map(|c| {
            let influence_vector = Vec3::ZERO; // TODO
            let bias = Vec3::ZERO; // TODO
            c + influence_vector + bias
        })
        .unwrap_or(camera_state.position);
    if let Some(camera_mesh) = camera_mesh {
        let (projected_point, _is_inside) =
            camera_mesh
                .collider
                .project_point(Vec3::ZERO, Quat::IDENTITY, target_position, false);
        target_position = projected_point;
    }
    let mut interpolated_position = camera_state.position;
    interpolated_position.smooth_nudge(&target_position, f32::ln(60.), time.delta_secs());
    let velocity = interpolated_position - camera_state.position;
    camera_state.position += velocity;
    // TODO: Determine target_magnification and interpolate camera_state.magnify towards it

    const CAMERAOFFSET: Vec3 = vec3(0., 144., 171.);
    const DEFAULTFOV: f32 = 5.0f32.to_radians();

    let position = camera_state.position + CAMERAOFFSET;
    let fov = DEFAULTFOV / camera_state.magnify;

    *camera_transform = Transform::from_translation(position);
    camera_transform.rotate_x(-(40.0f32.to_radians()));
    *camera_projection = Projection::Perspective(PerspectiveProjection { fov, ..default() });
}

mod free_camera {
    use crate::camera::PrimaryView;

    use super::world_view_camera;
    use bevy::{
        camera_controller::free_camera::{FreeCamera, FreeCameraPlugin, FreeCameraState},
        prelude::*,
    };
    use bevy_enhanced_input::prelude::*;

    pub fn plugin(app: &mut App) {
        app.add_plugins(FreeCameraPlugin);
        app.add_input_context::<FreeCameraInput>();
        app.add_systems(Startup, spawn_free_camera);
        app.add_observer(toggle_free_camera);
    }

    fn spawn_free_camera(mut commands: Commands) {
        commands.spawn((
            Name::new("Free Camera"),
            Camera {
                is_active: false,
                ..default()
            },
            world_view_camera(),
            FreeCamera::default(),
            {
                let mut state = FreeCameraState::default();
                state.enabled = false;
                state
            },
            FreeCameraInput,
            actions!(FreeCameraInput[
                (
                    Action::<ToggleFreeCamera>::new(),
                    bindings![KeyCode::KeyP],
                ),
            ]),
        ));
    }

    fn toggle_free_camera(
        event: On<Start<ToggleFreeCamera>>,
        mut cameras: Query<
            (Entity, &mut Camera, &mut Transform, Option<&mut Projection>),
            With<PrimaryView>,
        >,
        mut free_cameras: Query<(Entity, &mut FreeCameraState), With<PrimaryView>>,
    ) {
        let Ok((free_camera, mut free_camera_state)) = free_cameras.get_mut(event.context) else {
            warn_once!("ToggleFreeCamera input action triggered on non-free camera entity");
            return;
        };

        free_camera_state.enabled ^= true;

        let mut starting_view = None;

        for (camera_entity, mut camera, camera_transform, camera_projection) in &mut cameras {
            let is_free_camera = camera_entity == free_camera;
            camera.is_active = free_camera_state.enabled == is_free_camera;
            if !is_free_camera {
                starting_view.get_or_insert_with(|| {
                    (*camera_transform, camera_projection.as_deref().cloned())
                });
            }
        }

        // Start free camera at first non-free camera location
        if free_camera_state.enabled
            && let Some((source_trans, source_proj)) = starting_view
            && let Ok((_, _, mut target_trans, target_proj)) = cameras.get_mut(free_camera)
        {
            *target_trans = source_trans;
            // Apply dolly zoom to preserve visible width
            if let (Some(Projection::Perspective(source_persp)), Some(mut target_proj)) =
                (source_proj, target_proj)
                && let Projection::Perspective(target_persp) = &mut *target_proj
            {
                let source_distance = vec3(0., 144., 171.).length();
                let source_size_at_distance = 2. * (source_persp.fov / 2.) * source_distance;
                let target_distance =
                    source_size_at_distance / (2. * ((1. / 2.) * target_persp.fov).tan());
                let displacement = target_trans.forward() * (source_distance - target_distance);
                target_trans.translation += displacement;
            }
        }
    }

    #[derive(Component, Default)]
    struct FreeCameraInput;

    #[derive(InputAction)]
    #[action_output(bool)]
    struct ToggleFreeCamera;
}
