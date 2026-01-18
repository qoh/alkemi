use log::warn;
use std::{cmp::Ordering, io::BufRead};
use thiserror::Error;
use xml::{EventReader, attribute::OwnedAttribute, name::OwnedName, reader::XmlEvent};

#[derive(Debug, Error)]
pub enum SceneError {
    #[error("Error reading XML data: {0}")]
    Xml(#[from] xml::reader::Error),
    #[error("There is no root <Scene> element in the XML data.")]
    NoSceneElement,
}

#[derive(Debug, Default)]
pub struct SceneConfig {
    pub force_camera: Option<bool>,
    pub force_nav_mesh: Option<bool>,
    pub model: Option<String>,
    pub triggers: Vec<Trigger>,
}

#[derive(Debug)]
pub struct Trigger {
    pub id: Option<String>,
    pub autorun: bool,
    pub repeat: Option<f32>,
    /// Nested list represents [ [X and Y] or [Z and W] ]
    pub conditions: Vec<Vec<TriggerCondition>>,
    pub actions: Vec<TriggerAction>,
}

#[derive(Debug)]
pub struct TriggerCondition {
    pub invert: bool,
    pub logic: TriggerConditionLogic,
}

#[derive(Debug)]
pub enum TriggerConditionLogic {
    Present {
        area: Option<String>,
        member_type: Option<String>,
        include_invisible: bool,
        compare_method: Ordering,
        nr: i32,
    },
    Unknown,
}

#[derive(Debug)]
pub struct TriggerAction {
    pub delay: f32,
    pub behavior: TriggerActionBehavior,
}

#[derive(Debug)]
pub enum TriggerActionBehavior {
    ChangeScene {
        scene: Option<String>,
        transition: SceneChangeTransition,
        transition_time: f32,
        spawn_players: bool,
        spawn_point: Option<String>,
        save_npcs: bool,
    },
    Spawn {
        area: String,
        type_name: String,
        nr: u16,
        id: Option<String>,
    },
    Unknown,
}

#[derive(Debug)]
pub enum SceneChangeTransition {
    None,
    Fade,
    CrossFade,
}

pub fn read_scene(reader: impl BufRead) -> Result<SceneConfig, SceneError> {
    let mut parser = EventReader::new(reader);

    let mut scene = SceneConfig::default();

    // Find the root <Scene> element
    loop {
        match parser.next()? {
            XmlEvent::EndDocument => return Err(SceneError::NoSceneElement),
            XmlEvent::StartElement {
                name, attributes, ..
            } => {
                if name.local_name.eq_ignore_ascii_case("scene") && name.namespace.is_none() {
                    for attribute in attributes {
                        if attribute
                            .name
                            .local_name
                            .eq_ignore_ascii_case("forcecamera")
                            && attribute.name.namespace.is_none()
                        {
                            scene.force_camera = Some(attribute.value.parse().unwrap());
                        } else if attribute
                            .name
                            .local_name
                            .eq_ignore_ascii_case("forcenavmesh")
                            && attribute.name.namespace.is_none()
                        {
                            scene.force_nav_mesh = Some(attribute.value.parse().unwrap());
                        }
                    }
                    break;
                } else {
                    parser.skip()?;
                }
            }
            _ => {}
        }
    }

    // Interpret contents of the <Scene> element until we get EndElement
    loop {
        match parser.next()? {
            XmlEvent::EndDocument => {
                // This shouldn't happen as EventReader checks for balancing before emitting events
                // Handle it just in case, otherwise a bug in the xml crate would mean we loop forever
                unreachable!("Missing end of scene element")
            }
            XmlEvent::EndElement { .. } => {
                return Ok(scene);
            }
            XmlEvent::StartElement {
                name,
                attributes,
                namespace: _,
            } => {
                read_element(&mut scene, &mut parser, name, attributes)?;
            }
            _ => {}
        }
    }
}

fn read_element(
    scene: &mut SceneConfig,
    parser: &mut EventReader<impl BufRead>,
    name: OwnedName,
    attributes: Vec<OwnedAttribute>,
) -> Result<(), SceneError> {
    if let Some(ns) = name.namespace_ref() {
        warn!(
            "Unhandled scene XML element {:?} in namespace {:?}",
            &name.local_name, ns
        );
        parser.skip()?;
        return Ok(());
    }

    let name = name.local_name.as_str();

    if name.eq_ignore_ascii_case("model") {
        scene.model = Some(read_content_required(parser)?);
    } else if name.eq_ignore_ascii_case("trigger") {
        scene.triggers.push(read_trigger(parser, attributes)?);
    } else {
        warn!("Unhandled scene XML element {:?}", name);
        #[cfg(test)]
        eprintln!("Unhandled scene XML element {:?}", name);
        parser.skip()?;
    }

    Ok(())
}

fn read_trigger(
    parser: &mut EventReader<impl BufRead>,
    attributes: Vec<OwnedAttribute>,
) -> Result<Trigger, SceneError> {
    let mut id = None;
    let mut autorun = true;
    let mut repeat = None;
    let mut conditions_or = Vec::new();
    let mut actions = Vec::new();

    for OwnedAttribute { name, value } in attributes {
        if name.local_name.eq_ignore_ascii_case("id") && name.namespace.is_none() {
            id = Some(value);
        } else if name.local_name.eq_ignore_ascii_case("autorun") && name.namespace.is_none() {
            autorun = value.parse().unwrap();
        } else if name.local_name.eq_ignore_ascii_case("repeat") && name.namespace.is_none() {
            if let Ok(interval) = value.parse::<f32>() {
                repeat = Some(interval);
            } else if let Ok(enable) = value.parse::<bool>() {
                repeat = if enable { Some(0.0) } else { None };
            } else {
                warn!("invalid scene trigger repeat value, ignoring");
            }
        }
    }

    loop {
        match parser.next()? {
            XmlEvent::StartElement { name, .. } => {
                if name.local_name.eq_ignore_ascii_case("if") && name.namespace.is_none() {
                    conditions_or.push(read_trigger_conditions(parser)?);
                } else if name.local_name.eq_ignore_ascii_case("then") && name.namespace.is_none() {
                    actions = read_trigger_actions(parser)?;
                } else {
                    parser.skip()?;
                }
            }
            XmlEvent::EndElement { .. } => {
                return Ok(Trigger {
                    id,
                    autorun,
                    repeat,
                    conditions: conditions_or,
                    actions,
                });
            }
            XmlEvent::EndDocument => unreachable!(),
            _ => {}
        }
    }
}

fn read_trigger_conditions(
    parser: &mut EventReader<impl BufRead>,
) -> Result<Vec<TriggerCondition>, SceneError> {
    let mut conditions_and = Vec::new();
    loop {
        match parser.next()? {
            XmlEvent::StartElement {
                name, attributes, ..
            } => {
                conditions_and.push(read_trigger_condition(name, attributes)?);
                parser.skip()?;
            }
            XmlEvent::EndElement { .. } => break,
            XmlEvent::EndDocument => unreachable!(),
            _ => {}
        }
    }
    Ok(conditions_and)
}

fn read_trigger_condition(
    name: OwnedName,
    attributes: Vec<OwnedAttribute>,
) -> Result<TriggerCondition, SceneError> {
    let mut invert = false;
    for attr in &attributes {
        if attr.name.local_name.eq_ignore_ascii_case("invert") && attr.name.namespace.is_none() {
            invert = attr.value.parse().unwrap();
        }
    }
    let logic = if name.local_name.eq_ignore_ascii_case("present") && name.namespace.is_none() {
        let mut area = None;
        let mut member_type = None;
        let mut include_invisible = true;
        let mut compare_method = Ordering::Equal;
        let mut nr = 0;
        for OwnedAttribute { name, value } in attributes {
            if name.namespace.is_some() {
                continue;
            }
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
        TriggerConditionLogic::Present {
            area,
            member_type,
            include_invisible,
            compare_method,
            nr,
        }
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
    Ok(TriggerCondition { invert, logic })
}

fn read_trigger_actions(
    parser: &mut EventReader<impl BufRead>,
) -> Result<Vec<TriggerAction>, SceneError> {
    let mut actions = Vec::new();
    let random_action_groups: Vec<Vec<TriggerAction>> = Vec::new();
    let only_looped_actions: Vec<TriggerAction> = Vec::new();
    loop {
        match parser.next()? {
            XmlEvent::StartElement {
                name, attributes, ..
            } => {
                if name.local_name.eq_ignore_ascii_case("random") && name.namespace.is_none() {
                    warn!("unhandled random action group");
                    parser.skip()?;
                } else if name.local_name.eq_ignore_ascii_case("looped") && name.namespace.is_none()
                {
                    warn!("unhandled looped action group");
                    parser.skip()?;
                } else {
                    actions.push(read_trigger_action(parser, name, attributes)?);
                }
            }
            XmlEvent::EndElement { .. } => break,
            XmlEvent::EndDocument => unreachable!(),
            _ => {}
        }
    }
    if !random_action_groups.is_empty() {
        // actions.push(TriggerAction::Random { groups: random_action_groups });
    }
    // ... looped

    Ok(actions)
}

fn read_trigger_action(
    parser: &mut EventReader<impl BufRead>,
    name: OwnedName,
    mut attributes: Vec<OwnedAttribute>,
) -> Result<TriggerAction, SceneError> {
    let mut delay: f32 = 0.0;
    attributes.retain_mut(|attr| {
        if attr.name.namespace.is_some() {
            return false;
        }
        if attr.name.local_name.eq_ignore_ascii_case("delay") {
            delay = attr.value.parse().unwrap();
        } else {
            return true;
        }
        false
    });

    let behavior = if name.local_name.eq_ignore_ascii_case("changescene") {
        let mut scene: Option<String> = None;
        let mut transition: SceneChangeTransition = SceneChangeTransition::Fade;
        let mut transition_time: f32 = 1.;
        let mut spawn_players: bool = false;
        let mut spawn_point: Option<String> = None;
        let mut save_npcs: bool = false;
        for OwnedAttribute { name, value } in attributes {
            if name.local_name.eq_ignore_ascii_case("scene") {
                scene = Some(value);
            } else if name.local_name.eq_ignore_ascii_case("transition") {
                if value.eq_ignore_ascii_case("none") {
                    transition = SceneChangeTransition::None;
                } else if value.eq_ignore_ascii_case("fade") {
                    transition = SceneChangeTransition::Fade;
                } else if value.eq_ignore_ascii_case("crossfade") {
                    transition = SceneChangeTransition::CrossFade;
                } else {
                    todo!("invalid changescene action transition error")
                }
            } else if name.local_name.eq_ignore_ascii_case("transitiontime") {
                transition_time = value.parse().unwrap();
            } else if name.local_name.eq_ignore_ascii_case("spawnplayers") {
                spawn_players = value.parse().unwrap();
            } else if name.local_name.eq_ignore_ascii_case("spawnpoint") {
                spawn_point = Some(value);
            } else if name.local_name.eq_ignore_ascii_case("savenpcs") {
                save_npcs = value.parse().unwrap();
            }
        }
        parser.skip()?;
        TriggerActionBehavior::ChangeScene {
            scene,
            transition,
            transition_time,
            spawn_players,
            spawn_point,
            save_npcs,
        }
    } else if name.local_name.eq_ignore_ascii_case("spawn") {
        let mut area: Option<String> = None;
        let mut type_name: Option<String> = None;
        let mut id: Option<String> = None;
        let mut nr: u16 = 1;
        for OwnedAttribute { name, value } in attributes {
            if name.local_name.eq_ignore_ascii_case("area") {
                area = Some(value);
            } else if name.local_name.eq_ignore_ascii_case("type") {
                type_name = Some(value);
            } else if name.local_name.eq_ignore_ascii_case("id") {
                id = Some(value);
            } else if name.local_name.eq_ignore_ascii_case("nr") {
                nr = value.parse().unwrap();
            } else {
                warn!(
                    "Unhandled trigger spawn action attribute {:?}",
                    name.local_name
                );
            }
        }
        parser.skip()?;
        TriggerActionBehavior::Spawn {
            area: area.unwrap(),
            type_name: type_name.unwrap(),
            id,
            nr,
        }
    } else {
        warn!("Unhandled scene trigger action type {:?}", name.local_name);
        #[cfg(test)]
        eprintln!("Unhandled scene trigger action type {:?}", name.local_name);
        parser.skip()?;
        TriggerActionBehavior::Unknown
    };

    Ok(TriggerAction { delay, behavior })
}

fn read_content_required(parser: &mut EventReader<impl BufRead>) -> Result<String, SceneError> {
    let mut value = None;
    loop {
        match parser.next()? {
            XmlEvent::Characters(s) => {
                let existing = value.replace(s);
                if existing.is_some() {
                    todo!("error for content-only element getting multiple character events?")
                }
            }
            XmlEvent::StartElement { .. } => {
                todo!("error for unexpected child element in content-only element")
            }
            XmlEvent::EndElement { .. } => {
                return value
                    .take()
                    .ok_or_else(|| todo!("error when content missing in element"));
            }
            XmlEvent::EndDocument => unreachable!(),
            _ => {}
        }
    }
}

#[cfg(test)]
mod test {
    use super::read_scene;
    use std::fs::File;
    use std::io::BufReader;

    #[test]
    fn test() {
        let path =
            "/data/SteamLibrary/steamapps/common/Magicka/Content/Levels/WizardCastle/wc_s4.xml";
        let file = BufReader::new(File::open(path).unwrap());
        let scene = read_scene(file).unwrap();
        dbg!(&scene);
    }
}
