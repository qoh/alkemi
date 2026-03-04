use bevy::prelude::*;
use std::cmp::Ordering;

#[derive(Clone, Debug)]
pub(crate) struct TriggerConditionPresent {
    pub area: Option<String>,
    pub member_type: Option<String>,
    pub include_invisible: bool,
    pub compare_method: Ordering,
    pub nr: i32,
}

pub(super) fn condition_met_present(
    condition: In<impl AsRef<TriggerConditionPresent>>,
    areas: Query<(&Name, &crate::magicka_level_model::TriggerArea)>,
) -> Option<bool> {
    let TriggerConditionPresent {
        area: area_name,
        member_type,
        include_invisible,
        compare_method,
        nr,
    } = condition.as_ref();
    if !*include_invisible {
        warn_once!("Present condition with unhandled invisible exclusion");
        return None;
    }
    let Some(area_name) = area_name.as_ref() else {
        warn_once!("Present condition has no area name");
        return None;
    };
    let compare_count = match usize::try_from(*nr) {
        Ok(nr) => nr,
        Err(_) => {
            warn_once!("Present condition has out-of-range count of {nr}");
            return None;
        }
    };

    let Some((_name, area)) = areas
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(area_name))
    else {
        warn!("Present condition can't find area {area_name:?}");
        return None;
    };

    let current_count = if let Some(type_name) = member_type
        && !type_name.eq_ignore_ascii_case("any")
    {
        area.num_characters_of_type(type_name)
    } else {
        area.num_characters()
    };
    Some(current_count.cmp(&compare_count) == *compare_method)
}

pub(crate) fn from_xml(attributes: Vec<xml::attribute::OwnedAttribute>) -> TriggerConditionPresent {
    let mut area = None;
    let mut member_type = None;
    let mut include_invisible = true;
    let mut compare_method = Ordering::Equal;
    let mut nr = 0;
    for xml::attribute::OwnedAttribute { name, value } in attributes {
        if name.local_name.eq_ignore_ascii_case("area") {
            area = Some(value);
        } else if name.local_name.eq_ignore_ascii_case("type") {
            member_type = Some(value);
        } else if name.local_name.eq_ignore_ascii_case("includeinvisible") {
            include_invisible = value.parse().unwrap();
        } else if name.local_name.eq_ignore_ascii_case("comparemethod") {
            if value.eq_ignore_ascii_case("equal") {
                compare_method = Ordering::Equal;
            } else if value.eq_ignore_ascii_case("less") {
                compare_method = Ordering::Less;
            } else if value.eq_ignore_ascii_case("greater") {
                compare_method = Ordering::Greater;
            } else {
                todo!("invalid trigger condition comparemethod error")
            }
        } else if name.local_name.eq_ignore_ascii_case("nr") {
            nr = value.parse().unwrap();
        }
    }
    TriggerConditionPresent {
        area,
        member_type,
        include_invisible,
        compare_method,
        nr,
    }
}
