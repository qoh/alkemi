use std::{fs::File, io::BufReader};

use bevy::prelude::*;
use typed_path::{PlatformPath, PlatformPathBuf};

use crate::{
    magicka_scene::{SceneConfig, Trigger, read_scene},
    script_triggers::{ActionState, AutoEvaluateCriterion, TriggerAutoEvaluate},
};

#[derive(Component, Debug, Reflect)]
pub struct Scene {
    pub level: String,
    pub scene: String,
}

pub fn queue_delayed_scene_change(In(change_command): In<ChangeScene>, world: &mut World) {
    world.insert_resource(NextScene { change_command });
}

/// Used to delay scene change from the ChangeScene trigger action
/// until the next frame (escape the trigger processing stack)
#[derive(Resource, Debug)]
struct NextScene {
    change_command: ChangeScene,
}

pub fn change_to_next_scene(world: &mut World) {
    if let Some(next_scene) = world.remove_resource::<NextScene>() {
        next_scene.change_command.apply(world);
    }
}

#[derive(Debug)]
pub struct ChangeScene {
    pub level: String,
    pub scene: String,
    pub spawn_players: bool,
    pub spawn_point: Option<String>,
}

impl Command for ChangeScene {
    fn apply(self, world: &mut World) {
        let existing_scenes: Vec<_> = world
            .query_filtered::<Entity, With<Scene>>()
            .iter(world)
            .collect();
        for scene in existing_scenes {
            world.despawn(scene);
        }
        let start_command = StartScene {
            level: self.level,
            scene: self.scene,
            spawn_players: self.spawn_players,
            spawn_point: self.spawn_point,
        };
        start_command.apply(world);
    }
}

#[derive(Debug)]
pub struct StartScene {
    pub level: String,
    pub scene: String,
    pub spawn_players: bool,
    pub spawn_point: Option<String>,
}

impl Command for StartScene {
    fn apply(self, world: &mut World) {
        use std::ffi::OsStr;

        // Locate the file containing scene config
        let mut scene_content_path = ["Levels", &self.level, &self.scene]
            .iter()
            .collect::<PlatformPathBuf>();
        scene_content_path.set_extension("xml");

        // Read in the scene config
        let scene_full_path = crate::magicka_assets::content_root()
            .join_checked(&scene_content_path)
            .unwrap();
        let file = BufReader::new(File::open(scene_full_path.as_ref() as &OsStr).unwrap());
        let scene = read_scene(file).unwrap();

        // Spawn an entity to hold the scene
        let scene_entity = world
            .spawn((
                Name::new(format!("Scene - {} of level {}", &self.scene, &self.level)),
                Scene {
                    level: self.level.clone(),
                    scene: self.scene.clone(),
                },
                Transform::default(),
                Visibility::default(),
            ))
            .id();

        let spawn_result =
            self.spawn_scene_into(world, scene_entity, scene, scene_content_path.as_path());
        if spawn_result.is_err() {
            world.despawn(scene_entity);
        }
        spawn_result.unwrap();
    }
}

impl StartScene {
    fn spawn_scene_into(
        self,
        world: &mut World,
        parent_entity: Entity,
        scene: SceneConfig,
        scene_path: &PlatformPath,
    ) -> Result<(), ()> {
        // Spawn the scene's level model
        let level_model = scene.model.as_ref().unwrap();
        let mut level_model_path = scene_path.to_owned();
        level_model_path.pop();
        level_model_path.push(level_model);
        level_model_path.set_extension("xnb");
        let level_entity = world
            .run_system_cached_with::<_, Result<Entity>, _, _>(
                crate::magicka_level_model::spawn_level,
                level_model_path.as_path(),
            )
            .unwrap()
            .unwrap();
        world.entity_mut(parent_entity).add_child(level_entity);

        if self.spawn_players
            && let Some(spawn_point) = self.spawn_point
        {
            let player_entity = world
                .run_system_cached_with::<_, Result<_>, _, _>(
                    crate::character::spawn_player_character,
                    (Some(parent_entity), &spawn_point),
                )
                .unwrap()
                .unwrap();
            world
                .run_system_cached_with::<_, Result<_>, _, _>(
                    crate::character::spawn_follower,
                    (Some(parent_entity), &spawn_point, player_entity),
                )
                .unwrap()
                .unwrap();
        }

        // Set up triggers
        world.entity_mut(parent_entity).with_children(|parent| {
            for (priority, trigger) in scene.triggers.into_iter().enumerate() {
                let mut trigger_commands = parent.spawn(crate::script_triggers::Trigger {
                    priority,
                    conditions: trigger.conditions,
                    actions: trigger
                        .actions
                        .into_iter()
                        .map(|d| (d, ActionState::default()))
                        .collect(),
                });
                if let Some(name) = trigger.id {
                    trigger_commands.insert(Name::new(name.clone()));
                }
                if trigger.autorun {
                    let criterion = if let Some(interval) = trigger.repeat {
                        let mut timer = Timer::from_seconds(interval, TimerMode::Once);
                        // Ensure it runs on first tick
                        timer.finish();
                        AutoEvaluateCriterion::Every(timer)
                    } else {
                        AutoEvaluateCriterion::Once(false)
                    };
                    trigger_commands.insert(TriggerAutoEvaluate { criterion });
                }
            }
        });

        Ok(())
    }
}
