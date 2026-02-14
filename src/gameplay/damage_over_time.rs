use std::time::Duration;

use bevy::prelude::*;

use crate::gameplay::damage::{Damage, DamagePayload};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(FixedUpdate, deal_damage_over_time);
}

/// Apply [`damage`] to [`target`] over time.
#[derive(Component, Debug)]
pub struct DamageOverTime {
    pub target: Target,
    pub damage: DamagePayload,
    pub timer: Timer,
}

#[derive(Debug, Clone, Copy)]
pub enum Target {
    This,
    Parent,
    Entity(Entity),
}

impl DamageOverTime {
    /// Interpret [`DamagePayload::amount`] as damage per second, applied continuously.
    ///
    /// You want to set [`DamagePayload::silent`].
    pub fn continuous(target: Target, damage: DamagePayload) -> Self {
        Self {
            target,
            damage,
            timer: Timer::new(Duration::ZERO, TimerMode::Once),
        }
    }

    /// Apply [`damage`] every [`interval`].
    pub fn every(interval: Duration, target: Target, damage: DamagePayload) -> Self {
        Self {
            target,
            damage,
            timer: Timer::new(interval, TimerMode::Repeating),
        }
    }
}

fn deal_damage_over_time(
    dots: Query<(Entity, &mut DamageOverTime, Option<&ChildOf>)>,
    time: Res<Time>,
    mut commands: Commands,
) {
    let delta = time.delta();
    for (this, mut dot, parent) in dots {
        let instances_this_tick = if dot.timer.duration().is_zero() {
            1
        } else {
            dot.timer.tick(delta);
            dot.timer.times_finished_this_tick()
        };

        let target = match dot.target {
            Target::This => this,
            Target::Parent => {
                if let Some(parent) = parent {
                    parent.parent()
                } else {
                    return;
                }
            }
            Target::Entity(entity) => entity,
        };

        for _ in 0..instances_this_tick {
            let mut damage = dot.damage.clone();
            // Interpret amount as hp/s if there's no interval
            if dot.timer.duration().is_zero() {
                damage.amount *= delta.as_secs_f32();
            }

            commands.trigger(Damage {
                recipient: target,
                damage,
            });
        }
    }
}
