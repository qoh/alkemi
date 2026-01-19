// TODO: Move this out of magicka_level_model, since it isn't exclusive to levels

use crate::magicka_level_model::{Spawner, map_vec3};
use bevy::{asset::AsAssetId, prelude::*};

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
    assets: &AssetServer,
) {
    let position = map_vec3(visual_effect.position);
    let direction = map_vec3(visual_effect.direction);
    parent.spawn((
        Name::new(visual_effect.id.clone()),
        Transform::from_translation(position).looking_to(direction, Vec3::Y),
        Visibility::default(),
        VisualEffect {
            // TODO: Find the visual effect by name in the Effects/ folders
            asset: Handle::Uuid(AssetId::<Scene>::INVALID_UUID, std::marker::PhantomData),
            effect: visual_effect.effect.clone(),
            range: visual_effect.range,
        },
    ));
}

#[derive(Component, Debug, Reflect)]
#[require(SceneRoot(Handle::Uuid(AssetId::<Scene>::INVALID_UUID, std::marker::PhantomData)))]
pub struct VisualEffect {
    pub asset: Handle<crate::magicka_assets::visual_effect::VisualEffect>,
    pub effect: String,
    pub range: f32,
}

impl AsAssetId for VisualEffect {
    type Asset = crate::magicka_assets::visual_effect::VisualEffect;

    fn as_asset_id(&self) -> AssetId<Self::Asset> {
        self.asset.id()
    }
}

type ChangedOrAsset<C> = Or<(Changed<C>, AssetChanged<C>)>;
pub fn assign_visual_effect_scene(
    effects: Query<(&VisualEffect, &mut SceneRoot), ChangedOrAsset<VisualEffect>>,
    visual_effects: Res<Assets<crate::magicka_assets::visual_effect::VisualEffect>>,
) {
    for (effect, mut effect_scene_root) in effects {
        let scene = visual_effects
            .get(&effect.asset)
            .map(|e| e.scene.clone())
            .unwrap_or_else(|| {
                Handle::Uuid(AssetId::<Scene>::INVALID_UUID, std::marker::PhantomData)
            });
        if effect_scene_root.0 != scene {
            effect_scene_root.0 = scene;
        }
    }
}
