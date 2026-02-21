#![cfg(feature = "dev_console")]

use bevy::prelude::*;
use bevy_console::clap::Parser;
use bevy_console::{AddConsoleCommand, ConsoleCommand, ConsolePlugin};

pub fn plugin(app: &mut App) {
    app.add_plugins(ConsolePlugin)
        .add_console_command::<SceneCommand, _>(scene_command)
        .add_console_command::<TriggerCommand, _>(trigger_command);
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
