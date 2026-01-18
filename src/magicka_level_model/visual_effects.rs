// The data for the effects seems to be in .xml files under Content
//
// For example, WizardCastle/wc_s2.xnb has an {effect: "special_propp_spell"}
// There exists Content/Effects/Special/Special_Propp_Spell.xml
// 2 │ <Effect type="Single" duration="3" keyFramesPerSecond="10">
// 3 │   <ContinuousEmitter name="1. Continuous emitter">
// 4 │     <BlendMode Value="Additive" />
// ...

use crate::magicka_level_model::{Spawner, map_vec3};
use bevy::prelude::*;

pub fn debug_visual_effects(
    effects: Query<&GlobalTransform, With<VisualEffect>>,
    mut gizmos: Gizmos,
) {
    for transform in effects {
        gizmos.axes(*transform, 1.);
        gizmos.sphere(transform.to_isometry(), 1., Color::srgb(1., 0., 0.));
    }
}

pub fn spawn_visual_effect(
    Spawner::Parent(parent): Spawner,
    visual_effect: &remagic::xnb_readers::magicka_content::VisualEffect,
) {
    let position = map_vec3(visual_effect.position);
    let direction = map_vec3(visual_effect.direction);
    parent.spawn((
        Name::new(visual_effect.id.clone()),
        Transform::from_translation(position).looking_to(direction, Vec3::Y),
        Visibility::default(),
        VisualEffect {
            effect: visual_effect.effect.clone(),
            range: visual_effect.range,
        },
    ));
}

#[derive(Component, Debug, Reflect)]
pub struct VisualEffect {
    pub effect: String,
    pub range: f32,
}
