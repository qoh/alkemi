use crate::{
    camera::{CameraGroupMember, PlayerPointerCamera},
    character::{
        Character, CharacterArgs, CharacterDesiredMovement, spawn_character, turn_to_direction,
    },
    magicka_level_model::Locator,
};
use bevy::{
    camera::RenderTarget,
    ecs::system::{SystemParam, SystemState},
    input::InputSystems,
    prelude::*,
    window::{PrimaryWindow, WindowRef},
};

pub fn plugin(app: &mut App) {
    app.add_systems(
        PreUpdate,
        (player_walk, set_pointer_ray).after(InputSystems),
    );
    app.add_systems(FixedUpdate, face_pointer_ray.before(turn_to_direction));
}

pub fn spawn_player_character(
    (In(level_entity), InRef(spawn_point_basename)): (In<Option<Entity>>, InRef<str>),
    world: &mut World,
    locators: &mut <SpawnLocatorsQuery as SystemParam>::State,
    global_transforms: &mut SystemState<TransformHelper>,
) -> Result<Entity> {
    let player_index = 0;
    let spawn_transform = find_spawn_transform(
        spawn_point_basename,
        player_index,
        &locators.query(world),
        &global_transforms.get(world),
    )?;

    let player_entity = world.run_system_cached_with::<_, Result<_>, _, _>(
        spawn_character,
        &CharacterArgs {
            type_name: "Wizard".to_owned(),
            spawn_transform,
            scene_entity: level_entity,
            model_index: None,
            start_as_agent: false,
        },
    )??;

    let mut character_world = world.entity_mut(player_entity);
    let mut character_component = character_world.get_mut::<Character>().unwrap();
    character_component.type_name = "wizard".to_string(); // For scene triggers

    world.entity_mut(player_entity).insert((
        Name::new(format!("Player {player_index}")),
        crate::spelling::bundle_m1(),
        crate::spelling::bindings_m1(),
        CameraGroupMember,
        crate::PlayerControlled,
        FacePointer,
    ));

    Ok(player_entity)
}

pub(super) type SpawnLocatorsQuery<'w, 's, 'n> =
    Query<'w, 's, (&'static Name, Entity), With<Locator>>;

pub(super) fn find_spawn_transform(
    spawn_basename: &str,
    player_index: u8,
    locators: &SpawnLocatorsQuery,
    transform_helper: &TransformHelper,
) -> Result<Transform> {
    let (_, spawn) = locators
        .iter()
        .find(|(n, _)| n.as_str() == format!("{}{}", spawn_basename, player_index))
        .unwrap();
    let spawn_transform = Transform::from(transform_helper.compute_global_transform(spawn)?);
    Ok(spawn_transform)
}

fn player_walk(
    players: Query<&mut CharacterDesiredMovement, With<crate::PlayerControlled>>,
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

    let direction = direction.normalize_or_zero().extend(0.).xzy();

    for mut movement in players {
        movement.movement = direction;
    }
}

#[derive(Component, Default, Clone, Copy, Debug)]
#[require(FacePointerRay)]
pub struct FacePointer;

#[derive(Component, Default, Clone, Copy, Debug)]
pub struct FacePointerRay {
    pub pointer_ray: Option<Ray3d>,
}

fn set_pointer_ray(
    players: Query<&mut FacePointerRay, With<FacePointer>>,
    primary_windows: Query<&Window, With<PrimaryWindow>>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform), With<PlayerPointerCamera>>,
) {
    let pointer_ray = get_pointer_ray(primary_windows, windows, cameras);
    for mut look_at in players {
        look_at.pointer_ray = pointer_ray;
    }
}

fn face_pointer_ray(
    players: Query<(Entity, &mut CharacterDesiredMovement, &FacePointerRay)>,
    transform_helper: TransformHelper,
) {
    for (entity, mut movement, look_at) in players {
        let up = Vec3::Y;
        let plane = InfinitePlane3d::new(up);
        if let Ok(player_trans) = transform_helper.compute_global_transform(entity)
            && let plane_origin = player_trans.translation()
            && let Some(ray) = look_at.pointer_ray
            && let Some(distance) = ray.intersect_plane(player_trans.translation(), plane)
        {
            let look_at_point = ray.get_point(distance);
            movement.direction = (look_at_point - plane_origin).normalize_or_zero();
        } else {
            movement.direction = Vec3::ZERO;
        };
    }
}

fn get_pointer_ray(
    primary_windows: Query<&Window, With<PrimaryWindow>>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform), With<PlayerPointerCamera>>,
) -> Option<Ray3d> {
    let (camera, camera_trans) = cameras
        .iter()
        .filter(|(c, _)| c.is_active)
        .max_by_key(|(c, _)| c.order)?;

    let window = match camera.target {
        RenderTarget::Window(WindowRef::Primary) => primary_windows.single().ok(),
        RenderTarget::Window(WindowRef::Entity(window)) => windows.get(window).ok(),
        _ => None,
    }?;

    window
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world(camera_trans, cursor).ok())
}
