use crate::{
    camera::CameraGroupMember,
    character::{Character, CharacterArgs, CharacterDesiredMovement, spawn_character},
    magicka_level_model::Locator,
};
use bevy::{
    ecs::system::{SystemParam, SystemState},
    input::InputSystems,
    prelude::*,
};

pub fn plugin(app: &mut App) {
    app.add_systems(PreUpdate, player_walk.after(InputSystems));
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
