use std::time::Duration;

use bevy::prelude::*;

use crate::spelling::{
    element::{Element, Magnitudes},
    spells,
};

pub fn plugin(app: &mut App) {
    app.add_observer(release_channel);
    app.add_systems(FixedUpdate, elapse_sprays);
    app.add_systems(
        PostUpdate,
        debug_draw_sprays.after(TransformSystems::Propagate),
    );
}

const ARC_ANGLE: f32 = std::f32::consts::TAU / 20.;

/// The height of the cylinder segment hitbox
const HEIGHT: f32 = 10.;

const CHANNEL_MIN: Duration = Duration::from_millis(500);
const CHANNEL_MAX: Duration = Duration::from_millis(4000);

const RANGE_BASE: f32 = 5.;
const RANGE_ELEM: f32 = 1.;

const EXTEND_TIME: f32 = 0.25;

pub fn spray_spell(caster: Entity, elements: Magnitudes) -> impl Bundle {
    (SpraySpell {
        lifetime: Timer::new(CHANNEL_MAX, TimerMode::Once),
        full_range: range(&elements),
    },)
}

#[derive(Component, Debug, Reflect)]
#[require(Transform)]
struct SpraySpell {
    pub lifetime: Timer,
    pub full_range: f32,
}

fn range(elements: &Magnitudes) -> f32 {
    let spray_magnitude = elements.get(Element::Fire) as u16
        + (elements.get(Element::Cold) as u16)
        + (elements.get(Element::Steam) as u16)
        + (elements.get(Element::Water) as u16)
        + (elements.get(Element::Poison) as u16);
    RANGE_BASE + RANGE_ELEM * (spray_magnitude as f32)
}

fn current_range(spell: &SpraySpell) -> f32 {
    let t = (spell.lifetime.elapsed_secs() / EXTEND_TIME).min(1.);
    t * spell.full_range
}

fn release_channel(event: On<spells::Release>, mut spells: Query<&mut SpraySpell>) {
    let Ok(mut spell) = spells.get_mut(event.spell) else {
        return;
    };
    spell.lifetime.set_duration(CHANNEL_MIN);
}

fn elapse_sprays(
    spells: Query<(Entity, &mut SpraySpell)>,
    time: Res<Time>,
    mut commands: Commands,
) {
    for (entity, mut spell) in spells {
        spell.lifetime.tick(time.delta());
        if spell.lifetime.just_finished() {
            commands.trigger(spells::Complete { spell: entity });
            // TODO: Let any VFX complete..?
            commands.entity(entity).try_despawn();
        }
    }
}

fn debug_draw_sprays(spells: Query<(&SpraySpell, &GlobalTransform)>, mut gizmos: Gizmos) {
    for (spell, trans) in spells {
        let current_range = current_range(spell);
        let gizmo_trans = trans.compute_transform()
            * Transform::from_rotation(Quat::from_rotation_y(0.25 * std::f32::consts::TAU));
        let prim = CircularSegment::new(current_range, ARC_ANGLE);
        gizmo_circular_segment(
            &mut gizmos,
            prim,
            (gizmo_trans).to_isometry(),
            bevy::color::palettes::basic::WHITE,
        );
        gizmos.line(
            trans.transform_point(Vec3::ZERO),
            trans.transform_point(Vec3::NEG_Z * current_range),
            bevy::color::palettes::basic::WHITE,
        );
    }
}

fn gizmo_circular_segment(
    gizmos: &mut Gizmos,
    prim: CircularSegment,
    isometry: Isometry3d,
    color: impl Into<Color>,
) {
    let color = color.into();
    let [left, right] = prim.arc.endpoints();
    let left = vec3(left.y, 0., left.x);
    let right = vec3(right.y, 0., right.x);
    gizmos.line(
        isometry.transform_point(Vec3A::ZERO).to_vec3(),
        { isometry.transform_point(left.to_vec3a()).to_vec3() },
        color,
    );
    gizmos.line(
        isometry.transform_point(Vec3A::ZERO).to_vec3(),
        isometry.transform_point(right.to_vec3a()).to_vec3(),
        color,
    );
    gizmos.arc_3d(
        prim.angle(),
        prim.radius(),
        isometry * Isometry3d::from_rotation(Quat::from_rotation_y(-prim.half_angle())),
        color,
    );
}

/*
#[derive(Clone, Copy)]
struct CylinderSegment {
    pub half_height: f32,
    pub half_angle: f32,
    pub radius: f32,
    cylinder_inside_arc: avian3d::parry::shape::Cylinder,
    cylinder_outside_arc: avian3d::parry::shape::Cylinder,
}

impl avian3d::parry::shape::Shape for CylinderSegment {
    fn compute_local_aabb(&self) -> avian3d::parry::bounding_volume::Aabb {
        todo!()
    }

    fn compute_local_bounding_sphere(&self) -> avian3d::parry::bounding_volume::BoundingSphere {
        todo!()
    }

    fn clone_dyn(&self) -> Box<dyn avian3d::parry::shape::Shape> {
        Box::new(*self)
    }

    fn scale_dyn(
        &self,
        scale: &avian3d::parry::math::Vector<f32>,
        num_subdivisions: u32,
    ) -> Option<Box<dyn avian3d::parry::shape::Shape>> {
        todo!()
    }

    fn mass_properties(&self, density: f32) -> avian3d::parry::mass_properties::MassProperties {
        todo!()
    }

    fn shape_type(&self) -> avian3d::parry::shape::ShapeType {
        avian3d::parry::shape::ShapeType::Custom
    }

    fn as_typed_shape(&self) -> avian3d::parry::shape::TypedShape<'_> {
        todo!()
    }

    fn ccd_thickness(&self) -> f32 {
        todo!()
    }

    fn ccd_angular_thickness(&self) -> f32 {
        todo!()
    }

    fn is_convex(&self) -> bool {
        true
    }

    fn as_support_map(&self) -> Option<&dyn avian3d::parry::shape::SupportMap> {
        Some(self)
    }
}

impl avian3d::parry::shape::SupportMap for CylinderSegment {
    fn local_support_point(
        &self,
        dir: &avian3d::parry::math::Vector<f32>,
    ) -> avian3d::parry::math::Point<f32> {
        let eps = 0.000001;
        if dir.x.abs() >= eps || dir.z.abs() >= eps {
            if let Some(norm_dir) = dir.try_normalize(eps) {
                let dir_angle = norm_dir.x.atan2(norm_dir.z);
                if dir_angle.abs() > self.half_angle {
                    return self.cylinder_outside_arc.local_support_point(dir);
                }
            }
        }
        return self.cylinder_inside_arc.local_support_point(dir);
    }
}
*/
