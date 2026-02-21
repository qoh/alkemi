use crate::magicka_scene::{self, TriggerActionBehavior};
use bevy::{
    ecs::{query::QueryEntityError, system::SystemState},
    prelude::*,
    time::Stopwatch,
};

#[derive(Component, Debug)]
pub struct Trigger {
    pub priority: usize,
    // /// Nested list represents [ [X and Y] or [Z and W] ]
    pub conditions: Vec<Vec<magicka_scene::TriggerCondition>>,
    pub actions: Vec<(magicka_scene::TriggerAction, ActionState)>,
}

#[derive(Debug, Default)]
pub struct ActionState {
    pub pending: Option<(usize, Stopwatch)>,
}

#[derive(Component, Debug)]
pub struct TriggerAutoEvaluate {
    pub criterion: AutoEvaluateCriterion,
}

#[derive(Debug)]
pub enum AutoEvaluateCriterion {
    Once(bool),
    Every(Timer),
}

pub fn plugin(app: &mut App) {
    app.add_systems(Update, evaluate_auto_triggers);
}

fn evaluate_auto_triggers(
    world: &mut World,
    q_triggers: &mut QueryState<(Entity, &Trigger, Option<&mut TriggerAutoEvaluate>)>,
    mut buf_triggers: Local<Vec<Entity>>,
) -> Result {
    buf_triggers.clear();
    buf_triggers.extend(
        q_triggers
            .query(world)
            .iter()
            .sort_by_key::<&Trigger, _>(|t| t.priority)
            .map(|(ent, _, _)| ent),
    );
    let delta = world.resource::<Time>().delta();
    for trigger in &buf_triggers {
        let (_, _, mut auto_evaluate) = match q_triggers.get_mut(world, *trigger) {
            Err(QueryEntityError::NotSpawned(..) | QueryEntityError::QueryDoesNotMatch(..)) => {
                // It just went away
                continue;
            }
            r => r?,
        };
        if let Some(auto_evaluate) = auto_evaluate.as_mut() {
            let evaluate_now = match auto_evaluate.criterion {
                AutoEvaluateCriterion::Once(executed) => !executed,
                AutoEvaluateCriterion::Every(ref mut timer) => {
                    timer.tick(delta);
                    timer.is_finished()
                }
            };
            if evaluate_now {
                world.run_system_cached_with::<_, Result, _, _>(evaluate_trigger, *trigger)??;
            }
        }
        world.run_system_cached_with::<_, Result, _, _>(run_trigger_actions, *trigger)??;
    }
    Ok(())
}

fn evaluate_trigger(In(trigger): In<Entity>, world: &mut World) -> Result {
    let should_execute = world.run_system_cached_with(conditions_met, trigger)?;
    if should_execute {
        world.run_system_cached_with::<_, Result, _, _>(execute_trigger, trigger)??;
    }
    Ok(())
}

fn conditions_met(
    In(trigger): In<Entity>,
    world: &mut World,
    q_triggers: &mut QueryState<(&Trigger, Option<&Name>)>,
    areas: &mut QueryState<(&Name, &crate::magicka_level_model::TriggerArea)>,
) -> Result<bool> {
    let (trigger_data, name) = q_triggers.get(world, trigger)?;
    if trigger_data.conditions.is_empty() {
        return Ok(true);
    }
    'outer: for conjunction in &trigger_data.conditions {
        for condition in conjunction {
            use crate::magicka_scene::TriggerConditionLogic::*;
            let evaluated: bool = match &condition.logic {
                Present {
                    area: area_name,
                    member_type,
                    include_invisible,
                    compare_method,
                    nr,
                } => {
                    if !*include_invisible {
                        warn_once!(
                            "Trigger {name:?} has present condition with unhandled invisible exclusion, will never succeed"
                        );
                        continue 'outer;
                    }
                    let Some(area_name) = area_name.as_ref() else {
                        continue;
                    };
                    let area_query = areas.query(world);
                    let Some((_name, area)) = area_query
                        .iter()
                        .find(|(n, _)| n.eq_ignore_ascii_case(area_name))
                    else {
                        warn!("Trigger {name:?} present condition can't find area {area_name:?}");
                        continue 'outer;
                    };
                    let current_count = if let Some(type_name) = member_type
                        && !type_name.eq_ignore_ascii_case("any")
                    {
                        area.num_characters_of_type(type_name)
                    } else {
                        area.num_characters()
                    };
                    match usize::try_from(*nr) {
                        Ok(nr) => current_count.cmp(&nr) == *compare_method,
                        Err(_) => {
                            warn_once!(
                                "Trigger {name:?} has out-of-range present condition count of {nr}, will never succeed"
                            );
                            continue 'outer;
                        }
                    }
                }
                Unknown => {
                    warn_once!("Trigger {name:?} has unhandled condition, will never succeed");
                    continue 'outer;
                }
            };
            let result = evaluated ^ condition.invert;
            if !result {
                continue 'outer;
            }
        }
        return Ok(true);
    }
    Ok(false)
}

pub(crate) fn execute_trigger(
    In(trigger): In<Entity>,
    world: &mut World,
    q_triggers: &mut QueryState<(
        &mut Trigger,
        Option<&mut TriggerAutoEvaluate>,
        Option<&Name>,
    )>,
) -> Result {
    let (mut trigger_data, mut auto_evaluate, name) = q_triggers.get_mut(world, trigger)?;

    info!("Executing trigger {name:?} ({trigger:?}");

    // Cycle the auto evaluate state
    if let Some(auto) = auto_evaluate.as_mut() {
        match auto.criterion {
            AutoEvaluateCriterion::Once(ref mut executed) => *executed = true,
            AutoEvaluateCriterion::Every(ref mut timer) => timer.reset(),
        }
    }

    for action in &mut trigger_data.actions {
        if let Some((count, _)) = action.1.pending.as_mut() {
            *count = count.saturating_add(1);
        } else {
            action.1.pending = Some((0, Stopwatch::new()));
        }
    }

    Ok(())
}

fn run_trigger_actions(
    In(trigger): In<Entity>,
    world: &mut World,
    q_triggers: &mut QueryState<(&mut Trigger, Option<&Name>)>,
) -> Result {
    let delta = world.resource::<Time>().delta();
    let (trigger_data, _name) = q_triggers.get_mut(world, trigger)?;
    let action_count = trigger_data.actions.len();
    for action_index in 0..action_count {
        let (mut trigger_data, name) = q_triggers.get_mut(world, trigger)?;
        let Some(action) = trigger_data.actions.get_mut(action_index) else {
            info!("Action count changed while running actions of trigger {name:?}");
            break;
        };
        let Some(ref mut pending_state) = action.1.pending else {
            continue;
        };
        pending_state.1.tick(delta);
        if pending_state.1.elapsed_secs() >= action.0.delay {
            pending_state.1.reset();
            if let Some(new_pending) = pending_state.0.checked_sub(1) {
                pending_state.1.reset();
                pending_state.0 = new_pending;
            } else {
                action.1.pending = None;
            }
            world.run_system_cached_with::<_, Result, _, _>(
                execute_trigger_action,
                (trigger, action_index),
            )??;
        }
    }
    Ok(())
}

fn execute_trigger_action(
    (In(trigger), In(action_index)): (In<Entity>, In<usize>),
    world: &mut World,
    q_triggers: &mut QueryState<(&Trigger, Option<&Name>)>,
    q_scenes: &mut QueryState<(Entity, &crate::scene::Scene)>,
    q_locators: &mut QueryState<(Entity, &Name), With<crate::magicka_level_model::Locator>>,
    q_areas: &mut QueryState<(Entity, &Name), With<crate::magicka_level_model::TriggerArea>>,
    transform_helper: &mut SystemState<TransformHelper>,
) -> Result {
    use crate::magicka_scene::TriggerAction;
    let (trigger_data, name) = q_triggers.get(world, trigger)?;
    let Some((TriggerAction { behavior, .. }, _)) = trigger_data.actions.get(action_index) else {
        error!("Action index {action_index} on {name:?} ({trigger}) is out of range");
        return Ok(());
    };
    match behavior {
        TriggerActionBehavior::ChangeScene {
            scene,
            transition,
            transition_time,
            spawn_players,
            spawn_point,
            save_npcs,
        } => {
            let (current_scene_entity, current_scene) = q_scenes.single(world).unwrap();
            // XXX: It would be nicer to attach a scene transition component to current_scene_entity
            // than add something global
            let change = crate::scene::ChangeScene {
                level: current_scene.level.clone(),
                scene: scene.as_ref().unwrap().clone(),
                spawn_players: *spawn_players,
                spawn_point: spawn_point.as_ref().cloned(),
            };
            world.run_system_cached_with(crate::scene::queue_delayed_scene_change, change)?;
        }
        TriggerActionBehavior::Spawn {
            area: area_name,
            type_name,
            id,
            nr,
        } => {
            let (spawner_entity, sample_space) = if let Some((locator, _name)) = q_locators
                .query(world)
                .iter()
                .find(|(_, n)| n.eq_ignore_ascii_case(area_name))
            {
                (locator, false)
            } else if let Some((area, _name)) = q_areas
                .query(world)
                .iter()
                .find(|(_, n)| n.eq_ignore_ascii_case(area_name))
            {
                (area, true)
            } else {
                warn!("Trigger {name:?} spawn action can't find locator/area {area_name:?}");
                return Ok(());
            };
            let spawner_transform = transform_helper
                .get(world)
                .compute_global_transform(spawner_entity)
                .unwrap();
            let (current_scene_entity, _current_scene) = q_scenes.single(world).unwrap();
            let id = id.clone();
            let type_name = type_name.clone();
            for _ in 0..*nr {
                let spawn_transform = if sample_space {
                    use rand::{SeedableRng as _, distr::uniform::SampleRange as _};
                    use std::f32::consts::TAU;
                    let mut rng = rand::rngs::SmallRng::from_os_rng();
                    let local_point = vec3(
                        (-1.0..=1.0).sample_single(&mut rng).unwrap(),
                        (-1.0..=1.0).sample_single(&mut rng).unwrap(),
                        (-1.0..=1.0).sample_single(&mut rng).unwrap(),
                    );
                    let yaw = (0.0..TAU).sample_single(&mut rng).unwrap();
                    let translation = spawner_transform.transform_point(local_point);
                    GlobalTransform::from(
                        Transform::from_translation(translation)
                            .with_rotation(Quat::from_rotation_y(yaw)),
                    )
                } else {
                    spawner_transform
                };
                let character = world
                    .run_system_cached_with::<_, Result<_>, _, _>(
                        crate::character::spawn_character,
                        &crate::character::CharacterArgs {
                            type_name: type_name.clone(),
                            spawn_transform: spawn_transform.into(),
                            spawn_anchor: default(),
                            scene_entity: Some(current_scene_entity),
                            model_index: None,
                            start_as_agent: true,
                        },
                    )
                    .unwrap()
                    .unwrap();
                if let Some(ref id) = id {
                    world.entity_mut(character).insert(Name::new(id.clone()));
                }
            }
        }
        TriggerActionBehavior::Unknown => {
            debug!("Unhandled executed action of trigger {name:?} ({trigger}): {behavior:?}");
        }
    }
    Ok(())
}
