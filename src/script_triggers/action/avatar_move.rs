use bevy::prelude::*;

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

pub fn execute_avatar_move(action: In<impl AsRef<AvatarMove>>) {
    let action = action.as_ref();
    for (target, events) in &action.targets {
        for event in events {
            info!("NYI: AvatarMove of {target:?}: {event:?}");
        }
    }
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
                    && let Ok(player_index) = player_index_text.parse::<u8>()
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
