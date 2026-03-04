use bevy::prelude::*;

use crate::character::{Character, player::PlayerCharacter};

#[derive(Clone, Debug)]
pub struct AvatarMove {
    targets: Vec<(PlayerTarget, Vec<AvatarEvent>)>,
}

#[derive(Clone, Debug)]
enum PlayerTarget {
    Index(u8),
    Name,
}

#[derive(Clone, Debug)]
struct AvatarEvent {
    delay: f32,
    trigger: Option<String>,
    data: AvatarEventData,
}

#[derive(Clone, Debug)]
enum AvatarEventData {
    Move {
        position: MoveTarget,
        facing_direction: MoveFacing,
        speed: f32,
    },
    Animation,
    Face,
    Kill,
    Loop,
}

#[derive(Clone, Debug)]
enum MoveTarget {
    Position(Vec3),
    Trigger(String),
}

#[derive(Clone, Debug)]
enum MoveFacing {
    Direction(Dir3),
    Undirected(bool),
}

pub fn execute_avatar_move(
    action: In<impl AsRef<AvatarMove>>,
    numbered_players: Query<(Entity, &PlayerCharacter)>,
    mut player_transforms_and_transform_helper: ParamSet<(
        Query<&mut Transform, With<Character>>,
        TransformHelper,
    )>,
    locators: Query<(Entity, &Name), With<crate::magicka_level_model::Locator>>,
    areas: Query<(Entity, &Name), With<crate::magicka_level_model::TriggerArea>>,
) {
    let action = action.as_ref();
    for (target, events) in &action.targets {
        let target_entity = match *target {
            PlayerTarget::Index(n) => numbered_players
                .iter()
                .find(|(_e, ch)| ch.index == n)
                .map(|(e, _ch)| e),
            PlayerTarget::Name => {
                info!("NYI: AvatarMove target player by ID");
                None
            }
        };
        let Some(target_entity) = target_entity else {
            // There is no such player (common if playing multiplayer map in singleplayer)
            continue;
        };
        for event in events {
            // TODO: queueing?, delay, trigger
            match event.data {
                AvatarEventData::Move {
                    position: ref target_position_source,
                    facing_direction: _,
                    speed: _,
                } => {
                    let target_position = match *target_position_source {
                        MoveTarget::Position(target) => target,
                        MoveTarget::Trigger(ref id) => {
                            let Some(location) = named_trigger_location(
                                id,
                                &locators,
                                &areas,
                                &player_transforms_and_transform_helper.p1(),
                            ) else {
                                warn!(
                                    "AvatarMove Move can't find position target {target_position_source:?}"
                                );
                                continue;
                            };
                            location
                        }
                    };

                    // TODO: Walk there instead of teleporting there
                    if let Ok(mut trans) = player_transforms_and_transform_helper
                        .p0()
                        .get_mut(target_entity)
                    {
                        trans.translation = target_position;
                    } else {
                        warn!(
                            "AvatarMove target player {target:?} {target_entity:?} has no Transform for Move"
                        );
                    }
                }
                _ => info!("NYI: AvatarMove {event:?} (for {target:?})"),
            }
        }
    }
}

// FIXME: This somewhat duplicates what action::spawn_character does
fn named_trigger_location(
    name: &str,
    locators: &Query<(Entity, &Name), With<crate::magicka_level_model::Locator>>,
    areas: &Query<(Entity, &Name), With<crate::magicka_level_model::TriggerArea>>,
    transform_helper: &TransformHelper,
) -> Option<Vec3> {
    let (source_entity, sample_space) = if let Some((locator, _name)) =
        locators.iter().find(|(_, n)| n.eq_ignore_ascii_case(name))
    {
        (locator, false)
    } else if let Some((area, _name)) = areas.iter().find(|(_, n)| n.eq_ignore_ascii_case(name)) {
        (area, true)
    } else {
        return None;
    };

    let source_transform = transform_helper
        .compute_global_transform(source_entity)
        .ok()?;
    let location = if sample_space {
        use rand::{SeedableRng as _, distr::uniform::SampleRange as _};
        let mut rng = rand::rngs::SmallRng::from_os_rng();
        let local_point = vec3(
            (-1.0..=1.0).sample_single(&mut rng).unwrap(),
            (-1.0..=1.0).sample_single(&mut rng).unwrap(),
            (-1.0..=1.0).sample_single(&mut rng).unwrap(),
        );
        source_transform.transform_point(local_point)
    } else {
        source_transform.translation()
    };
    Some(location)
}

pub(crate) fn from_xml(
    attributes: Vec<xml::attribute::OwnedAttribute>,
    parser: &mut xml::EventReader<impl std::io::Read>,
) -> Result<AvatarMove, ()> {
    let mut targets = Vec::new();
    loop {
        match parser.next().map_err(|_e| todo!())? {
            xml::reader::XmlEvent::EndElement { .. } => break,
            xml::reader::XmlEvent::StartElement {
                name,
                attributes: _,
                namespace: _,
            } => {
                let lowercase = name.local_name.to_ascii_lowercase();
                let target = if let Some(player_index_text) = lowercase.strip_prefix("player")
                    && let Some(player_index) = player_index_text
                        .parse::<u8>()
                        .ok()
                        .and_then(|n| n.checked_sub(1))
                {
                    PlayerTarget::Index(player_index)
                } else if lowercase == "playerid" {
                    // TODO: Read 'name' attribute
                    PlayerTarget::Name
                } else {
                    parser.skip().map_err(|_e| todo!())?;
                    continue;
                };

                let events = parse_xml_events(parser)?;
                targets.push((target, events));
            }
            _ => {}
        }
    }
    Ok(AvatarMove { targets })
}

fn parse_xml_events(
    parser: &mut xml::EventReader<impl std::io::Read>,
) -> Result<Vec<AvatarEvent>, ()> {
    let mut events = Vec::new();
    loop {
        match parser.next().map_err(|_e| todo!())? {
            xml::reader::XmlEvent::EndElement { .. } => break,
            xml::reader::XmlEvent::StartElement {
                name,
                mut attributes,
                namespace: _,
            } => {
                let mut delay = 0.;
                let mut trigger = None;
                attributes.retain_mut(|xml::attribute::OwnedAttribute { name, value }| {
                    if name.local_name.eq_ignore_ascii_case("delay") {
                        delay = value.parse().unwrap_or_else(|_e| todo!());
                    } else if name.local_name.eq_ignore_ascii_case("trigger") {
                        trigger = Some(value.clone()); // Yucky clone
                    } else {
                        return true;
                    }
                    false
                });
                let data = if name.local_name.eq_ignore_ascii_case("move") {
                    let mut position = None;
                    let mut facing_direction = MoveFacing::Undirected(false);
                    let mut speed = 1.;
                    parser.skip().map_err(|_e| todo!())?;
                    for xml::attribute::OwnedAttribute { name, value } in attributes {
                        if name.local_name.eq_ignore_ascii_case("position") {
                            let coordinates: Vec<_> = value.splitn(3, ',').collect();
                            let target = if let [x, y, z] = coordinates.as_slice() {
                                let pos = vec3(
                                    x.parse().unwrap(),
                                    y.parse().unwrap(),
                                    z.parse().unwrap(),
                                );
                                MoveTarget::Position(pos)
                            } else {
                                MoveTarget::Trigger(value)
                            };
                            position = Some(target);
                        } else if name.local_name.eq_ignore_ascii_case("facingdirection") {
                            let coordinates: Vec<_> = value.splitn(3, ',').collect();
                            facing_direction = if let [x, y, z] = coordinates.as_slice() {
                                let dir = Dir3::new(vec3(
                                    x.parse().unwrap(),
                                    y.parse().unwrap(),
                                    z.parse().unwrap(),
                                ))
                                .unwrap_or(Dir3::X);
                                MoveFacing::Direction(dir)
                            } else {
                                MoveFacing::Undirected(value.parse().unwrap())
                            };
                        } else if name.local_name.eq_ignore_ascii_case("speed") {
                            speed = value.parse().unwrap();
                        } else {
                            warn!("Unknown avatar move action attribute {:?}", name.local_name);
                        }
                    }
                    AvatarEventData::Move {
                        position: position.unwrap(),
                        facing_direction,
                        speed,
                    }
                } else if name.local_name.eq_ignore_ascii_case("animation") {
                    // TODO: attributes: animation, idleanimation, blendtime, trigger
                    parser.skip().map_err(|_e| todo!())?;
                    AvatarEventData::Animation
                } else if name.local_name.eq_ignore_ascii_case("face") {
                    // TODO: attributes: facingdirection, speed
                    parser.skip().map_err(|_e| todo!())?;
                    AvatarEventData::Face
                } else if name.local_name.eq_ignore_ascii_case("kill") {
                    // TODO: attributes: remove
                    parser.skip().map_err(|_e| todo!())?;
                    AvatarEventData::Kill
                } else if name.local_name.eq_ignore_ascii_case("loop") {
                    // TODO: attributes: type
                    parser.skip().map_err(|_e| todo!())?;
                    AvatarEventData::Loop
                } else {
                    todo!("error for unknown avatarmove event type")
                };
                events.push(AvatarEvent {
                    delay,
                    trigger,
                    data,
                });
            }
            xml::reader::XmlEvent::Characters(_) => todo!(),
            _ => {}
        }
    }
    Ok(events)
}
