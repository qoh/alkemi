use super::map_vec3;
use bevy::{ecs::relationship::RelatedSpawnerCommands, light::light_consts, prelude::*};
use remagic::xnb_readers::magicka_content::LightData;

/// EV_100
pub const MAGICKA_EXPOSURE: f32 = bevy::camera::Exposure::EV100_BLENDER;

/// Lumens
pub const MAGICKA_TO_LUMINOUS_INTENSITY: f32 = 0.1 * light_consts::lumens::VERY_LARGE_CINEMA_LIGHT;

/// Lux (lumens per square meter)
pub const MAGICKA_TO_ILLUMINANCE: f32 = 0.5 * light_consts::lux::AMBIENT_DAYLIGHT;

/// Candela per square meter
pub const MAGICKA_TO_LUMINANCE: f32 = 1200.; // This looks pretty good!

// XXX: This should not be in the Magicka data loading module
pub fn plugin(app: &mut App) {
    app.insert_resource(AmbientLight {
        brightness: MAGICKA_TO_LUMINANCE,
        ..default()
    });
    app.add_systems(
        PostUpdate,
        apply_ambient_light_source
            .after(bevy::camera::visibility::VisibilitySystems::VisibilityPropagate),
    );
}
#[derive(Component, Debug)]
pub struct AmbientLightSource {
    pub color: Vec3,
}
fn apply_ambient_light_source(
    ambient_lights: Query<
        (&AmbientLightSource, Option<&InheritedVisibility>),
        With<DirectionalLight>,
    >,
    mut ambient_color: ResMut<AmbientLight>,
) {
    let mut ambient_lights = ambient_lights
        .iter()
        .filter(|(_, vis)| vis.copied().as_deref().copied().unwrap_or(true))
        .map(|(x, _vis)| x);
    let ambient_light = ambient_lights.next();
    if ambient_light.is_some() && ambient_lights.next().is_some() {
        warn_once!("There are multiple visible ambient light sources");
    }

    let color = ambient_light.map(|c| c.color).unwrap_or(Vec3::ZERO);
    ambient_color.color = LinearRgba::from_vec3(color).into();
}

pub(crate) fn spawn_light(
    parent: &mut RelatedSpawnerCommands<ChildOf>,
    light: &remagic::xnb_readers::magicka_content::Light,
) -> Entity {
    if !matches!(
        light.variation_type,
        remagic::xnb_readers::magicka_content::LightVariationType::None,
    ) {
        // When implementing light variation, remember to add support to AmbientLightSource/AmbientLight
        info!(
            "Unhandled light variation type on {:?}: {:?} (amount {}, speed {})",
            light.name, light.variation_type, light.variation_amount, light.variation_speed
        );
    }

    let transform = match &light.data {
        LightData::Point { position, .. } => Transform::from_translation(map_vec3(*position)),
        LightData::Directional { direction } => {
            let mut trans = Transform::IDENTITY;
            if let Ok(direction) = Dir3::new(map_vec3(*direction)) {
                trans.look_to(direction, Vec3::Y);
            }
            trans
        }
        LightData::Spot {
            position,
            direction,
            ..
        } => {
            let mut trans = Transform::from_translation(map_vec3(*position));
            if let Ok(direction) = Dir3::new(map_vec3(*direction)) {
                trans.look_to(direction, Vec3::Y);
            }
            trans
        }
    };
    let mut entity_commands = parent.spawn((
        Name::new(light.name.clone()),
        transform,
        AmbientLightSource {
            color: map_vec3(light.ambient_color),
        },
    ));
    let color_vec = map_vec3(light.diffuse_color);
    let (color_vec, magnitude) = color_vec.normalize_and_length();
    let diffuse_color = Color::linear_rgb(color_vec.x, color_vec.y, color_vec.z);
    match &light.data {
        LightData::Point {
            position: _,
            radius,
        } => {
            entity_commands.insert((PointLight {
                color: diffuse_color,
                intensity: magnitude * MAGICKA_TO_LUMINOUS_INTENSITY,
                range: *radius, // TODO: range
                radius: 0.,     // TODO radius
                shadows_enabled: light.cast_shadows,
                ..default()
            },))
        }
        LightData::Directional { direction: _ } => entity_commands.insert((
            DirectionalLight {
                color: diffuse_color,
                illuminance: magnitude * MAGICKA_TO_ILLUMINANCE,
                shadows_enabled: light.cast_shadows,
                ..default()
            },
            {
                let camera_offset_distance = 223.55536;
                bevy::light::CascadeShadowConfigBuilder {
                    minimum_distance: camera_offset_distance * 0.8,
                    first_cascade_far_bound: camera_offset_distance * 1.1,
                    maximum_distance: camera_offset_distance * 1.5,
                    ..default()
                }
                .build()
            },
        )),
        LightData::Spot {
            position: _,
            range,
            direction: _,
            cutoff_angle,
            sharpness,
            use_attenuation,
        } => entity_commands.insert(SpotLight {
            color: diffuse_color,
            intensity: magnitude * MAGICKA_TO_LUMINOUS_INTENSITY,
            range: 20., // TODO: range
            radius: 0., // TODO radius
            shadows_enabled: light.cast_shadows,
            outer_angle: *cutoff_angle,
            inner_angle: *cutoff_angle * *sharpness,
            ..default()
        }),
    }
    .id()
}
