pub mod condition;

pub mod action;

use std::sync::Arc;

use crate::magicka_scene;
use bevy::{ecs::query::QueryEntityError, prelude::*, time::Stopwatch};

#[derive(Component, Debug)]
pub struct Trigger {
    pub priority: usize,
    // /// Nested list represents [ [X and Y] or [Z and W] ]
    pub conditions: Arc<Vec<Vec<magicka_scene::TriggerCondition>>>,
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
    let should_execute = world.run_system_cached_with(condition::conditions_met, trigger)?;
    if should_execute {
        world.run_system_cached_with::<_, Result, _, _>(execute_trigger, trigger)??;
    }
    Ok(())
}

pub(crate) fn execute_trigger(
    In(trigger): In<Entity>,
    mut triggers: Query<(
        &mut Trigger,
        Option<&mut TriggerAutoEvaluate>,
        Option<&Name>,
    )>,
) -> Result {
    let (mut trigger_data, mut auto_evaluate, name) = triggers.get_mut(trigger)?;

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
                action::execute_trigger_action,
                (trigger, action_index),
            )??;
        }
    }
    Ok(())
}
