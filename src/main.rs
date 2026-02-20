mod camera;
mod character;
mod components_basic;
mod dev;
mod gameplay;
mod magicka_assets;
mod magicka_level_model;
mod magicka_scene;
mod scene;
mod script_triggers;
mod spelling;

use avian3d::prelude::*;
use bevy::{
    input::common_conditions::{input_just_pressed, input_pressed},
    prelude::*,
    window::{CursorGrabMode, CursorOptions},
};
use bevy_asset_loader::prelude::*;
use bevy_seedling::prelude::*;

fn main() -> AppExit {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins.set(AssetPlugin {
            unapproved_path_mode: bevy::asset::UnapprovedPathMode::Deny,
            ..default()
        }),
        PhysicsPlugins::default(),
        bevy_enhanced_input::EnhancedInputPlugin,
        SeedlingPlugin::default(),
        bevy_hanabi::HanabiPlugin,
    ))
    .register_type_data::<TrimeshFlags, ReflectDeserialize>();
    app.init_state::<LoadState>().add_loading_state(
        LoadingState::new(LoadState::Loading).continue_to_state(LoadState::Loaded),
    );
    app.add_plugins((
        magicka_assets::plugin,
        components_basic::plugin,
        magicka_level_model::plugin,
        camera::plugin,
        character::plugin,
        script_triggers::plugin,
        gameplay::plugin,
        spelling::plugin,
    ));
    app.add_systems(PreUpdate, update_cursor_grab);

    setup_scenes(&mut app);

    #[cfg(feature = "dev")]
    app.add_plugins(dev::plugin);

    app.run()
}

fn setup_scenes(app: &mut App) {
    // Check for transition
    app.add_systems(PreUpdate, scene::change_to_next_scene);
    // Load into a scene on startup
    app.add_systems(Startup, |mut commands: Commands| {
        commands.queue(crate::scene::StartScene {
            level: "WizardCastle".to_owned(),
            scene: "wc_s1".to_owned(),
            spawn_players: true,
            spawn_point: Some("start".to_owned()),
        });
    });
    // Keybinds for switching scene
    for (level, scene, input) in [
        ("WizardCastle", "wc_s1", KeyCode::Digit1),
        ("WizardCastle", "wc_s2", KeyCode::Digit2),
        ("WizardCastle", "wc_s3", KeyCode::Digit3),
        ("WizardCastle", "wc_s4", KeyCode::Digit4),
        ("WizardCastle", "wc_s5", KeyCode::Digit5),
        ("Highlands", "hl_s2", KeyCode::Digit6),
    ] {
        app.add_systems(
            PreUpdate,
            (|mut commands: Commands| {
                commands.queue(crate::scene::ChangeScene {
                    level: level.to_owned(),
                    scene: scene.to_owned(),
                    spawn_players: true,
                    spawn_point: Some("start".to_owned()),
                });
            })
            .run_if(input_just_pressed(input).and(input_pressed(KeyCode::AltLeft))),
        );
    }

    // Notable scenes:
    // "Levels/WizardCastle/wc_s1.xnb", // Game start
    // "Levels/WizardCastle/wc_s2.xnb", // Party
    // "Levels/WizardCastle/wc_s3.xnb", // Tutorial
    // "Levels/WizardCastle/wc_s4.xnb", // Exiting castle
    // "Levels/WizardCastle/wc_s5.xnb", // Settlement & arena
    // "Levels/Havindr/havindr_s1.xnb", // Animated horse carriage
    // "Levels/Havindr/havindr_s4.xnb", // Grinder
    // "Levels/Havindr/havindr_s6.xnb", // Fuse
    // "Levels/Highlands/hl_s2.xnb", // Airship ride
    // "Levels/Highlands/hl_s3.xnb", // Knights who say Ni
    // "Levels/BattleField/bf_s1.xnb", // Sparta hole
    // "Levels/BattleField/bf_s3.xnb", // Fort siege
    // "Levels/Ruins/ru_s5.xnb", // LavaEffect
    // "Levels/MountainDale/md_s4.xnb",
    // "Levels/EndofWorld/ew_s2.xnb", // Moving island platforms
    // "Levels/EndofWorld/ew_s4.xnb", // Contains LavaEffect
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, States, Debug)]
pub(crate) enum LoadState {
    #[default]
    Loading,
    Loaded,
}

#[derive(Component)]
pub struct PlayerControlled;

#[allow(clippy::type_complexity)]
fn update_cursor_grab(
    windows: Single<
        (&mut CursorOptions, &Window),
        (With<bevy::window::PrimaryWindow>, Changed<Window>),
    >,
    #[cfg(feature = "dev")] dev_tools: dev::DevToolsInfo,
) {
    let (mut cursor_options, window) = windows.into_inner();
    let focused = window.focused;

    #[cfg(feature = "dev")]
    let needs_cursor = dev_tools.needs_cursor();
    #[cfg(not(feature = "dev"))]
    let needs_cursor = false;

    let grab = focused && !needs_cursor;

    cursor_options.grab_mode = if grab {
        CursorGrabMode::Confined
    } else {
        CursorGrabMode::None
    };
}
