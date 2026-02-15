// Notes:
//
// Opposite beams that intersect will explode after 0.25 seconds

use std::time::Duration;

use avian3d::prelude::{LayerMask, PhysicsLayer as _, SpatialQuery, SpatialQueryFilter};
use bevy::prelude::*;

use crate::{
    magicka_level_model::Layers,
    spelling::{
        element::{Element, Magnitudes},
        spells,
    },
};

pub fn plugin(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        (
            timeout_beams,
            extend_beams,
            collide_beams,
            shrink_colliding_beams,
            shorten_despawn_stopped_beams,
        )
            .chain(),
    );
    app.add_systems(
        FixedUpdate,
        (
            stop_reflected_beams,
            reflect_colliding_beams,
            position_reflected_beams,
        )
            .chain()
            .after(collide_beams),
    );
    app.add_observer(release_channeling_beam);
    app.add_observer(stop_beam);
    app.add_plugins(vfx::plugin);
}

pub fn beam_spell(caster: Entity, elements: Magnitudes) -> impl Bundle {
    const LIFETIME_BASE: f32 = 1.;
    const LIFETIME_ELEM: f32 = 2.;

    let num_beam_elems =
        elements.get(Element::Life) as u16 + (elements.get(Element::Arcane) as u16);
    let lifetime = LIFETIME_BASE + LIFETIME_ELEM * (num_beam_elems as f32);

    (
        BeamSpell {
            lifetime: Timer::from_seconds(lifetime, TimerMode::Once),
        },
        Beam {
            elements,
            ignore_entity: Some(caster),
            ..default()
        },
    )
}

#[derive(Component, Debug, Reflect)]
#[require(Beam)]
struct BeamSpell {
    pub lifetime: Timer,
}

fn release_channeling_beam(event: On<spells::Release>, mut beam_spells: Query<&mut BeamSpell>) {
    let Ok(mut beam_spell) = beam_spells.get_mut(event.spell) else {
        return;
    };
    // Require channeling for a minimum amount of time in case input stops very early
    let minimum_channel_time = Duration::from_millis(500);
    // If elapsed time is already past the minimum, the next tick will finish it immediately
    beam_spell.lifetime.set_duration(minimum_channel_time);
}

fn timeout_beams(
    beam_spells: Query<(Entity, &mut BeamSpell)>,
    time: Res<Time>,
    mut commands: Commands,
) {
    for (ent, mut spell) in beam_spells {
        spell.lifetime.tick(time.delta());
        if spell.lifetime.just_finished() {
            commands.trigger(spells::Complete { spell: ent });
            commands.trigger(Stop(ent));
        }
    }
}

#[derive(Component, Debug, Clone, Reflect)]
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
    pub ray: Ray3d,
    pub distance: f32,
    pub normal: Vec3,
    pub entity: Entity,
    pub reflected_beam: Option<Entity>,
}

impl BeamCollision {
    pub fn point(&self) -> Vec3 {
        self.ray.get_point(self.distance)
    }
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

/// Reflects beams that hit it.
#[derive(Component, Default, Clone, Copy, Debug)]
pub struct ReflectBeams;

/// A beam that has been reflected.
#[derive(Component, Clone, Copy, Debug)]
struct ReflectedBeam {
    pub reflections: usize,
    pub source_beam: Entity,
    //pub reflector: Entity,
}

fn extend_beams(beams: Query<(&mut Transform, &Beam, &BeamState)>, time: Res<Time>) {
    for (trans, beam, state) in beams {
        if state.stopping() {
            continue;
        }
        let mut length =
            (trans.scale.z + time.delta_secs() * beam.extend_speed).max(beam.min_length);
        if let Some(max) = beam.max_length {
            length = length.min(max);
        }
        trans.map_unchanged(|t| &mut t.scale.z).set_if_neq(length);
    }
}

// XXX: Should this be integrated with FixedUpdate and use ShapeCaster instead?
fn collide_beams(
    beams: Query<(&mut BeamState, Entity, &Beam)>,
    transform_helper: TransformHelper,
    spatial_query: SpatialQuery,
) {
    // This is mainly to prevent reflected beams from hitting their original reflector
    // For players, prefer BeamState::ignore_entity
    let min_distance = 0.1;

    let repeat_collision_margin = 0.1;

    for (mut state, beam_ent, beam) in beams {
        let beam_trans = transform_helper.compute_global_transform(beam_ent).unwrap();
        let length = beam_trans.scale().z;
        // If we collided last update and haven't shrunk further, cast a bit further to ensure a chance to re-hit, just in case
        let modified_length = if let Some(last) = &state.colliding
            && length - last.distance >= -0.0001
        {
            length.max(last.distance + repeat_collision_margin)
        } else {
            length
        };

        let direction = beam_trans.rotation() * Dir3::NEG_Z;
        let origin = beam_trans.translation() + min_distance * direction;
        let max_distance = modified_length - min_distance;
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
            ray: Ray3d::new(origin, direction),
            distance: hit.distance,
            normal: hit.normal,
            entity: hit.entity,
            reflected_beam: state.colliding.as_ref().and_then(|prev| {
                if prev.entity == hit.entity {
                    prev.reflected_beam
                } else {
                    None
                }
            }),
        });
    }
}

fn shrink_colliding_beams(beams: Query<(&mut Transform, &BeamState), Changed<BeamState>>) {
    for (mut trans, beam) in beams {
        if let Some(collision) = &beam.colliding {
            trans.scale.z = collision.distance;
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
            if *removed >= trans.scale.z {
                commands.entity(entity).try_despawn();
            }
        }
    }
}

fn stop_reflected_beams(
    reflected_beams: Query<(Entity, &ReflectedBeam)>,
    beams: Query<&BeamState>,
    mut commands: Commands,
) {
    for (reflected_beam, reflection) in reflected_beams {
        let still_reflected = 'check: {
            let Ok(BeamState {
                colliding:
                    Some(BeamCollision {
                        reflected_beam: Some(source_reflected_beam),
                        ..
                    }),
                ..
            }) = beams.get(reflection.source_beam)
            else {
                break 'check false;
            };
            reflected_beam == *source_reflected_beam
        };

        if !still_reflected {
            commands.entity(reflected_beam).remove::<ReflectedBeam>();
            commands.trigger(Stop(reflected_beam));
        }
    }
}

fn reflect_colliding_beams(
    beams: Query<(Entity, &mut BeamState, &Beam, Option<&ReflectedBeam>), Changed<BeamState>>,
    reflectors: Query<(), With<ReflectBeams>>,
    mut commands: Commands,
) {
    let only_explicit_reflectors = true;

    for (source_beam, state, beam, already_reflected) in beams {
        if let Some(mut collision) = state.filter_map_unchanged(|s| s.colliding.as_mut())
            && collision.reflected_beam.is_none()
        {
            if only_explicit_reflectors && !reflectors.contains(collision.entity) {
                continue;
            }

            let reflected_beam = commands
                .spawn((
                    Beam {
                        //ignore_entity: Some(collision.entity),
                        ignore_entity: None,
                        min_length: (beam.min_length - collision.distance).max(0.),
                        max_length: beam.max_length.map(|l| (l - collision.distance).max(0.)),
                        ..beam.clone()
                    },
                    ReflectedBeam {
                        reflections: already_reflected.map(|r| r.reflections).unwrap_or(0) + 1,
                        source_beam,
                        //reflector: collision.entity,
                    },
                ))
                .id();
            collision.reflected_beam = Some(reflected_beam);
        }
    }
}

fn position_reflected_beams(
    reflected_beams: Query<(Entity, &ReflectedBeam)>,
    mut beams: Query<(&mut Transform, &BeamState)>,
) {
    let stay_on_plane = true;

    for (reflected_beam, reflection) in reflected_beams {
        let Ok((
            source_trans,
            BeamState {
                colliding: Some(collision),
                ..
            },
        )) = beams.get(reflection.source_beam)
        else {
            continue;
        };

        let source_trans = *source_trans;
        let /*mut*/ reflect_at = collision.point();
        let mut reflect_dir = collision.ray.direction.reflect(collision.normal);

        if stay_on_plane {
            let plane = Vec3::Y; // TODO: Rotate by beam
            reflect_dir = match reflect_dir.reject_from(plane).try_normalize() {
                Some(d) => d,
                // TODO: Delete the beam without recreating it every frame? This will leave the beam untransformed.
                None => continue,
            };
            // Ensure the position is in-plane. Theoretically unnecessary, just making sure.
            // Requires global transform or parenting.
            //reflect_at =
            //    reflect_at.reject_from(plane) + source_trans.translation.project_onto(plane);
        }

        let Ok((mut reflected_trans, _)) = beams.get_mut(reflected_beam) else {
            continue;
        };
        reflected_trans.translation = reflect_at;
        reflected_trans.rotation = Quat::look_to_rh(reflect_dir, Vec3::Y).inverse();
        reflected_trans.scale.x = source_trans.scale.x;
        reflected_trans.scale.y = source_trans.scale.y;
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

    use crate::spelling::{color::spell_color, element::Element, spells::beam::BeamState};

    use super::{Beam, extend_beams, shorten_despawn_stopped_beams, shrink_colliding_beams};
    use bevy::{light::NotShadowCaster, prelude::*, time::Stopwatch};

    pub fn plugin(app: &mut App) {
        let _ = app.try_register_required_components::<Beam, Visibility>();
        app.add_observer(add_beam_mesh);
        app.add_systems(
            Update,
            animate_beam_scale_offset
                .after(shrink_colliding_beams)
                .after(shorten_despawn_stopped_beams),
        );
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

    #[derive(Component, Debug, Default)]
    struct BeamMesh {
        elapsed: Stopwatch,
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
        let color = spell_color(&beam.elements).unwrap_or(LinearRgba::rgb(0.03, 0.03, 0.03));
        let material = StandardMaterial {
            base_color: Color::BLACK,
            emissive: color,
            emissive_exposure_weight: -30., // -5.,
            ..default()
        };
        let beam_mesh = meshes.add(beam_mesh());
        let display_child = commands
            .spawn((
                ChildOf(event.entity),
                BeamMesh::default(),
                Mesh3d(beam_mesh.clone()),
                MeshMaterial3d(materials.add(material)),
                NotShadowCaster,
                Children::spawn(light_line(PointLight {
                    color: color.into(),
                    // Perf: Would like to cast shadows, but with the repeated light line hack, it's too laggy
                    //shadows_enabled: true,
                    radius: 0.25,
                    ..default()
                })),
            ))
            .id();

        if beam.elements.contains(Element::Arcane) {
            let material_core = StandardMaterial {
                base_color: Color::BLACK,
                unlit: true,
                // depth_bias: 4_550.,
                // depth_bias: 6_000.,
                depth_bias: 2_250.,
                ..default()
            };
            commands.entity(display_child).with_child((
                Transform::from_scale(vec3(0.9, 0.9, 0.98)).with_translation(Vec3::NEG_Z * 0.01),
                Mesh3d(beam_mesh),
                MeshMaterial3d(materials.add(material_core)),
                NotShadowCaster,
            ));
        }
    }

    fn beam_mesh() -> Mesh {
        // Cylinder creates meshes that point +Y, but beams are -Z
        let y_to_neg_z = Quat::from_mat3(&Mat3::from_cols(-Vec3::Z, Vec3::X, Vec3::Y));
        Cylinder::new(BEAM_RADIUS, 1.)
            .mesh()
            .anchor(bevy::mesh::CylinderAnchor::Bottom)
            // TODO: Round caps that don't stretch with length - likely want .without_caps()
            .build()
            .rotated_by(y_to_neg_z)
    }

    /// As a workaround for the absence of capsule lights, spawn a line of point lights
    fn light_line(defaults: PointLight) -> SpawnIter<impl Iterator<Item = impl Bundle>> {
        let lights = (0..LIGHTS).map(move |i| {
            let t = (i as f32 + 0.5) / (LIGHTS as f32);
            (
                Transform::from_translation(t * Vec3::NEG_Z),
                defaults,
                // PhasingLight { phase_offset: t },
            )
        });
        SpawnIter(lights)
    }

    fn animate_beam_scale_offset(
        beams: Query<(&BeamState, &Transform), Without<BeamMesh>>,
        meshes: Query<(&mut Transform, &mut BeamMesh, &ChildOf)>,
        time: Res<Time>,
    ) {
        for (mut trans, mut mesh_state, parent_ref) in meshes {
            mesh_state.elapsed.tick(time.delta());
            let Ok((beam, beam_trans)) = beams.get(parent_ref.parent()) else {
                continue;
            };
            let open_t = (mesh_state.elapsed.elapsed_secs() / 0.175).min(1.);
            let mut width = CircularOutCurve.sample_clamped(open_t)
                * QuinticInCurve.sample_clamped(open_t)
                * BackOutCurve
                    .sample_clamped(1. - (1. - open_t).powi(2))
                    .powi(2);
            if let Some(length_removed) = beam.stopping_length_removed {
                let length = beam_trans.scale.z;
                let t = (length_removed / length).min(1.);
                trans.translation.z = -t;
                trans.scale.z = 1. - t;
                width = (1. - t).powi(3).min(width);
            }
            let pulse_variance = width * 0.025;
            let pulse_phase =
                (mesh_state.elapsed.elapsed_secs().fract() * std::f32::consts::TAU).cos();
            width *= 1. + (pulse_phase * pulse_variance);
            // eprintln!("beam {} gets width {width} t {open_t}", parent_ref.parent());
            trans.scale.x = width;
            trans.scale.y = width;
        }
    }

    // TODO: This should take into account global scale
    fn adjust_light_proportion(
        beams: Query<(&Transform, &Children), With<BeamMesh>>,
        mut lights: Query<&mut PointLight, With<ChildOf>>,
    ) {
        let lumens_per_length = 10_000_000.0 / 90.;
        for (beam_trans, children) in beams {
            for child in children {
                let Ok(mut light) = lights.get_mut(*child) else {
                    continue;
                };
                let beam_length = beam_trans.scale.z;
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
            trans.translation.z = -phase;
        }
    }
}
