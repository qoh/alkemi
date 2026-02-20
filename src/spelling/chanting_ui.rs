use bevy::{
    camera::ViewportConversionError,
    prelude::*,
    ui::{UiSystems, ui_layout_system},
};

use crate::spelling::{
    chanting::ElementQueue,
    color::{element_color, normalize_color},
    element::Element,
};

pub fn plugin(app: &mut App) {
    app.add_observer(spawn_bar).add_observer(despawn_bar);
    app.add_systems(
        PostUpdate,
        (display_queued_elements_text, display_queued_elements_icon).before(UiSystems::Prepare),
    );
    app.add_systems(
        PostUpdate,
        position_bar
            // Because it needs ComputedUiTargetCamera
            .after(UiSystems::Prepare)
            // Because it needs GlobalTransform of the camera & chanter
            //.after(TransformSystems::Propagate)
            // Because we write the UiTransform that this propagates to UiGlobalTransform
            .before(ui_layout_system),
    );
}

#[derive(Component, Debug, Reflect)]
#[relationship(relationship_target = HasChantingUiBar)]
#[require(Visibility::Hidden, UiTransform)]
struct ChantingUiBar(Entity);

#[derive(Component, Debug, Reflect)]
#[relationship_target(relationship = ChantingUiBar, linked_spawn)]
struct HasChantingUiBar(Entity);

fn spawn_bar(
    event: On<Add, ElementQueue>,
    barless_chanters: Query<&ElementQueue, Without<HasChantingUiBar>>,
    mut commands: Commands,
    assets: Res<AssetServer>,
) {
    let Ok(queue) = barless_chanters.get(event.entity) else {
        return;
    };

    let hud_path = {
        crate::magicka_assets::content_root()
            .join_checked("UI/HUD/hud.xnb")
            .unwrap()
    };
    let hud_path = hud_path.as_ref() as &std::ffi::OsStr;
    let hud_image = assets.load_override(bevy::asset::AssetPath::from_path(hud_path.as_ref()));
    let hud_atlas = assets.add(hud_atlas_layout());

    commands
        .entity(event.entity)
        .with_related::<ChantingUiBar>((
            Node {
                top: px(32),
                height: px(25),
                ..default()
            },
            LayoutConfig {
                // use_rounding: false,
                ..default()
            },
            /*
            Text::default(),
            TextShadow {
                offset: vec2(2., 2.),
                color: Color::BLACK.with_alpha(0.7),
            },
            */
            // XXX: If .limit changes then we won't have the right amount
            Children::spawn((
                /*
                SpawnIter(std::iter::repeat_n(
                    (
                        TextSpan::default(),
                        TextFont {
                            font_size: 20. * 1.25,
                            ..default()
                        },
                    ),
                    queue.limit as usize,
                )),
                */
                SpawnIter(std::iter::repeat_n(
                    element_icon(&hud_image, &hud_atlas),
                    queue.limit as usize,
                )),
            )),
            // HACK: Hide at first to workaround a Bevy bug? Spam this until an element is chanted:
            // ERROR bevy_ui_render::render_pass: Error encountered while rendering the ui phase RenderCommandFailure("missing vertices to draw ui")
            //Visibility::Hidden,
        ));
}

fn element_icon(
    hud_image: &Handle<Image>,
    hud_atlas: &Handle<TextureAtlasLayout>,
) -> impl Bundle + Clone {
    ImageNode {
        image: hud_image.clone(),
        texture_atlas: Some(TextureAtlas {
            layout: hud_atlas.clone(),
            index: 0,
        }),
        ..default()
    }
}

fn hud_atlas_layout() -> TextureAtlasLayout {
    let dim = uvec2(512, 512);
    let mut atlas = TextureAtlasLayout::new_empty(dim);
    let elements_pos = uvec2(0, 156);
    let element_size = uvec2(50, 50);
    for i in 0..=9 {
        let min = elements_pos + uvec2(i % 5, i / 5) * element_size;
        atlas.add_texture(URect {
            min,
            max: min + element_size - uvec2(0, 1),
        });
    }
    atlas
}

fn element_icon_index(element: Element) -> Option<usize> {
    use Element::*;
    Some(match element {
        Earth => 0,
        Water => 1,
        Cold => 2,
        Fire => 3,
        Lightning => 4,
        Arcane => 5,
        Life => 6,
        Shield => 7,
        Ice => 8,
        Steam => 9,
        Poison | Lok => return None, // Poison is kinda index 10
    })
}

fn despawn_bar(event: On<Remove, super::chanting::ElementQueue>, mut commands: Commands) {
    // HACK: For some reason using despawn_related causes panic on scene change
    // And there doesn't seem to be a try_despawn_related.
    //commands.entity(event.entity).despawn_related::<HasChantingUiBar>();
    let chanter = event.entity;
    commands.queue(move |world: &mut World| {
        if let Some(bar_ref) = world.get::<HasChantingUiBar>(chanter) {
            world.despawn(bar_ref.0);
        }
    });
}

type NeedsElementsUpdated = Or<(Added<HasChantingUiBar>, Changed<ElementQueue>)>;

fn display_queued_elements_icon(
    bars: Query<&Children, With<ChantingUiBar>>,
    mut bar_icons: Query<(&mut ImageNode, &mut Visibility)>,
    chanters: Query<(&ElementQueue, &HasChantingUiBar), NeedsElementsUpdated>,
) {
    for (element_queue, bar_target) in chanters {
        let bar = bar_target.0;
        let Ok(bar_children) = bars.get(bar) else {
            continue;
        };
        let mut index = 0;
        for child in bar_children {
            let Ok((mut icon, mut icon_visible)) = bar_icons.get_mut(*child) else {
                continue;
            };
            if let Some(element) = element_queue.queued_elements.get(index).copied() {
                let (icon_index, color) = match element_icon_index(element) {
                    Some(i) => (i, Color::default()),
                    // Magicka 1 lacks icons for poison & lok
                    None => match element {
                        Element::Poison => (
                            element_icon_index(Element::Arcane).unwrap(),
                            Color::linear_rgb(0., 1., 0.),
                        ),
                        Element::Lok => (
                            element_icon_index(Element::Water).unwrap(),
                            Color::linear_rgb(1., 0., 0.),
                        ),
                        _ => unimplemented!(),
                    },
                };
                if let Some(texture_atlas) = &mut icon.texture_atlas {
                    texture_atlas.index = icon_index;
                }
                icon.color = color;
                icon_visible.set_if_neq(Visibility::Inherited);
            } else {
                icon_visible.set_if_neq(Visibility::Hidden);
            }
            index += 1;
        }
    }
}

fn display_queued_elements_text(
    chanters: Query<(&ElementQueue, &HasChantingUiBar), NeedsElementsUpdated>,
    mut text_writer: TextUiWriter,
) {
    for (element_queue, bar_target) in chanters {
        let bar = bar_target.0;
        let mut index = 0;
        text_writer.for_each(
            bar,
            |_span, depth, mut text, _font, mut color, _line_height| {
                if depth != 1 {
                    return;
                }
                text.clear();
                if let Some(element) = element_queue.queued_elements.get(index).copied() {
                    text.push(element_abbr(element));
                    **color = normalize_color(element_color(element)).0.into();
                } else {
                    text.push(' ');
                    **color = Color::WHITE;
                }
                assert!(text.is_changed());
                index += 1;
            },
        );
    }
    use super::element::Element;
    fn element_abbr(element: Element) -> char {
        use Element::*;
        match element {
            Water => 'Q',
            Life => 'W',
            Shield => 'E',
            Cold => 'R',
            Lightning => 'A',
            Arcane => 'S',
            Earth => 'D',
            Fire => 'F',
            Steam => 't',
            Ice => 'I',
            Poison => 'P',
            Lok => 'L',
        }
    }
}

fn position_bar(
    bars: Query<(
        &mut UiTransform,
        &mut Visibility,
        &ChantingUiBar,
        // FIXME: ComputedNode is a frame behind
        // Maybe ComputedUiTargetCamera is too?
        &ComputedUiTargetCamera,
        &ComputedNode,
    )>,
    // chanters: Query<&GlobalTransform, With<HasChantingUiBar>>,
    chanters: Query<&ElementQueue>,
    cameras: Query<(&Camera /*&GlobalTransform*/,)>,
    // windows: Query<&Window>,
    transform_helper: TransformHelper,
) -> Result {
    for (mut bar_ui_transform, mut bar_visibility, bar, bar_camera_target, bar_computed) in bars {
        let chanter = bar.0;
        /* let Ok(chanter_transform) = chanters.get(chanter) else {
            continue;
        }; */
        let chanter_transform = transform_helper.compute_global_transform(chanter)?;
        let Some(bar_camera_target) = bar_camera_target.get() else {
            debug_once!("chanting ui bar has no camera target");
            bar_visibility.set_if_neq(Visibility::Hidden);
            continue;
        };
        let Ok((camera /*camera_transform*/,)) = cameras.get(bar_camera_target) else {
            warn_once!("chanting ui bar camera target not matched");
            continue;
        };
        let camera_transform = &transform_helper.compute_global_transform(bar_camera_target)?;

        let anchor_world =
            chanter_transform.translation() - Vec3::Y * 1. * chanter_transform.scale();
        // WISH: Use _with_depth to sort
        let anchor_viewport = match camera.world_to_viewport(camera_transform, anchor_world) {
            Err(
                ViewportConversionError::PastNearPlane
                | ViewportConversionError::PastFarPlane
                | ViewportConversionError::InvalidData,
            ) => None,
            r @ Ok(_) | r @ Err(ViewportConversionError::NoViewportSize) => Some(r?),
        };

        let desired_top = anchor_viewport;
        // Offset the viewport position to center the node
        let top_left = desired_top.map(|desired_top| {
            let pretend_bounds = Rect::from_center_size(
                desired_top,
                bar_computed.size() * bar_computed.inverse_scale_factor(),
            );
            vec2(pretend_bounds.min.x, desired_top.y)
        });

        let desired_visibility = if let Some(top_left) = top_left {
            bar_ui_transform.translation = Val2::px(top_left.x, top_left.y);
            Visibility::default()
        } else {
            Visibility::Hidden
        };
        // TODO: Should this also copy the chanter's Visibility?

        // HACK: Hide if no elements to workaround a Bevy bug? Spam this until an element is chanted:
        //  ERROR bevy_ui_render::render_pass: Error encountered while rendering the ui phase RenderCommandFailure("missing vertices to draw ui")
        let has_no_elements = chanters
            .get(chanter)
            .ok()
            .is_none_or(|q| q.queued_elements.is_empty());
        let desired_visibility = if has_no_elements {
            Visibility::Hidden
        } else {
            desired_visibility
        };

        bar_visibility.set_if_neq(desired_visibility);
    }
    Ok(())
}
