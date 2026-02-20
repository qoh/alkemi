use std::{marker::PhantomData, time::Duration};

use bevy::{prelude::*, time::Stopwatch};

pub fn plugin(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        (tick_elapsed::<Fixed>, despawn_lifetimes::<Fixed>).chain(),
    );
    app.add_systems(
        Update,
        (tick_elapsed::<Virtual>, despawn_lifetimes::<Virtual>).chain(),
    );
}

/// Time in [`T`] elapsed since this component was added.
#[derive(Component, Clone, Default, Debug, Reflect)]
pub struct Elapsed<T: Send + Sync = Fixed> {
    pub time: Stopwatch,
    _marker: PhantomData<T>,
}

impl<T: Send + Sync> AsRef<Stopwatch> for Elapsed<T> {
    fn as_ref(&self) -> &Stopwatch {
        &self.time
    }
}

impl<T: Send + Sync> AsMut<Stopwatch> for Elapsed<T> {
    fn as_mut(&mut self) -> &mut Stopwatch {
        &mut self.time
    }
}

/// Maximum amount of time in [`T`] that may elapse (in [`Elapsed<T>`]) before the entity is despawned.
#[derive(Component, Clone, Debug, Reflect)]
#[require(Elapsed<T>)]
pub struct Lifetime<T: Default + Send + Sync = Fixed> {
    pub limit: Duration,
    _marker: PhantomData<T>,
}

impl<T: Default + Send + Sync> From<Duration> for Lifetime<T> {
    fn from(limit: Duration) -> Self {
        Self {
            limit,
            _marker: default(),
        }
    }
}

impl<T: Default + Send + Sync> Lifetime<T> {
    pub fn from_secs(secs: f32) -> Self {
        Duration::from_secs_f32(secs).into()
    }
}

fn tick_elapsed<T: Default + Send + Sync + 'static>(
    elapseds: Query<&mut Elapsed<T>>,
    time: Res<Time<T>>,
) {
    for mut elapsed in elapseds {
        elapsed.time.tick(time.delta());
    }
}

fn despawn_lifetimes<T: Default + Send + Sync + 'static>(
    lifetimes: Query<(Entity, &Elapsed<T>, &Lifetime<T>)>,
    mut commands: Commands,
) {
    for (entity, elapsed, lifetime) in lifetimes {
        if elapsed.time.elapsed() >= lifetime.limit {
            commands.entity(entity).try_despawn();
        }
    }
}
