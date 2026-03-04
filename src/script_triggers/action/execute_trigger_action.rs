use bevy::prelude::*;

use crate::script_triggers::{Trigger, evaluate_trigger};

#[derive(Clone, Debug)]
pub struct ExecuteTrigger {
    pub trigger_name: String,
}

pub fn execute_execute_trigger(
    action: In<impl AsRef<ExecuteTrigger>>,
    world: &mut World,
    triggers: &mut QueryState<(Entity, &Name), With<Trigger>>,
) -> Result {
    let action = action.as_ref();
    let (trigger, _name) = triggers
        .iter(world)
        .find(|(_t, name)| name.eq_ignore_ascii_case(&action.trigger_name))
        .ok_or("Can't find target trigger by name")?;
    // FIXME: This should also consider the trigger's run-once/repeat-interval criteria. Need to decouple that from autorun.
    world.run_system_cached_with::<_, Result, _, _>(evaluate_trigger, trigger)??;
    Ok(())
}

pub(crate) fn from_xml(
    attributes: Vec<xml::attribute::OwnedAttribute>,
) -> Result<ExecuteTrigger, ()> {
    let mut trigger_name: Option<String> = None;
    for xml::attribute::OwnedAttribute { name, value } in attributes {
        if name.local_name.eq_ignore_ascii_case("trigger") {
            trigger_name = Some(value);
        }
    }
    Ok(ExecuteTrigger {
        trigger_name: trigger_name.ok_or(())?,
    })
}
