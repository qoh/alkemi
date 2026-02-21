#![cfg(feature = "dev")]

mod console;

use avian3d::prelude::*;
use bevy::{
    camera::{CameraOutputMode, visibility::RenderLayers},
    color::palettes,
    input::{ButtonState, common_conditions::input_just_pressed, keyboard::KeyboardInput},
    prelude::*,
    render::render_resource::BlendState,
};
use bevy_enhanced_input::{EnhancedInputSystems, prelude::ActionSources};
use bevy_inspector_egui::{
    bevy_egui::{EguiContext, EguiGlobalSettings, EguiPlugin, PrimaryEguiContext},
    quick::WorldInspectorPlugin,
};

// Use MangoHUD for frametime diagnostics.

pub fn plugin(app: &mut App) {
    app.add_plugins(EguiPlugin::default())
        .add_systems(Startup, setup_egui)
        .add_systems(
            PreUpdate,
            suppress_enhanced_input.before(EnhancedInputSystems::Update),
        );

    if !cfg!(feature = "dev_minibuffer") {
        app.init_resource::<InspectorVisible>();
        app.add_plugins(WorldInspectorPlugin::default().run_if(inspector_visible));
        app.add_systems(
            PreUpdate,
            toggle_inspector_visible.run_if(input_just_pressed(KeyCode::F3)),
        );
    }

    app.add_plugins(PhysicsDebugPlugin)
        .add_plugins(PhysicsDiagnosticsPlugin)
        .add_plugins(PhysicsDiagnosticsUiPlugin)
        .add_systems(
            PreUpdate,
            |mut events: MessageReader<KeyboardInput>,
             mut store: ResMut<GizmoConfigStore>,
             mut ui: ResMut<PhysicsDiagnosticsUiSettings>| {
                for event in events.read() {
                    if event.state == ButtonState::Pressed && event.key_code == KeyCode::F7 {
                        let config = store.config_mut::<PhysicsGizmos>();
                        let (enable, show_colliders, hide_meshes) = match (
                            config.0.enabled,
                            config.1.collider_color.is_some(),
                            config.1.hide_meshes,
                        ) {
                            (false, _, _) => (true, false, false),
                            (true, false, _) => (true, true, false),
                            (true, true, false) => (true, true, true),
                            (true, true, true) => (false, false, false),
                        };
                        config.0.enabled = enable;
                        config.1.hide_meshes = hide_meshes;
                        config.1.collider_color = if show_colliders {
                            Some(palettes::css::ORANGE.into())
                        } else {
                            None
                        };
                        ui.enabled = enable;
                    }
                }
            },
        )
        .insert_gizmo_config(
            DefaultGizmoConfigGroup,
            GizmoConfig {
                depth_bias: -0.02,
                line: GizmoLineConfig {
                    width: 4.0,
                    ..default()
                },
                ..default()
            },
        )
        .insert_gizmo_config(
            PhysicsGizmos {
                contact_point_color: Some(palettes::tailwind::AMBER_500.into()),
                contact_normal_color: Some(palettes::tailwind::GREEN_500.into()),
                ..default()
            },
            GizmoConfig {
                enabled: false,
                ..default()
            },
        )
        .insert_resource(PhysicsDiagnosticsUiSettings {
            enabled: false,
            ..default()
        });

    // For Skein and https://github.com/doup/birp
    #[cfg(not(target_family = "wasm"))]
    {
        debug!(
            "BRP HTTP server running at: {}:{}",
            bevy::remote::http::DEFAULT_ADDR,
            bevy::remote::http::DEFAULT_PORT
        );
        // Allow `https://doup.github.io` to access the BRP API
        let cors_headers = bevy::remote::http::Headers::new()
            .insert("Access-Control-Allow-Origin", "https://doup.github.io")
            .insert("Access-Control-Allow-Headers", "Content-Type");
        app.add_plugins((
            bevy::remote::RemotePlugin::default(),
            bevy::remote::http::RemoteHttpPlugin::default().with_headers(cors_headers),
        ));
    }

    #[cfg(feature = "dev_minibuffer")]
    app.add_plugins(minibuffer::plugins);

    #[cfg(feature = "dev_console")]
    app.add_plugins(console::plugin);
}

fn inspector_visible(vis: Res<InspectorVisible>) -> bool {
    vis.0
}

fn toggle_inspector_visible(mut vis: ResMut<InspectorVisible>) {
    vis.0 ^= true;
}

#[derive(bevy::ecs::system::SystemParam)]
pub struct DevToolsInfo<'w> {
    inspector_visible: Option<Res<'w, InspectorVisible>>,
    #[cfg(feature = "dev_minibuffer")]
    minibuffer_prompt: Res<'w, State<bevy_minibuffer::prompt::PromptState>>,
}

impl<'w> DevToolsInfo<'w> {
    pub fn needs_cursor(&self) -> bool {
        let inspector_visible = self
            .inspector_visible
            .as_deref()
            .copied()
            .unwrap_or_default()
            .0;

        #[cfg(feature = "dev_minibuffer")]
        let minibuffer_visible = *self.minibuffer_prompt.get() == PromptState::Visible;
        #[cfg(not(feature = "dev_minibuffer"))]
        let minibuffer_visible = false;

        inspector_visible || minibuffer_visible
    }
}

#[derive(Resource, Clone, Copy, Default)]
struct InspectorVisible(bool);

fn setup_egui(mut commands: Commands, mut egui_global_settings: ResMut<EguiGlobalSettings>) {
    // Disable the automatic creation of a primary context to set it up manually.
    egui_global_settings.auto_create_primary_context = false;
    commands.spawn((
        PrimaryEguiContext,
        Camera2d,
        // Setting RenderLayers to none makes sure we won't render anything apart from the UI.
        RenderLayers::none(),
        Camera {
            order: 1,
            clear_color: ClearColorConfig::None,
            // Prevent flicker when switching main camera
            output_mode: CameraOutputMode::Write {
                blend_state: Some(BlendState::ALPHA_BLENDING),
                clear_color: ClearColorConfig::None,
            },
            ..default()
        },
    ));
}

fn suppress_enhanced_input(
    mut action_sources: ResMut<ActionSources>,
    interactions: Query<&Interaction>,
    mut egui: Query<&mut EguiContext>,
) {
    let bevy_mouse_unused = interactions
        .iter()
        .all(|&interaction| interaction == Interaction::None);

    let egui_mouse_unused = !egui
        .iter_mut()
        .any(|mut ctx| ctx.get_mut().wants_pointer_input());
    let egui_keyboard_unused = !egui
        .iter_mut()
        .any(|mut ctx| ctx.get_mut().wants_keyboard_input());

    let mouse_unused = bevy_mouse_unused && egui_mouse_unused;
    let keyboard_unused = egui_keyboard_unused;

    action_sources.mouse_buttons = mouse_unused;
    action_sources.mouse_wheel = mouse_unused;

    action_sources.keyboard = keyboard_unused;
}

#[cfg(feature = "dev_minibuffer")]
mod minibuffer {
    use bevy_minibuffer::prelude::*;

    pub(super) fn plugin(app: &mut App) {
        app.add_plugins(MinibufferPlugins).add_acts((
            BasicActs::default(),
            bevy_minibuffer_inspector::WorldActs::default().configure(
                "inspect_world",
                |mut act| {
                    act.bind(keyseq! { I W })
                        .add_flags(ActFlags::ShowMinibuffer);
                },
            ),
            Act::new(switch_scene).named("scene"), /*.bind(keyseq! { S S })*/
            Act::new(execute_trigger),
        ));
    }

    fn switch_scene(mut minibuffer: Minibuffer) {
        minibuffer
            .prompt::<TextField>("level containing scene: ")
            .observe(
                |mut trigger: On<Submit<String>>, mut minibuffer: Minibuffer| -> Result {
                    let level = match trigger.event_mut().take_result() {
                        Ok(x) => x,
                        Err(e) => {
                            minibuffer.message(format!("{e}"));
                            return Ok(());
                        }
                    };
                    minibuffer.prompt::<TextField>("scene in level: ").observe(
                        move |mut trigger: On<Submit<String>>,
                              mut minibuffer: Minibuffer,
                              mut commands: Commands|
                              -> Result {
                            let scene = match trigger.event_mut().take_result() {
                                Ok(x) => x,
                                Err(e) => {
                                    minibuffer.message(format!("{e}"));
                                    return Ok(());
                                }
                            };
                            let spawn_point = "start";
                            commands.queue(crate::scene::ChangeScene {
                                level: level.to_owned(),
                                scene: scene.to_owned(),
                                spawn_players: true,
                                spawn_point: Some(spawn_point.to_owned()),
                            });
                            minibuffer.clear();
                            Ok(())
                        },
                    );
                    Ok(())
                },
            );
    }

    fn execute_trigger(
        mut minibuffer: Minibuffer,
        triggers: Query<(Entity, &Name), With<crate::script_triggers::Trigger>>,
    ) {
        let map: std::collections::HashMap<_, _> = triggers
            .iter()
            .map(|(entity, name)| (name.as_str().to_owned(), entity))
            .collect();
        dbg!(&map);
        minibuffer.prompt_map("trigger: ", map).observe(
            |mut trigger: On<Completed<Entity>>,
             mut minibuffer: Minibuffer,
             mut commands: Commands|
             -> Result {
                let trigger = match trigger.event_mut().state.take_result().unwrap() {
                    Ok(x) => x,
                    Err(e) => {
                        minibuffer.message(format!("{e}"));
                        return Ok(());
                    }
                };
                commands.queue(move |world: &mut World| -> Result {
                    world.run_system_cached_with::<_, Result<_, _>, _, _>(
                        crate::script_triggers::execute_trigger,
                        trigger,
                    )?
                });
                minibuffer.clear();
                Ok(())
            },
        );
    }
}
