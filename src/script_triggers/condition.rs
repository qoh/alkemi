pub mod dialog_done;
pub mod present;

use super::Trigger;
use bevy::prelude::*;
use std::sync::Arc;

#[derive(Debug)]
pub enum TriggerConditionLogic {
    Present(Arc<present::TriggerConditionPresent>),
    DialogDone(Arc<dialog_done::TriggerConditionDialogDone>),
    Unknown,
}

pub(super) fn conditions_met(
    In(trigger): In<Entity>,
    world: &mut World,
    q_triggers: &mut QueryState<(&Trigger, Option<&Name>)>,
) -> Result<bool> {
    let (trigger_data, _name) = q_triggers.get(world, trigger)?;
    if trigger_data.conditions.is_empty() {
        return Ok(true);
    }
    let conditions = trigger_data.conditions.clone();
    'outer: for conjunction in conditions.as_ref() {
        for condition in conjunction {
            use TriggerConditionLogic::*;
            let evaluated: Option<bool> = match &condition.logic {
                Present(handle) => {
                    world.run_system_cached_with(present::condition_met_present, handle.clone())?
                }
                DialogDone(handle) => world.run_system_cached_with(
                    dialog_done::condition_met_dialog_done,
                    handle.clone(),
                )?,
                Unknown => None,
            };
            let evaluated = evaluated.map(|r| r ^ condition.invert);
            if evaluated.is_none() {
                let name = q_triggers
                    .get(world, trigger)
                    .ok()
                    .and_then(|(_data, name)| name)
                    .map(|n| n.as_str())
                    .unwrap_or("(unknown)");
                warn_once!(
                    "Trigger {name:?} condition could not be evaluated (will never succeed)"
                );
            }
            if evaluated != Some(true) {
                continue 'outer;
            }
        }
        return Ok(true);
    }
    Ok(false)
}

pub fn read_logic(
    name: xml::name::OwnedName,
    attributes: Vec<xml::attribute::OwnedAttribute>,
) -> Result<TriggerConditionLogic, ()> {
    let logic = if name.local_name.eq_ignore_ascii_case("present") {
        TriggerConditionLogic::Present(Arc::new(present::from_xml(attributes)))
    } else if name.local_name.eq_ignore_ascii_case("dialogdone") {
        TriggerConditionLogic::DialogDone(Arc::new(dialog_done::from_xml(attributes).unwrap()))
    } else {
        warn!(
            "Unhandled scene trigger condition type {:?}",
            name.local_name
        );
        #[cfg(test)]
        eprintln!(
            "Unhandled scene trigger condition type {:?}",
            name.local_name
        );
        TriggerConditionLogic::Unknown
    };
    Ok(logic)
}
