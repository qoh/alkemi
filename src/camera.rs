use crate::PlayerControlled;
use crate::magicka_level_model::CameraMesh;
use bevy::{
    core_pipeline::tonemapping::Tonemapping,
    input::mouse::AccumulatedMouseMotion,
    post_process::bloom::Bloom,
    prelude::*,
    render::view::Hdr,
    window::{CursorGrabMode, CursorOptions},
};
use bevy_seedling::prelude::*;
use std::f32::consts::FRAC_PI_2;

pub fn plugin(app: &mut App) {
    app.add_systems(Startup, spawn_camera)
        .add_systems(Update, (rotate_free_camera, move_free_camera).chain())
        .add_systems(PostUpdate, camera_follow_group);
}

#[derive(Component, Reflect)]
pub struct CameraGroupFollower {
    pub position: Vec3,
    pub magnify: f32,
}

/// A camera with [`CameraGroupFollower`] includes this entity in its view.
#[derive(Component, Default)]
pub struct CameraGroupMember;

#[derive(Debug, Component, Deref, DerefMut)]
pub(crate) struct CameraSensitivity(Vec2);

impl Default for CameraSensitivity {
    fn default() -> Self {
        Self(Vec2::new(0.003, 0.002))
    }
}

/// The camera to use for controlling players that face the pointer.
#[derive(Component, Default, Debug, Clone, Copy)]
pub struct PlayerPointerCamera;

pub(crate) fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Name::new("Camera"),
        Camera {
            clear_color: ClearColorConfig::Custom(Color::BLACK),
            ..default()
        },
        Camera3d::default(),
        SpatialListener3D,
        Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Dir3::Y),
        Hdr,
        Tonemapping::None, // Magicka would not
        bevy::camera::Exposure {
            ev100: crate::magicka_level_model::light::MAGICKA_EXPOSURE,
        },
        Bloom::default(),
        CameraSensitivity::default(),
        CameraGroupFollower {
            position: default(),
            magnify: 1.,
        },
        PlayerPointerCamera,
    ));
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

fn move_free_camera(
    mut camera: Single<&mut Transform, (With<Camera3d>, With<PlayerControlled>)>,
    time: Res<Time>,
    kb_input: Res<ButtonInput<KeyCode>>,
) {
    let mut direction = Vec2::ZERO;

    if kb_input.pressed(KeyCode::KeyW) {
        direction.y -= 1.;
    }

    if kb_input.pressed(KeyCode::KeyS) {
        direction.y += 1.;
    }

    if kb_input.pressed(KeyCode::KeyA) {
        direction.x -= 1.;
    }

    if kb_input.pressed(KeyCode::KeyD) {
        direction.x += 1.;
    }

    let direction = camera.rotation * direction.normalize_or_zero().extend(0.).xzy();

    let move_delta = direction * 30. * time.delta_secs();
    camera.translation += move_delta;
}

fn rotate_free_camera(
    accumulated_mouse_motion: Res<AccumulatedMouseMotion>,
    camera: Single<(&mut Transform, &CameraSensitivity), (With<Camera3d>, With<PlayerControlled>)>,
    cursor_grab: Option<Single<&mut CursorOptions, With<bevy::window::PrimaryWindow>>>,
) {
    if let Some(cursor_grab) = cursor_grab
        && cursor_grab.grab_mode == CursorGrabMode::None
    {
        return;
    }

    let (mut transform, camera_sensitivity) = camera.into_inner();

    let delta = accumulated_mouse_motion.delta;

    if delta != Vec2::ZERO {
        let delta_yaw = -delta.x * camera_sensitivity.x;
        let delta_pitch = -delta.y * camera_sensitivity.y;

        let (yaw, pitch, roll) = transform.rotation.to_euler(EulerRot::YXZ);
        let yaw = yaw + delta_yaw;

        const PITCH_LIMIT: f32 = FRAC_PI_2 - 0.01;
        let pitch = (pitch + delta_pitch).clamp(-PITCH_LIMIT, PITCH_LIMIT);

        transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, roll);
    }
}
