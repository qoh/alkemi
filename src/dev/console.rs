#![cfg(feature = "dev_console")]

use avian3d::prelude::{PhysicsLayer as _, SpatialQuery, SpatialQueryFilter};
use bevy::prelude::*;
use bevy_console::clap::Parser;
use bevy_console::{
    AddConsoleCommand, ConsoleCommand, ConsoleConfiguration, ConsolePlugin, reply_ok,
};

use crate::camera::PointerRay;
use crate::magicka_level_model::Layers;

pub fn plugin(app: &mut App) {
    app.add_plugins(ConsolePlugin)
        .insert_resource(ConsoleConfiguration {
            block_keyboard: true,
            width: 400.,
            height: 200.,
            ..default()
        })
        .add_console_command::<SceneCommand, _>(scene_command)
        .add_console_command::<TriggerCommand, _>(trigger_command)
        .add_console_command::<SpawnCharacterCommand, _>(spawn_character_command);
}

/// Load a scene.
#[derive(Parser, ConsoleCommand)]
#[command(name = "scene")]
struct SceneCommand {
    /// Name of level containing scene. If omitted, assumes current level.
    #[arg(long)]
    level: Option<String>,

    /// Name of scene to switch to.
    scene: String,
}

fn scene_command(
    mut command: ConsoleCommand<SceneCommand>,
    current_scene: Option<Single<&crate::scene::Scene>>,
    mut commands: Commands,
) {
    let Some(Ok(SceneCommand { level, scene })) = command.take() else {
        return;
    };
    let Some(level) = level.or_else(|| current_scene.map(|s| s.level.clone())) else {
        command.reply_failed("Can't determine level");
        return;
    };
    let spawn_point = "start";
    commands.queue(crate::scene::ChangeScene {
        level,
        scene,
        spawn_players: true,
        spawn_point: Some(spawn_point.to_owned()),
    });
}

/// Activate a scene trigger.
#[derive(Parser, ConsoleCommand)]
#[command(name = "trigger")]
struct TriggerCommand {
    /// The name of the trigger to activate.
    pub name: String,
}

fn trigger_command(
    mut command: ConsoleCommand<TriggerCommand>,
    triggers: Query<(Entity, &Name), With<crate::script_triggers::Trigger>>,
    mut commands: Commands,
) {
    let Some(Ok(TriggerCommand { name })) = command.take() else {
        return;
    };
    let Some((trigger, _)) = triggers.iter().find(|(_, n)| n.as_str() == name) else {
        command.reply_failed("Can't find a trigger with that name");
        return;
    };
    commands.queue(move |world: &mut World| -> Result {
        world.run_system_cached_with::<_, Result<_, _>, _, _>(
            crate::script_triggers::execute_trigger,
            trigger,
        )?
    });
}

#[derive(Parser, ConsoleCommand)]
#[command(name = "spawn-character")]
struct SpawnCharacterCommand {
    /// The name of the character template to spawn.
    pub template: String,
}

fn spawn_character_command(
    mut command: ConsoleCommand<SpawnCharacterCommand>,
    pointer_rays: Query<&PointerRay>,
    spatial_query: SpatialQuery,
    mut commands: Commands,
) {
    let Some(Ok(SpawnCharacterCommand { template })) = command.take() else {
        return;
    };

    let Some(ray) = pointer_rays.iter().find_map(|r| r.current) else {
        command.reply_failed("Can't determine where to spawn character: Cursor direction unknown.");
        return;
    };
    let Some(cast) = spatial_query.cast_ray(
        ray.origin,
        ray.direction,
        f32::MAX,
        false,
        &SpatialQueryFilter::from_mask(!Layers::Trigger.to_bits()),
    ) else {
        command.reply_failed(
            "Can't determine where to spawn character: Cursor target position unknown.",
        );
        return;
    };

    let spawn_transform = Transform::from_translation(ray.get_point(cast.distance));

    commands.queue(move |world: &mut World| {
        let _character = world
            .run_system_cached_with::<_, Result<_>, _, _>(
                crate::character::spawn_character,
                &crate::character::CharacterArgs {
                    type_name: template,
                    spawn_transform,
                    spawn_anchor: crate::character::CharacterAnchorPoint::Bottom,
                    // scene_entity: Some(current_scene_entity),
                    scene_entity: None,
                    model_index: None,
                    start_as_agent: true,
                },
            )
            .unwrap()
            .unwrap();
    });
}
