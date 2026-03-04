use bevy::{animation::RepeatAnimation, prelude::*};

use crate::magicka_level_model::animated_parts::AnimatedPart;

#[derive(Clone, Debug)]
pub struct PlayAnimation {
    pub name_path: Vec<String>,
    pub play_children: bool,
    pub args: PlayArgs,
}

#[derive(Clone, Copy, Debug)]
pub struct PlayArgs {
    pub speed: f32,
    pub start_time: Option<f32>,
    pub end_time: Option<f32>,
    pub looping: bool,
    pub resume_paused: bool,
}

pub fn execute_play_animation(
    action: In<impl AsRef<PlayAnimation>>,
    maybe_root_animated_parts: Query<(Entity, &Name, Option<&ChildOf>), With<AnimatedPart>>,
    children: Query<&Children>,
    mut animated_parts: Query<(&mut AnimationPlayer, &AnimationGraphHandle), With<AnimatedPart>>,
    mut animation_clips: ResMut<Assets<AnimationClip>>,
    mut animation_graphs: ResMut<Assets<AnimationGraph>>,
) -> Result {
    let action = action.as_ref();
    let mut name_path = action.name_path.iter();

    // First find the root
    let root_name = name_path
        .next()
        .ok_or("Play animation action has no animation name")?;
    let (mut part, _name, _parent) = maybe_root_animated_parts
        .iter()
        .filter(|(_, _, parent)| {
            // Only allow parts with non-part parents to avoid matching child parts or parts parented to scenes
            if let Some(parent) = parent {
                !maybe_root_animated_parts.contains(parent.parent())
            } else {
                true
            }
        })
        .find(|(_, name, _)| name.eq_ignore_ascii_case(root_name))
        .ok_or("Play animation action can't find named animated part")?;
    // Then any children
    for child_name in name_path {
        part = children
            .get(part)
            .into_iter()
            .flatten()
            .copied()
            .flat_map(|child| {
                maybe_root_animated_parts
                    .get(child)
                    .map(|(_, name, _)| (child, name))
                    .ok()
            })
            .find(|(_, name)| name.eq_ignore_ascii_case(child_name))
            .map(|(child, _name)| child)
            .ok_or("Play animation action can't find child named animated part")?;
    }

    play_inner(
        action.play_children.then_some(&children),
        part,
        action.args,
        animated_parts.reborrow(),
        animation_clips.reborrow(),
        animation_graphs.reborrow(),
    )
}

fn play_inner(
    children: Option<&Query<&Children>>,
    part: Entity,
    args: PlayArgs,
    mut animated_parts: Query<(&mut AnimationPlayer, &AnimationGraphHandle), With<AnimatedPart>>,
    mut animation_clips: Mut<Assets<AnimationClip>>,
    mut animation_graphs: Mut<Assets<AnimationGraph>>,
) -> Result {
    let (mut player, graph_handle) = animated_parts
        .get_mut(part)
        .map_err(|_e| "Play animation action can't access animation player")?;

    let (existing_node_index, animation) = player
        .playing_animations_mut()
        .next()
        .ok_or("Play animation action target part has no animations")?;

    let graph = animation_graphs
        .get(graph_handle)
        .ok_or("Play animation action targeting part with invalid animation graph handle")?;
    let graph_node = graph
        .get(*existing_node_index)
        .ok_or("Animation player refers to missing animation graph node")?;
    let clip_handle = match &graph_node.node_type {
        AnimationNodeType::Clip(handle) => Some(handle),
        _ => None,
    }
    .ok_or("Animated part is playing non-clip animation graph node")?;
    let clip = animation_clips
        .get(clip_handle)
        .ok_or("Animated part is playing missing animation clip")?;

    animation.set_speed(args.speed);
    animation.set_repeat(if args.looping {
        RepeatAnimation::Forever
    } else {
        RepeatAnimation::Never
    });

    // Defaults to [0..duration)
    let start_time = args.start_time.unwrap_or(0.);
    let end_time = args.end_time.unwrap_or(clip.duration());
    // In case they're swapped
    let (start_time, end_time) = (start_time.min(end_time), start_time.max(end_time));

    if start_time > 0. || end_time < clip.duration() {
        info!("Unimplemented trigger <playAnimation> start=/end= time parameters");
    }

    if !args.resume_paused {
        animation.set_seek_time(args.start_time.unwrap_or(0.));
    }
    animation.resume();

    if let Some(children) = children {
        for child in children.get(part).ok().into_iter().flatten().copied() {
            if !animated_parts.contains(child) {
                // TODO: This could be an error signaled from the play_inner call that we silence instead
                continue;
            }
            play_inner(
                Some(children),
                child,
                args,
                animated_parts.reborrow(),
                animation_clips.reborrow(),
                animation_graphs.reborrow(),
            )?;
        }
    }

    Ok(())
}

pub(crate) fn from_xml(
    attributes: Vec<xml::attribute::OwnedAttribute>,
) -> Result<PlayAnimation, ()> {
    let mut name_path = None;
    let mut speed = 1.;
    let mut start_time = None;
    let mut end_time = None;
    let mut looping = false;
    let mut play_children = true;
    let mut resume_paused = false;

    for xml::attribute::OwnedAttribute { name, value } in attributes {
        if name.local_name.eq_ignore_ascii_case("name") {
            name_path = Some(value.split('/').map(|s| s.to_owned()).collect());
        } else if name.local_name.eq_ignore_ascii_case("speed") {
            speed = value.parse().map_err(|_e| ())?;
        } else if name.local_name.eq_ignore_ascii_case("start") {
            let t: f32 = value.parse().map_err(|_e| ())?;
            start_time = if t < 0. { None } else { Some(t) };
        } else if name.local_name.eq_ignore_ascii_case("end") {
            let t: f32 = value.parse().map_err(|_e| ())?;
            end_time = if t < 0. { None } else { Some(t) };
        } else if name.local_name.eq_ignore_ascii_case("loop") {
            looping = value.parse().map_err(|_e| ())?;
        } else if name.local_name.eq_ignore_ascii_case("children") {
            play_children = value.parse().map_err(|_e| ())?;
        } else if name.local_name.eq_ignore_ascii_case("resume") {
            resume_paused = value.parse().map_err(|_e| ())?;
        }
    }
    Ok(PlayAnimation {
        name_path: name_path.ok_or(())?,
        play_children,
        args: PlayArgs {
            speed,
            start_time,
            end_time,
            looping,
            resume_paused,
        },
    })
}
