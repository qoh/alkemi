use std::{io::Read, str::FromStr};

use bevy::{
    asset::{AssetLoader, LoadContext},
    prelude::*,
};
use bevy_hanabi::{EffectAsset, ParticleEffect};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use xml::{EventReader, ParserConfig, reader::XmlEvent};

#[derive(Asset, Reflect, Debug)]
#[reflect(from_reflect = false)]
pub struct VisualEffect {
    pub scene: Handle<Scene>,
}

#[derive(Default)]
pub(crate) struct VisualEffectLoader;

impl AssetLoader for VisualEffectLoader {
    type Asset = VisualEffect;

    type Settings = VisualEffectLoaderSettings;

    type Error = VisualEffectLoaderError;

    async fn load(
        &self,
        reader: &mut dyn bevy::asset::io::Reader,
        _settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> std::result::Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;

        let mut xml = xml::EventReader::new_with_config(
            bytes.as_slice(),
            ParserConfig::new().allow_multiple_root_elements(false),
        );

        let parsed = parse(&mut xml)?;

        let scene = load_context
            .labeled_asset_scope::<_, ()>("scene".to_owned(), |load_context| {
                Ok(create_scene(&parsed, load_context))
            })
            .expect("does not error");

        let asset = VisualEffect { scene };

        Ok(asset)
    }

    fn extensions(&self) -> &[&str] {
        &["xml"]
    }
}

/// An error when loading a visual effect using [`VisualEffectLoader`].
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum VisualEffectLoaderError {
    /// An error occurred while trying to load the file bytes.
    #[error("Failed to load file bytes: {0}")]
    Io(#[from] std::io::Error),
    /// An error occurred while trying to parse the file XML.
    #[error("Failed to deserialize XML: {0}")]
    Xml(#[from] xml::reader::Error),
    #[error("Failed to parse visual effect data: {0}")]
    Parse(&'static str),
}

/// Settings for loading a [`VisualEffect`] using [`VisualEffectLoader`].
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct VisualEffectLoaderSettings;

fn create_scene(effect: &Effect, load_context: &mut LoadContext) -> Scene {
    let mut world = World::default();

    // TODO:
    // If effect_type is Single, then the timeline pos is lifetime, and the effect should stop when lifetime exceeds duration
    // If effect_type is Looping, then the timeline pos is lifetime % duration
    // If effect_type is Infinite, then the timeline pos is min(lifetime, duration)

    for (i, emitter) in effect.emitters.iter().enumerate() {
        let emitter_asset = load_context
            .labeled_asset_scope::<_, ()>(format!("emitter{i}"), |load_context| {
                Ok(create_emitter_effect(emitter, load_context))
            })
            .expect("does not error");
        world.spawn((
            // TODO: Use the name from the .xml
            Name::new(format!("emitter{i}")),
            ParticleEffect::new(emitter_asset),
        ));
    }
    for _light in &effect.lights {
        // TODO: Spawn light
    }

    Scene::new(world)
}

fn create_emitter_effect(emitter: &Emitter, load_context: &mut LoadContext) -> EffectAsset {
    use bevy_hanabi::prelude::*;

    let writer = ExprWriter::new();

    let init_pos = SetPositionCircleModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        axis: writer.lit(Vec3::Y).expr(),
        radius: writer.lit(30.).expr(),
        dimension: ShapeDimension::Volume,
    };

    let zero = writer.lit(0.);
    let y = writer.lit(140.).uniform(writer.lit(160.));
    let v = zero.clone().vec3(y, zero);
    let init_vel = SetAttributeModifier::new(Attribute::VELOCITY, v.expr());

    let age = writer.lit(0.).expr();
    let init_age = SetAttributeModifier::new(Attribute::AGE, age);

    let lifetime = writer.lit(0.8).uniform(writer.lit(1.2)).expr();
    let init_lifetime = SetAttributeModifier::new(Attribute::LIFETIME, lifetime);

    let spawner = SpawnerSettings::rate(10.0.into());
    EffectAsset::new(32, spawner, writer.finish())
        // .with_name("")
        .init(init_pos)
        .init(init_vel)
        .init(init_age)
        .init(init_lifetime)
}

#[derive(Debug, Clone)]
pub struct Effect {
    pub effect_type: EffectType,
    pub duration: f32,
    // pub keyframes_per_second: i32,
    // pub version: i32,
    pub emitters: Vec<Emitter>,
    pub lights: Vec<Light>,
}

#[derive(Debug, Clone)]
pub enum Emitter {
    Continuous,
    Pulse,
}

#[derive(Debug, Clone)]
pub struct Light;

fn parse(xml: &mut EventReader<impl Read>) -> Result<Effect, VisualEffectLoaderError> {
    let attributes = loop {
        match xml.next()? {
            XmlEvent::EndDocument => return Err(VisualEffectLoaderError::Parse("No root element")),
            XmlEvent::StartElement {
                name,
                attributes,
                namespace: _,
            } => {
                if !name.local_name.eq_ignore_ascii_case("Effect") {
                    return Err(VisualEffectLoaderError::Parse(
                        "Root element is not <Effect>",
                    ));
                }
                break attributes;
            }
            XmlEvent::EndElement { .. } => unreachable!(),
            _ => {}
        }
    };

    let mut effect_type = EffectType::Single;
    let mut duration = 0f32;
    let mut keyframes_per_second = 10i32;
    let mut version = 1i32;

    for attr in attributes {
        if attr.name.local_name.eq_ignore_ascii_case("type") {
            effect_type = attr
                .value
                .parse()
                .map_err(|_e| VisualEffectLoaderError::Parse("Invalid type="))?;
        } else if attr.name.local_name.eq_ignore_ascii_case("duration") {
            duration = attr
                .value
                .parse()
                .map_err(|_e| VisualEffectLoaderError::Parse("Invalid duration="))?;
        } else if attr
            .name
            .local_name
            .eq_ignore_ascii_case("keyFramesPerSecond")
        {
            keyframes_per_second = attr
                .value
                .parse()
                .map_err(|_e| VisualEffectLoaderError::Parse("Invalid keyFramesPerSecond="))?;
        } else if attr.name.local_name.eq_ignore_ascii_case("version") {
            version = attr
                .value
                .parse()
                .map_err(|_e| VisualEffectLoaderError::Parse("Invalid version="))?;
        }
    }

    let mut emitters = Vec::new();
    let mut lights = Vec::new();

    loop {
        match xml.next()? {
            XmlEvent::EndElement { .. } => break,
            XmlEvent::EndDocument => unreachable!(),
            XmlEvent::StartElement {
                name,
                attributes,
                namespace: _,
            } => {
                let emitter_name = attributes
                    .into_iter()
                    .find(|a| a.name.local_name.eq_ignore_ascii_case("name"))
                    .map(|a| a.value);
                if name.local_name.eq_ignore_ascii_case("ContinuousEmitter") {
                    emitters.push(parse_emitter_continuous(
                        xml,
                        version,
                        keyframes_per_second,
                    )?);
                } else if name.local_name.eq_ignore_ascii_case("PulseEmitter") {
                    emitters.push(parse_emitter_pulse(xml, version)?);
                } else if name.local_name.eq_ignore_ascii_case("Light") {
                    lights.push(parse_light(xml)?);
                } else {
                    xml.skip()?;
                }
            }
            _ => {}
        }
    }

    Ok(Effect {
        effect_type,
        duration,
        // keyframes_per_second,
        // version,
        emitters,
        lights,
    })
}

fn parse_emitter_continuous(
    xml: &mut EventReader<impl Read>,
    version: i32,
    keyframes_per_second: i32,
) -> Result<Emitter, VisualEffectLoaderError> {
    eprintln!("Note: Visual effect <ContinuousEmitter> elements are not yet supported");
    xml.skip()?;
    Ok(Emitter::Continuous)
}

fn parse_emitter_pulse(
    xml: &mut EventReader<impl Read>,
    version: i32,
) -> Result<Emitter, VisualEffectLoaderError> {
    eprintln!("Note: Visual effect <PulseEmitter> elements are not yet supported");
    xml.skip()?;
    Ok(Emitter::Pulse)
}

fn parse_light(xml: &mut EventReader<impl Read>) -> Result<Light, VisualEffectLoaderError> {
    eprintln!("Note: Visual effect <Light> elements are not yet supported");
    xml.skip()?;
    Ok(Light)
}

#[derive(Debug, Clone, Copy)]
pub enum EffectType {
    Single,
    Looping,
    Infinite,
}

impl FromStr for EffectType {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "Single" => Ok(Self::Single),
            "Looping" => Ok(Self::Looping),
            "Infinite" => Ok(Self::Infinite),
            _ => Err(()),
        }
    }
}
