use avian3d::prelude::{Collider, PhysicsLayer};
use remagic::xnb_readers::magicka_content::GenericTriangleMesh;

use crate::magicka_level_model::map_vec3;

#[derive(PhysicsLayer, Clone, Copy, Debug, Default)]
pub enum Layers {
    #[default]
    Default, // Layer 0 - the default layer that objects are assigned to
    Level,   // Layer 1 - static level collision
    Trigger, // Layer 2 - area triggers
}

pub fn to_collider(mesh: &GenericTriangleMesh) -> Option<Collider> {
    if mesh.indices.is_empty() {
        return None;
    }
    let vertices = mesh.vertices.as_slice();
    let indices = mesh.indices.as_slice();
    let collider = to_collider_raw(vertices, indices);
    Some(collider)
}

pub fn to_collider_raw(
    vertices: &[remagic::xnb::types::Vector3],
    indices: &[(i32, i32, i32)],
) -> Collider {
    let vertices = vertices.iter().copied().map(map_vec3).collect();
    let indices = indices
        .iter()
        .copied()
        .map(|(a, b, c)| {
            [
                u32::try_from(a).unwrap(),
                u32::try_from(b).unwrap(),
                u32::try_from(c).unwrap(),
            ]
        })
        .collect();
    let collider = Collider::trimesh(vertices, indices);
    collider
}
