pub mod avatar_move;
pub mod change_scene;
pub mod execute_trigger_action;
pub mod play_animation;
pub mod spawn_character;

use crate::{magicka_scene::TriggerAction, script_triggers::Trigger};
use bevy::prelude::*;
use std::sync::Arc;

#[derive(Debug)]
pub enum TriggerActionBehavior {
    ChangeScene(Arc<change_scene::ChangeScene>),
    SpawnCharacter(Arc<spawn_character::SpawnCharacter>),
    ExecuteTrigger(Arc<execute_trigger_action::ExecuteTrigger>),
    PlayAnimation(Arc<play_animation::PlayAnimation>),
    AvatarMove(Arc<avatar_move::AvatarMove>),
    Unknown,
}

pub(super) fn execute_trigger_action(
    (In(trigger), In(action_index)): (In<Entity>, In<usize>),
    world: &mut World,
    q_triggers: &mut QueryState<(&Trigger, Option<&Name>)>,
) -> Result {
    let (trigger_data, name) = q_triggers.get(world, trigger)?;
    let Some((TriggerAction { behavior, .. }, _)) = trigger_data.actions.get(action_index) else {
        error!("Action index {action_index} on {name:?} ({trigger}) is out of range");
        return Ok(());
    };
    use TriggerActionBehavior::*;
    match behavior {
        ChangeScene(handle) => {
            world.run_system_cached_with::<_, Result, _, _>(
                change_scene::execute_change_scene,
                handle.clone(),
            )??;
        }
        SpawnCharacter(handle) => {
            world.run_system_cached_with::<_, Result, _, _>(
                spawn_character::execute_spawn_character,
                handle.clone(),
            )??;
        }
        ExecuteTrigger(handle) => {
            world.run_system_cached_with::<_, Result, _, _>(
                execute_trigger_action::execute_execute_trigger,
                handle.clone(),
            )??;
        }
        PlayAnimation(handle) => {
            world.run_system_cached_with::<_, Result, _, _>(
                play_animation::execute_play_animation,
                handle.clone(),
            )??;
        }
        AvatarMove(handle) => {
            world.run_system_cached_with(avatar_move::execute_avatar_move, handle.clone())?;
        }
        Unknown => {
            debug!("Unhandled executed action of trigger {name:?} ({trigger}): {behavior:?}");
        }
    }
    Ok(())
}

pub fn read_behavior(
    parser: &mut xml::EventReader<impl std::io::Read>,
    name: xml::name::OwnedName,
    attributes: Vec<xml::attribute::OwnedAttribute>,
) -> Result<TriggerActionBehavior, crate::magicka_scene::SceneError> {
    use TriggerActionBehavior::*;
    let behavior = if name.local_name.eq_ignore_ascii_case("changescene") {
        parser.skip()?;
        ChangeScene(Arc::new(change_scene::from_xml(attributes).unwrap()))
    } else if name.local_name.eq_ignore_ascii_case("spawn") {
        parser.skip()?;
        SpawnCharacter(Arc::new(spawn_character::from_xml(attributes).unwrap()))
    } else if name.local_name.eq_ignore_ascii_case("executetrigger") {
        parser.skip()?;
        ExecuteTrigger(Arc::new(
            execute_trigger_action::from_xml(attributes).unwrap(),
        ))
    } else if name.local_name.eq_ignore_ascii_case("playanimation") {
        parser.skip()?;
        PlayAnimation(Arc::new(play_animation::from_xml(attributes).unwrap()))
    } else if name.local_name.eq_ignore_ascii_case("avatarmove") {
        AvatarMove(Arc::new(avatar_move::from_xml(attributes, parser).unwrap()))
    } else {
        warn!("Unhandled scene trigger action type {:?}", name.local_name);
        #[cfg(test)]
        eprintln!("Unhandled scene trigger action type {:?}", name.local_name);
        parser.skip()?;
        TriggerActionBehavior::Unknown
    };
    Ok(behavior)
}
