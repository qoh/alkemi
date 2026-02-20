use std::{
    cmp::{Ordering, Reverse},
    collections::BinaryHeap,
    time::Duration,
};

use avian3d::prelude::{Collider, PhysicsLayer, SpatialQuery, SpatialQueryFilter};
use bevy::prelude::*;
use rand::Rng;

use crate::{
    components_basic::{Elapsed, Lifetime},
    magicka_level_model::Layers,
    spelling::{
        element::{Element, Magnitudes},
        spells::{self, Spell},
    },
};

const INTERVAL: f32 = 0.25;

const DURATION_FORWARD_BASE: f32 = 5.;
const DURATION_FORWARD_EXTRA_PER: f32 = 0.5;

const RANGE_INITIAL: f32 = 9.5;

const RANGE_ARC_BASE: f32 = 5.;
const RANGE_ARC_EXTRA_PER: f32 = 0.5;

const JUMPS_BASE: usize = 1;
const JUMPS_EXTRA_PER: usize = 3;

const CONE_ANGLE_FORWARD: f32 = 25.0f32.to_radians();
const CONE_ANGLE_AREA: f32 = 45.0f32.to_radians();

/*
local ARC_RANGE = SpellSettings.lightning_arc_range
local ARC_RANGE_EA = SpellSettings.lightning_arc_range_per_element
local NUM_JUMPS = SpellSettings.lightning_num_jumps
local NUM_JUMPS_EA = SpellSettings.lightning_num_jumps_per_element
local ARC_DURATION = SpellSettings.lightning_arc_duration_base
local ARC_DURATION_EA = SpellSettings.lightning_arc_duration_per_element
*/

pub fn plugin(app: &mut App) {
    app.add_observer(stop_channeling);
    app.add_systems(FixedUpdate, apply_lightning);
    app.add_plugins(vfx::plugin);
}

pub fn cast_forward(elements: Magnitudes) -> impl Bundle {
    let lightning = elements.get(Element::Lightning);
    let duration =
        DURATION_FORWARD_BASE + DURATION_FORWARD_EXTRA_PER * (lightning.saturating_sub(1) as f32);
    let range_arc = RANGE_ARC_BASE + RANGE_ARC_EXTRA_PER * (lightning.saturating_sub(1) as f32);
    let number_of_jumps = JUMPS_BASE + JUMPS_EXTRA_PER * (lightning.saturating_sub(1) as usize);

    (LightningSpell {
        lifetime: Timer::from_seconds(duration, TimerMode::Once),
        discharge_timer: Timer::new(Duration::from_secs_f32(INTERVAL), TimerMode::Repeating),
        discharge_is_first: true,
        range_initial: RANGE_INITIAL,
        range_arc,
    },)
}

#[derive(Component, Debug, Reflect)]
#[require(Transform)]
struct LightningSpell {
    lifetime: Timer,
    discharge_timer: Timer,
    /// Have we discharged before?
    discharge_is_first: bool,
    range_initial: f32,
    range_arc: f32,
}

fn stop_channeling(event: On<spells::Release>, mut spells: Query<&mut LightningSpell>) {
    let Ok(mut beam_spell) = spells.get_mut(event.spell) else {
        return;
    };
    beam_spell.lifetime.set_duration(Duration::ZERO);
}

fn apply_lightning(
    spells: Query<(Entity, &mut LightningSpell, Option<&Spell>)>,
    time: Res<Time>,
    spatial_query: SpatialQuery,
    transform_helper: TransformHelper,
    mut commands: Commands,
    // Debug draw
    mut gizmo_assets: ResMut<Assets<GizmoAsset>>,
) {
    for (spell_entity, mut spell, spell_common) in spells {
        if spell.lifetime.is_finished() {
            continue;
        }
        spell.lifetime.tick(time.delta());
        if spell.lifetime.just_finished() {
            commands.trigger(spells::Complete {
                spell: spell_entity,
            });
            commands.entity(spell_entity).try_despawn();
            continue;
        }

        let discharges = if spell.discharge_is_first {
            // For spells that are just starting, always discharge immediately
            spell.discharge_is_first = false;
            1
        } else {
            spell.discharge_timer.tick(time.delta());
            spell.discharge_timer.times_finished_this_tick()
        };

        if discharges > 0 {
            let transform = match transform_helper.compute_global_transform(spell_entity) {
                Ok(t) => t,
                Err(e) => {
                    error_once!(
                        "Lightning spell unable to compute global transform of {spell_entity:?}: {e}"
                    );
                    continue;
                }
            };
            let caster = spell_common.map_or(Entity::PLACEHOLDER, |s| s.caster);
            for _ in 0..discharges {
                discharge_lightning(
                    caster,
                    (&transform, spell.reborrow()),
                    CONE_ANGLE_FORWARD,
                    0,
                    &transform_helper,
                    &spatial_query,
                    commands.reborrow(),
                    // Debug draw
                    gizmo_assets.reborrow(),
                );
            }
        }
    }
}

fn discharge_lightning(
    caster: Entity,
    (spell_trans, spell): (&GlobalTransform, Mut<LightningSpell>),
    cone_angle: f32,
    arcs: u8,
    transform_helper: &TransformHelper,
    spatial_query: &SpatialQuery,
    mut commands: Commands,
    // Debug draw
    mut gizmo_assets: Mut<Assets<GizmoAsset>>,
) {
    let debug_draw = false;

    let range = if arcs == 0 {
        spell.range_initial
    } else {
        spell.range_arc
    };

    let search_radius = spell.range_initial + spell.range_arc;

    let position = spell_trans.translation();
    let direction = spell_trans.rotation() * Dir3::NEG_Z;

    if debug_draw {
        let mut gizmo = GizmoAsset::default();
        gizmo.sphere(position, search_radius, bevy::color::palettes::basic::GREEN);
        commands.spawn((
            Gizmo {
                handle: gizmo_assets.add(gizmo),
                ..default()
            },
            Lifetime::<Virtual>::from_secs(INTERVAL),
        ));
    }

    let mut intersections = BinaryHeap::new();

    spatial_query.shape_intersections_callback(
        &Collider::sphere(search_radius),
        spell_trans.translation(),
        Quat::IDENTITY,
        // TODO: Narrow down this mask
        &SpatialQueryFilter::from_mask(Layers::Default),
        |target| {
            if target == caster {
                return true;
            }

            let target_transform = match transform_helper.compute_global_transform(target) {
                Ok(t) => t,
                Err(e) => {
                    warn_once!("Lightning discharge skipping target {target:?} because transform is unattainable: {e}");
                    return true;
                },
            };

            // Check that they are close enough
            let delta = target_transform.translation() - position;
            let distance_sq = delta.length_squared();
            if distance_sq > range.powi(2) {
                return true;
            }

            let distance = distance_sq.sqrt();
            let target_effective_direction_unnorm = if distance_sq != 0. {
                delta
            } else {
                direction.as_vec3()
            };

            // Check that they are within the cone
            let dot_vs_target = direction.with_y(0.).normalize_or_zero().dot(target_effective_direction_unnorm.with_y(0.).normalize_or_zero());
            let minimum_dot_at_distance = (0.3 * distance).min(cone_angle.cos());
            if dot_vs_target < minimum_dot_at_distance {
                return true;
            }
            let target_effective_direction = if distance != 0. {
                Dir3::new_unchecked(delta / distance)
            } else {
                direction
            };

            intersections.push(CmpBy(Reverse(TotalF32(distance)), DischargeTarget {
                distance,
                direction: target_effective_direction,
                target,
                target_position: target_transform.translation(),
            }));

            true
        }
    );

    let mut i = 0;
    while let Some(entry) = intersections.pop() {
        let target = entry.1;

        if debug_draw {
            let mut gizmo = GizmoAsset::default();
            gizmo.arrow(
                position,
                target.target_position,
                Color::from(bevy::color::palettes::basic::RED).rotate_hue(18. * (i as f32)),
            );
            commands.spawn((
                Gizmo {
                    handle: gizmo_assets.add(gizmo),
                    ..default()
                },
                Lifetime::<Virtual>::from_secs(INTERVAL),
            ));
        }

        // TODO: Consider raycasting

        // TODO: Chaining/jumps

        commands.trigger(LightningStrike {
            path: Segment3d::new(position, target.target_position),
            source: Some(caster),
            target: Some(target.target),
        });

        i += 1;
    }
}

struct CmpBy<K, T>(K, T);

impl<K: PartialEq, T> PartialEq for CmpBy<K, T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<K: Eq, T> Eq for CmpBy<K, T> {}

impl<K: PartialOrd, T> PartialOrd for CmpBy<K, T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl<K: Ord, T> Ord for CmpBy<K, T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct TotalF32(pub f32);

impl From<f32> for TotalF32 {
    fn from(value: f32) -> Self {
        Self(value)
    }
}

impl From<TotalF32> for f32 {
    fn from(value: TotalF32) -> Self {
        value.0
    }
}

impl Eq for TotalF32 {}

impl PartialOrd for TotalF32 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TotalF32 {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.total_cmp(&other.0)
    }
}

#[derive(Debug)]
struct DischargeTarget {
    distance: f32,
    direction: Dir3,
    target: Entity,
    target_position: Vec3,
}

#[derive(Event, Debug)]
pub struct LightningStrike {
    pub path: Segment3d,
    pub source: Option<Entity>,
    pub target: Option<Entity>,
}

mod vfx {
    use super::TotalF32;
    use crate::{magicka_level_model::Layers, spelling::spells::lightning::LightningStrike};
    use avian3d::prelude::{PhysicsLayer, SpatialQuery, SpatialQueryFilter};
    use bevy::prelude::*;
    use rand::SeedableRng as _;

    /// From here: https://github.com/hatoo/blackbody/blob/4733fcd5745266e865241d42940b7bfa873aa0c9/src/lib.rs
    ///
    /// Needs to be replaced because it does not properly handle high temperatures shifting to blue
    ///
    /// Would prefer to use this: https://docs.rs/colorimetry/latest/colorimetry/illuminant/struct.Planck.html
    /// colorimetry depends on an incompatible version of geo.
    mod blackbody;

    pub(super) fn plugin(app: &mut App) {
        app.add_observer(spawn_effect_for_lightning_strike);
        // TODO: Before material is read?
        app.add_systems(
            PostUpdate,
            fade_ionized_air.before(apply_emissive_temperature_to_materials),
        );
        app.add_systems(PostUpdate, apply_emissive_temperature_to_materials);
    }

    fn spawn_effect_for_lightning_strike(
        event: On<LightningStrike>,
        spatial_query: SpatialQuery,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
        mut commands: Commands,
    ) {
        let mut rng = rand::rngs::SmallRng::from_os_rng();

        let [source_position, target_position] = event.path.vertices;
        let distance = source_position.distance(target_position);
        let strike_delta = target_position - source_position;

        let rrt_iters = 64;
        let extend_increment = distance / (rrt_iters as f32) * 20.;
        let ray_filter = SpatialQueryFilter::from_mask(!Layers::Trigger.to_bits())
            .with_excluded_entities(event.source);

        let mut nodes = vec![RrtNode {
            point: source_position,
            parent: None,
            index: 0,
        }];

        let closest_node_distance = std::cell::Cell::new(None);

        let final_node = rrt(
            nodes[0],
            rrt_iters,
            {
                let mut rng = rand::rngs::SmallRng::from_rng(&mut rng);
                // let mut samples = 0;
                let closest_node_distance = &closest_node_distance;
                move || {
                    // let t = (samples as f32) / (rrt_iters.saturating_sub(1) as f32);
                    // samples += 1;
                    // // The closer we are to the sample limit, the closer we should sample
                    // let dis_t = t;

                    // The closer we've gotten to the goal before, the closer we should sample
                    let dis_t =
                        (1.0 - (closest_node_distance.get().unwrap_or(0.) / distance) - 0.1)
                            .max(0.);
                    let distribution_origin = source_position + (0.5 + dis_t * 0.5) * strike_delta;
                    let distribution_radius = (1. - dis_t).powi(2) * 0.5 * (distance);
                    let random = distribution_origin
                        + Sphere::new(distribution_radius).sample_interior(&mut rng);
                    // Bias toward straight line
                    random
                        - 0.1
                            * (random - source_position)
                                .reject_from(target_position - source_position)
                }
            },
            |from, to| {
                let delta = to - from.point;
                let distance_squared = delta.length_squared();
                let to_distance = distance_squared.sqrt();
                let to_direction = Dir3::new_unchecked(delta / to_distance);
                let max_extend_distance = extend_increment;

                let mut extend_to = if max_extend_distance >= to_distance || distance <= 0. {
                    to
                } else {
                    from.point + to_direction * max_extend_distance
                };
                let mut extend_distance = to_distance.min(max_extend_distance);

                if let Some(hit) = spatial_query.cast_ray(
                    from.point,
                    to_direction,
                    extend_distance,
                    false,
                    &ray_filter,
                ) {
                    if event.target.is_some_and(|t| t == hit.entity) {
                        extend_to = target_position;
                        extend_distance = from.point.distance(extend_to);
                    } else {
                        return None;
                    }
                }

                let extend_node = RrtNode {
                    point: extend_to,
                    index: nodes.len(),
                    parent: Some((from.index, 1500.)),
                };
                nodes.push(extend_node);

                let distance_to_goal = extend_node.point.distance(target_position);
                closest_node_distance.update(|prev| {
                    prev.map(|d| d.min(distance_to_goal))
                        .or(Some(distance_to_goal))
                });
                Some(extend_node)
            },
            |node| node.point.distance_squared(target_position) < 0.1f32.powi(2),
        );

        // Tons of energy transfers through the final path
        if let Some(final_node) = final_node {
            let mut node_index = final_node.index;
            while let Some(node) = nodes.get_mut(node_index)
                && let Some((parent_index, ref mut temperature)) = node.parent
            {
                *temperature += 3000.;
                node_index = parent_index;
            }
        }

        // TODO: Increase transfered energy based on the max of how close the child paths get to target

        for node in &nodes {
            if let Some((parent_index, segment_temperature)) = node.parent
                && let Some(parent_node) = nodes.get(parent_index)
            {
                single_lightning_segment(
                    Segment3d::new(parent_node.point, node.point),
                    segment_temperature,
                    meshes.reborrow(),
                    materials.reborrow(),
                    commands.reborrow(),
                );
            }
        }
    }

    fn single_lightning_segment(
        segment: Segment3d,
        temperature: f32,
        mut meshes: Mut<Assets<Mesh>>,
        mut materials: Mut<Assets<StandardMaterial>>,
        mut commands: Commands,
    ) {
        let distance = segment.length();
        if distance <= 0. {
            debug!("Zero distance lightning segment, ignoring");
            return;
        }

        let source_position = segment.point1();
        let target_position = segment.point2();

        let radius = 0.02;

        commands.spawn((
            Mesh3d(
                meshes.add(
                    Cylinder::new(radius, 1.)
                        .mesh()
                        .anchor(bevy::mesh::CylinderAnchor::Bottom)
                        .build()
                        .rotated_by(Quat::from_rotation_x(-0.25 * std::f32::consts::TAU)),
                ),
            ),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::BLACK,
                emissive_exposure_weight: 1.,
                ..default()
            })),
            EmissiveBlackbody { temperature },
            IonizedAir,
            Transform::from_translation(source_position)
                .looking_at(target_position, Vec3::Y)
                .with_scale(Vec3::ONE.with_z(distance)),
        ));
    }

    fn rrt(
        start: RrtNode,
        max_iters: usize,
        mut sample_goal: impl FnMut() -> Vec3,
        mut extend: impl FnMut(RrtNode, Vec3) -> Option<RrtNode>,
        mut is_goal: impl FnMut(RrtNode) -> bool,
    ) -> Option<RrtNode> {
        let mut nodes = vec![start];
        for _ in 0..max_iters {
            let sample = sample_goal();
            // TODO: Optimize nearest node search
            let closest = nodes
                .iter()
                .copied()
                .min_by_key(|n| TotalF32(n.sort_distance_to_config(sample)))
                .expect("not empty, contains start point");
            if let Some(extension) = extend(closest, sample) {
                nodes.push(extension);
                if is_goal(extension) {
                    return Some(extension);
                }
            }
        }
        None
    }

    #[derive(Clone, Copy, Debug)]
    struct RrtNode {
        point: Vec3,
        index: usize,
        /// (RrtNode::index, segment temperature)
        parent: Option<(usize, f32)>,
    }

    impl RrtNode {
        pub fn sort_distance_to_config(self, other: Vec3) -> f32 {
            self.point.distance_squared(other)
        }
    }

    /// The associated material handle must be unique to this entity.
    #[derive(Component, Debug, Reflect)]
    #[require(MeshMaterial3d<StandardMaterial>)]
    struct EmissiveBlackbody {
        /// Temperature in Kelvin
        pub temperature: f32,
    }

    #[derive(Component, Debug, Reflect)]
    struct IonizedAir;

    fn apply_emissive_temperature_to_materials(
        temperatures: Query<
            (&EmissiveBlackbody, &MeshMaterial3d<StandardMaterial>),
            Changed<EmissiveBlackbody>,
        >,
        mut materials: ResMut<Assets<StandardMaterial>>,
    ) {
        for (temperature, material_handle) in temperatures {
            debug_assert!(material_handle.is_strong());
            let Some(material) = materials.get_mut(material_handle) else {
                continue;
            };
            let [x, y, z] = blackbody::temperature_to_xyz(temperature.temperature);
            let intensity = blackbody::temperature_intensity(temperature.temperature);
            let mut rgb = LinearRgba::from(Xyza::xyz(x, y, z));
            rgb.red = rgb.red.max(0.);
            rgb.green = rgb.green.max(0.);
            rgb.blue = rgb.blue.max(0.);
            rgb *= intensity;
            rgb.red = rgb.red.sqrt();
            rgb.green = rgb.green.sqrt();
            rgb.blue = rgb.blue.sqrt();
            material.emissive = rgb;
        }
    }

    fn fade_ionized_air(
        channels: Query<(Entity, &mut EmissiveBlackbody), With<IonizedAir>>,
        time: Res<Time>,
        mut commands: Commands,
    ) {
        let ambient_temp = 294.;
        let min_temp = 1_000.;
        for (entity, mut temp) in channels {
            temp.temperature -= time.delta_secs() * 4. * (temp.temperature - ambient_temp);
            if temp.temperature < min_temp {
                commands.entity(entity).try_despawn();
            }
        }
    }
}
