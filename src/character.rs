use crate::{
    character::model::attach_model, gameplay::damage::Health,
    magicka_assets::skinned_model::AnimationLibrary,
};
use avian3d::prelude::*;
use bevy::prelude::*;
use remagic::xnb_readers::magicka_character::CharacterTemplate;
use std::ffi::OsStr;
use typed_path::PlatformPathBuf;

mod agent;
mod model;
mod player;

pub(crate) use agent::spawn_follower;
pub(crate) use player::spawn_player_character;

pub fn plugin(app: &mut App) {
    app.add_plugins((model::plugin, agent::plugin, player::plugin));
    app.add_systems(FixedUpdate, character_walk);
    app.add_systems(FixedUpdate, face_move_dir.in_set(PhysicsSystems::Last));
    app.add_systems(
        PostUpdate,
        (
            select_animation,
            character_play_animation.before(bevy::app::AnimationSystems),
        )
            .chain(),
    );
}

#[derive(Component, Debug)]
pub struct Character {
    pub type_name: String,
    pub speed: f32,
    pub accel: f32,
    pub turn_speed: f32,
}

#[derive(Component, Default, Debug, Reflect)]
pub struct CharacterDesiredMovement {
    /// Magnitude 1 means max speed
    pub movement: Vec3,
}

#[derive(Component, Debug, Reflect)]
pub struct CharacterAnimationState {
    pub base_animation: &'static str,
    pub force_animation: Option<String>,
    animation_skeleton: Entity,
    currently_playing: Option<AnimationNodeIndex>,
    #[reflect(ignore)]
    animation_sets: Vec<remagic::xnb_readers::magicka_character::AnimationSet>,
    animation_set_index: usize,
}

#[derive(Debug)]
pub struct CharacterArgs {
    pub type_name: String,
    pub spawn_transform: Transform,
    pub scene_entity: Option<Entity>,
    pub model_index: Option<usize>,
    pub start_as_agent: bool,
}

pub(crate) fn spawn_character(
    InRef(CharacterArgs {
        type_name: template_name,
        spawn_transform,
        scene_entity: level_entity,
        model_index,
        start_as_agent,
    }): InRef<CharacterArgs>,
    mut commands: Commands,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<StandardMaterial>>,
    nav_mesh_archipelago: Option<
        Single<Entity, With<bevy_landmass::Archipelago<bevy_landmass::coords::ThreeD>>>,
    >,
    assets: Res<AssetServer>,
) -> Result<Entity> {
    fn read_template(template_name: &str) -> (CharacterTemplate, PlatformPathBuf) {
        let mut content_path: PlatformPathBuf =
            ["Data", "Characters", template_name].iter().collect();
        content_path.set_extension("xnb");
        let path = crate::magicka_assets::content_root()
            .join_checked(&content_path)
            .unwrap();
        let path = std::path::Path::new(path.as_ref() as &OsStr);
        let template_bytes = crate::magicka_assets::read_ignore_path_ascii_case(path).unwrap();

        let template = remagic::parse_character(&template_bytes)
            .map_err(|e| e.into_inner())
            .unwrap()
            .into_inner()
            .unwrap();
        (template, content_path)
    }

    // HACK: Character template parsing is not fully implemented, catch todo!/unimplemented!s and fall back to known supported template
    let (template, content_path) = std::panic::catch_unwind(|| read_template(template_name))
        .unwrap_or_else(|_why| {
            eprintln!("Reading character template {template_name:?} failed (will fall back to Wizard_Detective)");
            read_template("Wizard_Detective")
        });

    let CharacterTemplate {
        id: _, // This is the type="" for triggers, unless this is for a player, where it's overridden to "wizard"
        display_id: _,
        max_hitpoints,
        length,
        radius,
        mass,
        speed,
        turn_speed,
        skinned_models: _,
        ref animation_sets,
    } = template;

    let model_index = model_index.unwrap_or(0); // TODO: random

    let full_height = length + radius * 2.;

    let mut player = commands.spawn((
        Name::new("Character"),
        *spawn_transform,
        Visibility::default(),
        RigidBody::Dynamic,
        Collider::capsule(radius, length),
        Mass(mass),
        LockedAxes::ROTATION_LOCKED,
        TransformInterpolation,
        Health::full(max_hitpoints),
        Character {
            type_name: template_name.clone(),
            speed,
            accel: 100.,
            turn_speed,
        },
        CharacterDesiredMovement::default(),
    ));
    let player_entity = player.id();
    if let Some(level_entity) = level_entity {
        player.insert(ChildOf(*level_entity));
    }

    if let Some(archipelago) = nav_mesh_archipelago {
        let nav_radius = full_height * 0.5;

        player.insert(
            bevy_landmass::ArchipelagoRef::<bevy_landmass::coords::ThreeD>::new(
                archipelago.into_inner(),
            ),
        );

        if *start_as_agent {
            player.insert((
                bevy_landmass::Agent3d::default(),
                bevy_landmass::AgentSettings {
                    radius: nav_radius,
                    desired_speed: speed * 0.7,
                    max_speed: speed,
                },
            ));
        } else {
            player.insert((
                bevy_landmass::Character::<bevy_landmass::coords::ThreeD>::default(),
                bevy_landmass::CharacterSettings { radius: nav_radius },
            ));
        }
    }

    let attached_model = attach_model(
        content_path.as_path(),
        &template,
        model_index,
        player.reborrow(),
        Transform::default(),
        meshes.into(),
        materials.into(),
        &assets,
    );

    player.insert(CharacterAnimationState {
        base_animation: "idle",
        force_animation: None,
        animation_skeleton: attached_model.skeleton,
        currently_playing: None,
        animation_set_index: 0,
        // HACK: The character should really refer to the character template with an asset handle instead
        animation_sets: animation_sets.clone(),
    });

    Ok(player_entity)
}

fn select_animation(
    characters: Query<(
        &mut CharacterAnimationState,
        Option<&CharacterDesiredMovement>,
        Option<&LinearVelocity>,
    )>,
) {
    for (mut char_anim, movement, _vel) in characters {
        let move_length = movement.map_or(0., |m| m.movement.length_squared());
        let current_anim = char_anim.base_animation;
        // TODO: blend?
        let anim = if move_length > 0.01 {
            let actual_move_length = move_length; // TODO: Use LinearVelocity relative to ground?
            if actual_move_length >= 0.6 {
                "move_run"
            } else if actual_move_length < 0.4 {
                "move_walk"
            } else if current_anim != "move_run" && current_anim != "move_walk" {
                "move_walk"
            } else {
                current_anim
            }
        } else {
            "idle"
        };
        if anim != current_anim {
            char_anim.base_animation = anim;
        }
    }
}

fn character_play_animation(
    mut skinnedmesh_roots: Query<(&mut AnimationPlayer, &AnimationLibrary)>,
    characters: Query<(Entity, &CharacterAnimationState)>,
    children: Query<&Children>,
) {
    for (char, char_anim) in characters {
        for descendant in children.iter_descendants(char_anim.animation_skeleton) {
            let Some((mut animator, anims)) = skinnedmesh_roots.get_mut(descendant).ok() else {
                continue;
            };

            let anim = char_anim
                .force_animation
                .as_deref()
                .unwrap_or(char_anim.base_animation);

            let Some(animation_set) = char_anim.animation_sets.get(char_anim.animation_set_index)
            else {
                warn_once!(
                    "Animation set index {} not found on entity {char:?}",
                    char_anim.animation_set_index
                );
                break;
            };

            // TODO: Map the general animation name `anim` through the character template's AnimationSet to get the clip name
            let Some(animation_entry) = animation_set.animations.get(anim) else {
                warn_once!(
                    "Animation {anim:?} not found in active animation set {} of entity {char:?}",
                    char_anim.animation_set_index
                );
                break;
            };

            let Some(clip) = anims.animations.get(&animation_entry.clip_name).copied() else {
                warn_once!(
                    "Model animation {model_anim:?} not found, referenced for animation {anim:?} by active animation set {set} of entity {char:?}",
                    model_anim = animation_entry.clip_name,
                    set = char_anim.animation_set_index
                );
                break; // Give up if there's no idle either
            };

            // TODO: Crossfade (animation_entry.blend_time)
            if !animator.is_playing_animation(clip) {
                animator.stop_all();

                let playback = animator.play(clip);
                playback.set_speed(animation_entry.speed);
                if animation_entry.repeat {
                    playback.repeat();
                }
            }

            // TODO: Actions (animation_entry.actions)

            break; // There shouldn't be multiple skeletons, that would be scary
        }
    }
}

fn face_move_dir(
    characters: Query<(&mut Transform, &CharacterDesiredMovement, &Character)>,
    time: Res<Time>,
) {
    for (mut trans, movement, char) in characters {
        let (dir, speed_scale) = movement.movement.normalize_and_length();
        if speed_scale > 0.01 {
            let want_rot = Quat::look_to_rh(dir, Vec3::Y).inverse();
            trans.rotation = trans
                .rotation
                .interpolate_stable(&want_rot, time.delta_secs() * char.turn_speed);
        }
    }
}

fn character_walk(
    characters: Query<(
        Entity,
        &Character,
        &CharacterDesiredMovement,
        &mut LinearVelocity,
    )>,
    time: Res<Time>,
    gravity: Res<Gravity>,
    // collisions: Collisions,
) {
    let up_dir = -gravity.0.normalize_or_zero();
    for (char_ent, char, movement, mut velocity) in characters {
        // TODO: Target velocity should be our desired velocity plus the velocity of what we're standing on
        /*
        for manifold in collisions
            .collisions_with(char_ent)
            .flat_map(|pair| pair.manifolds.iter().map(|m| (pair, m)))
            .max_by(|(_, manifold1), (_, manifold2)| {
                manifold1
                    .normal
                    .dot(up_dir)
                    .partial_cmp(manifold2.normal.up_dir)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        {
            let relative_velocity =
        }
        */
        let target_velocity = movement.movement * char.speed;
        let target_velocity = **velocity
            + (target_velocity - **velocity).clamp_length_max(char.accel * time.delta_secs());
        velocity.x = target_velocity.x;
        velocity.z = target_velocity.z;
    }
}
