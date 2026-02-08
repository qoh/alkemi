// Notes:
//
// Opposite beams that intersect will explode after 0.25 seconds

use avian3d::prelude::{LayerMask, PhysicsLayer as _, SpatialQuery, SpatialQueryFilter};
use bevy::prelude::*;

use crate::{magicka_level_model::Layers, spelling::element::Magnitudes};

pub fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (
            extend_beams,
            collide_beams,
            shrink_colliding_beams,
            shorten_despawn_stopped_beams,
        )
            .chain(),
    );
    app.add_observer(stop_beam);
    app.add_plugins(vfx::plugin);
}

#[derive(Component, Debug, Reflect)]
#[require(BeamState, Transform)]
pub struct Beam {
    pub elements: Magnitudes,
    pub ignore_entity: Option<Entity>,
    pub min_length: f32,
    pub max_length: Option<f32>,
    pub extend_speed: f32,
}

impl Default for Beam {
    fn default() -> Self {
        Self {
            elements: Default::default(),
            ignore_entity: None,
            min_length: 0.,
            max_length: Some(75.),
            extend_speed: 30. * 3.,
        }
    }
}

// TODO: Merge beams that intersect
// TODO: Unmerge beams that no longer intersect
// TODO: Stop merged beams when they have no sources?
#[derive(Component, Debug, Reflect)]
#[relationship(relationship_target = BeamMergeFrom)]
pub struct BeamMergeInto(pub Entity);
#[derive(Component, Debug, Reflect)]
#[relationship_target(relationship = BeamMergeInto)]
pub struct BeamMergeFrom(Vec<Entity>);

#[derive(Component, Debug, Reflect)]
struct BeamState {
    time: f32,
    colliding: Option<BeamCollision>,
    stopping_length_removed: Option<f32>,
}

#[derive(Debug, Reflect)]
struct BeamCollision {
    pub distance: f32,
    pub normal: Vec3,
    pub entity: Entity,
}

impl Default for BeamState {
    fn default() -> Self {
        Self {
            time: 0.,
            colliding: None,
            stopping_length_removed: None,
        }
    }
}

impl BeamState {
    fn stopping(&self) -> bool {
        self.stopping_length_removed.is_some()
    }
}

fn extend_beams(beams: Query<(&mut Transform, &Beam, &BeamState)>, time: Res<Time>) {
    for (trans, beam, state) in beams {
        if state.stopping() {
            continue;
        }
        let mut length =
            (trans.scale.x + time.delta_secs() * beam.extend_speed).max(beam.min_length);
        if let Some(max) = beam.max_length {
            length = length.min(max);
        }
        trans.map_unchanged(|t| &mut t.scale.x).set_if_neq(length);
    }
}

// XXX: Should this be integrated with FixedUpdate and use ShapeCaster instead?
fn collide_beams(
    beams: Query<(&mut BeamState, Entity, &Beam)>,
    transform_helper: TransformHelper,
    spatial_query: SpatialQuery,
) {
    for (mut state, beam_ent, beam) in beams {
        let beam_trans = transform_helper.compute_global_transform(beam_ent).unwrap();
        let origin = beam_trans.translation();
        let direction = beam_trans.rotation() * Dir3::X;
        let max_distance = beam_trans.scale().x;
        let filter = SpatialQueryFilter {
            mask: LayerMask::ALL & !(Layers::Trigger.to_bits()),
            excluded_entities: beam.ignore_entity.iter().copied().collect(),
        };
        let Some(hit) = spatial_query.cast_ray(origin, direction, max_distance, false, &filter)
        else {
            if state.colliding.is_some() {
                state.colliding = None;
            }
            continue;
        };
        state.colliding = Some(BeamCollision {
            distance: hit.distance,
            normal: hit.normal,
            entity: hit.entity,
        });
    }
}

fn shrink_colliding_beams(beams: Query<(&mut Transform, &BeamState), Changed<BeamState>>) {
    for (mut trans, beam) in beams {
        if let Some(collision) = &beam.colliding {
            trans.scale.x = collision.distance;
        }
    }
}

/// Stop emitting a beam. Detaches it from any parent & later despawns it.
#[derive(EntityEvent)]
pub struct Stop(pub Entity);

fn stop_beam(event: On<Stop>, mut beams: Query<&mut BeamState>, mut commands: Commands) {
    let Ok(mut beam) = beams.get_mut(event.0) else {
        return;
    };

    beam.stopping_length_removed.get_or_insert(0.);
    commands.entity(event.0).remove_parent_in_place();
}

fn shorten_despawn_stopped_beams(
    beams: Query<(&mut BeamState, Entity, &Beam, &Transform)>,
    time: Res<Time>,
    mut commands: Commands,
) {
    for (state, entity, beam, trans) in beams {
        if let Some(mut removed) =
            state.filter_map_unchanged(|b| b.stopping_length_removed.as_mut())
        {
            *removed += time.delta_secs() * beam.extend_speed;
            if *removed >= trans.scale.x {
                commands.entity(entity).try_despawn();
            }
        }
    }
}

// TODO: We need to do this in tree order to propagate properly
fn move_merged_beams(
    // merged_beams: Query<(&mut Transform, &BeamMergeFrom)>,
    // source_beams: Query<&Transform, With<BeamMergeInto>>,
    transform_helper: TransformHelper,
) {
    // Set to average position/direction
}

mod vfx {
    // TODO: Shrink the front of the beam based on BeamState.stopped_length_removed

    // TODO: Particles

    use crate::spelling::color::spell_color;

    use super::{Beam, extend_beams, shrink_colliding_beams};
    use bevy::{light::NotShadowCaster, prelude::*};

    pub fn plugin(app: &mut App) {
        app.add_observer(add_beam_mesh);
        app.add_systems(
            Update,
            adjust_light_proportion
                .after(extend_beams)
                .after(shrink_colliding_beams),
        );
        app.add_systems(Update, move_phasing_lights);
    }

    const LIGHT_DIVISIONS: usize = 4;

    const LIGHTS: usize = 2 * LIGHT_DIVISIONS + 1;

    const BEAM_RADIUS: f32 = 0.25;

    #[derive(Component, Debug)]
    pub struct PhasingLight {
        pub phase_offset: f32,
    }

    fn add_beam_mesh(
        event: On<Add, Beam>,
        beams: Query<&Beam>,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
        mut commands: Commands,
    ) {
        let Ok(beam) = beams.get(event.entity) else {
            return;
        };
        // let color = LinearRgba::rgb(10., 0.2, 0.2);
        // TODO: Color based on elements
        let color = spell_color(&beam.elements).unwrap_or(LinearRgba::rgb(0.03, 0.03, 0.03));
        let material = StandardMaterial {
            base_color: Color::BLACK,
            emissive: color,
            emissive_exposure_weight: -30., // -5.,
            ..default()
        };
        commands.entity(event.entity).insert((
            Mesh3d(meshes.add(beam_mesh())),
            MeshMaterial3d(materials.add(material)),
            NotShadowCaster,
            Children::spawn(light_line(PointLight {
                color: color.into(),
                // Perf: Would like to cast shadows, but with the repeated light line hack, it's too laggy
                //shadows_enabled: true,
                radius: 0.25,
                ..default()
            })),
        ));
    }

    fn beam_mesh() -> Mesh {
        // Cylinder creates meshes that point +Y, but beams are +X
        let y_to_x = Quat::from_mat3(&Mat3::from_cols(-Vec3::Y, Vec3::X, Vec3::Z));
        Cylinder::new(BEAM_RADIUS, 1.)
            .mesh()
            .anchor(bevy::mesh::CylinderAnchor::Bottom)
            // TODO: Round caps that don't stretch with length - likely want .without_caps()
            .build()
            .rotated_by(y_to_x)
    }

    /// As a workaround for the absence of capsule lights, spawn a line of point lights
    fn light_line(defaults: PointLight) -> SpawnIter<impl Iterator<Item = impl Bundle>> {
        let lights = (0..LIGHTS).map(move |i| {
            let t = (i as f32 + 0.5) / (LIGHTS as f32);
            (
                Transform::from_translation(t * Vec3::X),
                defaults,
                // PhasingLight { phase_offset: t },
            )
        });
        SpawnIter(lights)
    }

    fn adjust_light_proportion(
        beams: Query<(&Transform, &Children)>,
        mut lights: Query<&mut PointLight, With<ChildOf>>,
    ) {
        let lumens_per_length = 2_000_000.0 / 90.;
        for (beam_trans, children) in beams {
            for child in children {
                let Ok(mut light) = lights.get_mut(*child) else {
                    continue;
                };
                let beam_length = beam_trans.scale.x;
                let light_length = beam_length / (LIGHTS as f32);
                light.intensity = lumens_per_length * light_length;
                // Perf: For short beams, could aggressively fade out odd index lights and hide them
            }
        }
    }

    // XXX: This causes lights to warp in at start/out at end. Needs to fade in/out intensity before it's usable.
    fn move_phasing_lights(lights: Query<(&mut Transform, &PhasingLight)>, time: Res<Time>) {
        let period = 2.;
        if time.wrap_period().as_secs_f32() % period > 0.00001 {
            warn_once!("time wrapping period does not evenly divide beam light animation period");
        }
        for (mut trans, phasing) in lights {
            let phase = (phasing.phase_offset + (time.elapsed_secs_wrapped() / period)).fract();
            trans.translation.x = phase;
        }
    }
}
