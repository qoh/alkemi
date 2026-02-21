use crate::character::{
    Character, CharacterArgs, CharacterDesiredMovement, character_walk,
    player::{SpawnLocatorsQuery, find_spawn_transform},
    spawn_character, turn_to_direction,
};
use bevy::{
    ecs::system::{SystemParam, SystemState},
    prelude::*,
};

pub fn plugin(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        agent_walk
            .after(bevy_landmass::LandmassSystems::Output)
            .before(character_walk),
    );
    app.add_systems(FixedUpdate, face_move_dir.before(turn_to_direction));
}

pub fn spawn_follower(
    (In(level_entity), InRef(spawn_point_basename), In(target_entity)): (
        In<Option<Entity>>,
        InRef<str>,
        In<Entity>,
    ),
    world: &mut World,
    locators: &mut <SpawnLocatorsQuery as SystemParam>::State,
    global_transforms: &mut SystemState<TransformHelper>,
) -> Result {
    let player_index = 1;
    let spawn_transform = find_spawn_transform(
        spawn_point_basename,
        player_index,
        &locators.query(world),
        &global_transforms.get(world),
    )?;

    let follower = world.run_system_cached_with::<_, Result<_>, _, _>(
        spawn_character,
        &CharacterArgs {
            type_name: "Wizard_Reddit".to_owned(),
            spawn_transform,
            spawn_anchor: default(),
            scene_entity: level_entity,
            model_index: None,
            start_as_agent: true,
        },
    )??;

    let mut character_world = world.entity_mut(follower);
    let mut character_component = character_world.get_mut::<Character>().unwrap();
    character_component.accel = 12.;

    world.entity_mut(follower).insert((
        Name::new("Follower"),
        bevy_landmass::AgentTarget3d::Entity(target_entity),
    ));

    Ok(())
}

#[derive(Component, Default, Debug)]
pub struct FaceMoveDir;

fn agent_walk(
    agents: Query<(
        &bevy_landmass::AgentDesiredVelocity3d,
        &Character,
        &mut CharacterDesiredMovement,
    )>,
) {
    for (target_velocity, char, mut movement) in agents {
        let speed_recip = if char.speed != 0. && char.speed.is_finite() {
            1. / char.speed
        } else {
            0.
        };
        movement.movement =
            (target_velocity.velocity().with_y(0.) * speed_recip).clamp_length_max(1.);
    }
}

fn face_move_dir(
    facers: Query<
        &mut CharacterDesiredMovement,
        (With<FaceMoveDir>, Changed<CharacterDesiredMovement>),
    >,
) {
    for mut movement in facers {
        let dir = movement.movement.normalize_or_zero();
        if movement.direction != dir {
            movement.direction = dir;
        }
    }
}
