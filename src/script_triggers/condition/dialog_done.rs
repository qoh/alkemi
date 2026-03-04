use bevy::prelude::*;

#[allow(dead_code)] // TODO: Start dialog action
#[derive(Clone, Debug)]
pub(crate) struct TriggerConditionDialogDone {
    pub dialog: String,
    pub interact_index: i32,
}

pub(super) fn condition_met_dialog_done(
    _condition: In<impl AsRef<TriggerConditionDialogDone>>,
) -> Option<bool> {
    Some(false)
}

pub(crate) fn from_xml(
    attributes: Vec<xml::attribute::OwnedAttribute>,
) -> Result<TriggerConditionDialogDone, ()> {
    let mut dialog = None;
    let mut interact_index = -1;
    for xml::attribute::OwnedAttribute { name, value } in attributes {
        if name.local_name.eq_ignore_ascii_case("dialog") {
            dialog = Some(value);
        } else if name.local_name.eq_ignore_ascii_case("interactindex") {
            interact_index = value.parse().unwrap();
        }
    }
    Ok(TriggerConditionDialogDone {
        dialog: dialog.ok_or(())?,
        interact_index,
    })
}
