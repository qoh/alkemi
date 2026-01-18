use super::xna_geom;
use bevy::{
    asset::RenderAssetUsages, ecs::relationship::RelatedSpawnerCommands, light::NotShadowCaster,
    mesh::PrimitiveTopology, prelude::*,
};
use remagic::xnb_readers::magicka_mesh::{BiTree, BiTreeModel, BiTreeNode};
use typed_path::PlatformPath;

pub(crate) fn spawn_bitree_model(
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    assets: &AssetServer,
    parent_commands: &mut RelatedSpawnerCommands<'_, ChildOf>,
    model: &BiTreeModel,
    content_path: &PlatformPath,
) {
    let mut i = 0;
    for bitree in &model.bitrees {
        let base_mesh = bitree_base_mesh(bitree);

        let mut root_ent_commands = parent_commands.spawn((
            Name::new("BiTreeRootNode"),
            Transform::default(),
            if bitree.visible {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            },
        ));
        root_ent_commands.with_children(|parent| {
            i = spawn_bitree_node(
                parent,
                content_path,
                &bitree.node,
                bitree,
                &base_mesh,
                meshes,
                materials,
                assets,
                i,
            );
        });
    }
}

fn spawn_bitree_node(
    parent_commands: &mut RelatedSpawnerCommands<'_, ChildOf>,
    content_path: &PlatformPath,
    tree_node: &BiTreeNode,
    tree_root: &BiTree,
    tree_root_base_mesh: &Mesh,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    assets: &AssetServer,
    mut i: usize,
) -> usize {
    let mut mesh = tree_root_base_mesh.clone();

    let start_index: usize = tree_node.start_index.try_into().unwrap();
    let primitive_count: usize = tree_node.primitive_count.try_into().unwrap();
    let index_buffer = tree_root.index_buffer.as_ref().unwrap();
    let invert_winding =
        xna_geom::assign_mesh_indices(&mut mesh, 0, start_index, primitive_count, index_buffer);

    let (maybe_material, vertex_color_state) = super::effect::translate_effect(
        tree_root.effect.as_ref(),
        content_path,
        invert_winding,
        assets,
    );

    if matches!(
        vertex_color_state,
        super::effect::VertexColorState::Disabled
    ) {
        mesh.remove_attribute(Mesh::ATTRIBUTE_COLOR);
    }

    let mesh_handle = meshes.add(mesh);

    let mut node_commands =
        parent_commands.spawn((Name::new("BiTreeNode"), Mesh3d::from(mesh_handle)));
    if let Some(material) = maybe_material {
        node_commands.insert(MeshMaterial3d(materials.add(material)));
    }
    if !tree_root.cast_shadows {
        node_commands.insert(NotShadowCaster);
    }
    i += 1;
    node_commands.with_children(|parent_commands| {
        if let Some(child) = &tree_node.child_a {
            i = spawn_bitree_node(
                parent_commands,
                content_path,
                child,
                tree_root,
                tree_root_base_mesh,
                meshes,
                materials,
                assets,
                i,
            );
        }
        if let Some(child) = &tree_node.child_b {
            i = spawn_bitree_node(
                parent_commands,
                content_path,
                child,
                tree_root,
                tree_root_base_mesh,
                meshes,
                materials,
                assets,
                i,
            );
        }
    });
    i
}

fn bitree_base_mesh(bitree: &BiTree) -> Mesh {
    let Some(declaration) = &bitree.vertex_declaration else {
        warn!("bitree has no vertex declaration");
        return Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
    };
    let Some(buffer) = &bitree.vertex_buffer else {
        warn!("bitree has no vertex buffer");
        return Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
    };
    xna_geom::init_mesh_from_xna_vertices(
        declaration,
        buffer,
        bitree.vertex_stride,
        bitree.vertex_count,
        0,
    )
}
