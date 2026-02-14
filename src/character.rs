use crate::gameplay::damage::Health;
use crate::magicka_level_model::Locator;
use crate::{camera::CameraGroupMember, magicka_assets::skinned_model::AnimationLibrary};
use avian3d::prelude::*;
use bevy::ecs::system::SystemState;
use bevy::input::InputSystems;
use bevy::prelude::*;
use remagic::xnb_readers::magicka_character::CharacterTemplate;
use std::ffi::OsStr;
use typed_path::{PlatformPath, PlatformPathBuf};

pub fn plugin(app: &mut App) {
    app.add_systems(PreUpdate, player_walk.after(InputSystems));
    app.add_systems(
        FixedUpdate,
        agent_walk
            .after(bevy_landmass::LandmassSystems::Output)
            .before(character_walk),
    );
    app.add_systems(FixedUpdate, character_walk);
    app.add_systems(
        PostUpdate,
        model_face_dir.before(TransformSystems::Propagate),
    );
    app.add_systems(
        PostUpdate,
        (
            select_animation,
            character_play_animation.before(bevy::app::AnimationSystems),
        )
            .chain(),
    );
    app.add_systems(PostUpdate, copy_skinnedmesh_from_source);
}

#[derive(Component, Debug)]
pub struct Character {
    pub type_name: String,
    pub speed: f32,
    pub accel: f32,
    pub turn_speed: f32,
    pub nav_radius: f32,
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

#[derive(Component, Debug, Reflect)]
pub struct FaceParentDir;

pub(crate) fn spawn_follower(
    (In(level_entity), InRef(spawn_point_basename), In(target_entity)): (
        In<Option<Entity>>,
        InRef<str>,
        In<Entity>,
    ),
    world: &mut World,
    locators: &mut QueryState<(&Name, Entity), With<Locator>>,
    global_transforms: &mut SystemState<TransformHelper>,
) -> Result {
    let player_index = 1;

    let (_, spawn_entity) = locators
        .query(world)
        .iter()
        .find(|(n, _)| n.as_str() == format!("{}{}", spawn_point_basename, player_index))
        .unwrap();
    let spawn_transform = Transform::from(
        global_transforms
            .get(world)
            .compute_global_transform(spawn_entity)?,
    );

    let follower = world.run_system_cached_with::<_, Result<_>, _, _>(
        spawn_character,
        &CharacterArgs {
            type_name: "Wizard_Reddit".to_owned(),
            spawn_transform,
            scene_entity: level_entity,
            model_index: None,
            start_as_agent: true,
        },
    )??;

    let mut character_world = world.entity_mut(follower);
    let mut character_component = character_world.get_mut::<Character>().unwrap();
    character_component.accel = 12.;

    let Character {
        speed, nav_radius, ..
    } = *character_component;

    world.entity_mut(follower).insert((
        Name::new("Follower"),
        // CameraGroupMember,
        bevy_landmass::AgentTarget3d::Entity(target_entity),
    ));

    Ok(())
}

pub(crate) fn spawn_player_character(
    (In(level_entity), InRef(spawn_point_basename)): (In<Option<Entity>>, InRef<str>),
    world: &mut World,
    locators: &mut QueryState<(&Name, Entity), With<Locator>>,
    global_transforms: &mut SystemState<TransformHelper>,
) -> Result<Entity> {
    let player_index = 0;

    let (_, spawn_entity) = locators
        .query(world)
        .iter()
        .find(|(n, _)| n.as_str() == format!("{}{}", spawn_point_basename, player_index))
        .unwrap();
    let spawn_transform = Transform::from(
        global_transforms
            .get(world)
            .compute_global_transform(spawn_entity)?,
    );

    let player_entity = world.run_system_cached_with::<_, Result<_>, _, _>(
        spawn_character,
        &CharacterArgs {
            type_name: "Wizard".to_owned(),
            spawn_transform,
            scene_entity: level_entity,
            model_index: None,
            start_as_agent: false,
        },
    )??;

    let mut character_world = world.entity_mut(player_entity);
    let mut character_component = character_world.get_mut::<Character>().unwrap();
    character_component.type_name = "wizard".to_string(); // For scene triggers

    let nav_radius = character_component.nav_radius;

    world.entity_mut(player_entity).insert((
        Name::new(format!("Player {player_index}")),
        crate::spelling::bundle_m1(),
        crate::spelling::bindings_m1(),
        CameraGroupMember,
        crate::PlayerControlled,
    ));

    Ok(player_entity)
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
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
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
        ref skinned_models,
        ref animation_sets,
    } = template;

    let model_index = model_index.unwrap_or(0); // TODO: random

    let full_height = length + radius * 2.;

    // HACK?: It would easier to rotate the root with the collider itself,
    // but that seems to cause jittery movement
    // Maybe Avian thinks I'm teleporting the character when setting
    let mut phys_transform = *spawn_transform;
    phys_transform.rotation = Default::default();
    let vis_transform = Transform::from_rotation(spawn_transform.rotation);

    let nav_radius = full_height * 0.5;
    let mut player = commands.spawn((
        Name::new("Character"),
        phys_transform,
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
            nav_radius,
        },
        CharacterDesiredMovement::default(),
    ));
    let player_entity = player.id();
    if let Some(level_entity) = level_entity {
        player.insert(ChildOf(*level_entity));
    }
    if let Some(archipelago) = nav_mesh_archipelago {
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

    // Attach invisible shared "skeleton" model that actually plays all the animations
    let skeleton_scene = load_model(
        &mut meshes,
        &mut materials,
        &assets,
        content_path.as_path(),
        skinned_models.1.path.as_str(),
    );
    let skeleton_ent = player
        .commands_mut()
        .spawn((
            ChildOf(player_entity),
            Transform::from_translation(Vec3::Y * -0.5 * full_height) * vis_transform,
            FaceParentDir,
            SceneRoot(skeleton_scene),
            Visibility::Hidden,
        ))
        .id();
    player.insert(CharacterAnimationState {
        base_animation: "idle",
        force_animation: None,
        animation_skeleton: skeleton_ent,
        currently_playing: None,
        animation_set_index: 0,
        // HACK: The character should really refer to the character template with an asset handle instead
        animation_sets: animation_sets.clone(),
    });

    // Attach visible model that reflects the animation of the invisible one
    let visual_scene = load_model(
        &mut meshes,
        &mut materials,
        &assets,
        content_path.as_path(),
        skinned_models.0[model_index].0.path.as_str(),
    );
    player.with_child((SceneRoot(visual_scene), CopiesSkinnedMeshFrom(skeleton_ent)));

    Ok(player_entity)
}

fn load_model(
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    assets: &AssetServer,
    content_path: &PlatformPath,
    relative_path: &str,
) -> Handle<Scene> {
    let relative_path = typed_path::WindowsPathBuf::from(relative_path);
    let model_content_path = content_path.parent().unwrap().join(
        relative_path
            .with_platform_encoding_checked()
            .unwrap()
            .as_bytes(),
    );
    let mut model_path = crate::magicka_assets::content_root()
        .join_checked(&model_content_path)
        .unwrap();
    model_path.set_extension("xnb");

    let model_path = std::path::Path::new(model_path.as_ref() as &OsStr);
    let bytes = crate::magicka_assets::read_ignore_path_ascii_case(model_path).unwrap();
    let xnb_asset = remagic::parse_skinned_model(&bytes)
        .map_err(|e| e.into_inner())
        .unwrap();
    let skinned_mesh = xnb_asset.inner().as_ref().unwrap();

    assets.add(crate::magicka_assets::skinned_model::load_skinned_model(
        skinned_mesh,
        &xnb_asset,
        model_content_path.as_ref(),
        meshes,
        materials,
        &assets,
    ))
}

/// All descendant SkinnedMesh components will be replaced once with
/// the first descendant SkinnedMesh component on the target entity.
#[derive(Component, Debug)]
#[relationship(relationship_target = CopiesSkinnedMeshTo)]
struct CopiesSkinnedMeshFrom(Entity);

#[derive(Component, Debug)]
#[relationship_target(relationship = CopiesSkinnedMeshFrom)]
struct CopiesSkinnedMeshTo(Vec<Entity>);

fn copy_skinnedmesh_from_source(
    sources: Query<Option<&bevy::scene::SceneInstance>, With<CopiesSkinnedMeshTo>>,
    targets: Query<(
        Entity,
        &CopiesSkinnedMeshFrom,
        Option<&bevy::scene::SceneInstance>,
    )>,
    mut skinned_meshes: Query<&mut bevy::mesh::skinning::SkinnedMesh>,
    children: Query<&Children>,
    scene_spawner: Res<SceneSpawner>,
    mut commands: Commands,
) {
    for (target, target_source, target_scene) in targets {
        // If the target is an instanced scene, wait for it to become ready
        if let Some(instance) = target_scene
            && !scene_spawner.instance_is_ready(**instance)
        {
            continue;
        }

        let source = target_source.0;
        let source_scene = sources.get(source).unwrap();
        // If the source is an instanced scene, wait for it to become ready
        if let Some(instance) = source_scene
            && !scene_spawner.instance_is_ready(**instance)
        {
            continue;
        }

        // Grab the first available SkinnedMesh in tree order
        let mut source_skinned_mesh = None;
        for child in children.iter_descendants(source) {
            if let Ok(skinned_mesh) = skinned_meshes.get(child) {
                source_skinned_mesh = Some(skinned_mesh.clone());
                break;
            }
        }

        // If there was none, sad
        let Some(source_skinned_mesh) = source_skinned_mesh else {
            warn_once!("No skinned mesh config to copy from source");
            // Give up
            commands.entity(target).remove::<CopiesSkinnedMeshFrom>();
            continue;
        };

        // Replace all target SkinnedMeshes with it
        for child in children.iter_descendants(target) {
            if let Ok(mut skinned_mesh) = skinned_meshes.get_mut(child) {
                *skinned_mesh = source_skinned_mesh.clone();
            }
        }

        // Success, don't check anymore
        commands.entity(target).remove::<CopiesSkinnedMeshFrom>();
    }
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

fn model_face_dir(
    facers: Query<(&mut Transform, &ChildOf), (With<FaceParentDir>, Without<Character>)>,
    mut parents: Query<(&mut Transform, &CharacterDesiredMovement, &Character)>,
    time: Res<Time>,
) {
    for (mut transform, parent) in facers {
        if let Ok((mut parent_trans, movement, char)) = parents.get_mut(parent.parent()) {
            let (dir, speed_scale) = movement.movement.normalize_and_length();
            if speed_scale > 0.01 {
                let want_rot = Quat::look_to_rh(dir, Vec3::Y).inverse();
                // Would like to set parent_trans instead, simpler
                // let max_angle
                transform.rotation = transform
                    .rotation
                    .interpolate_stable(&want_rot, time.delta_secs() * char.turn_speed);
            }
        }
    }
}

fn agent_walk(
    agents: Query<(
        &bevy_landmass::AgentDesiredVelocity3d,
        &Character,
        &mut CharacterDesiredMovement,
    )>,
) {
    for (target_velocity, char, mut movement) in agents {
        movement.movement =
            (target_velocity.velocity().with_y(0.) / char.speed).clamp_length_max(1.);
    }
}

fn player_walk(
    players: Query<&mut CharacterDesiredMovement, (With<crate::PlayerControlled>)>,
    kb_input: Res<ButtonInput<KeyCode>>,
) {
    let mut direction = Vec2::ZERO;
    if kb_input.pressed(KeyCode::KeyW) {
        direction.y -= 1.;
    }
    if kb_input.pressed(KeyCode::KeyS) {
        direction.y += 1.;
    }
    if kb_input.pressed(KeyCode::KeyA) {
        direction.x -= 1.;
    }
    if kb_input.pressed(KeyCode::KeyD) {
        direction.x += 1.;
    }

    let direction = direction.normalize_or_zero().extend(0.).xzy();

    for mut movement in players {
        movement.movement = direction;
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
