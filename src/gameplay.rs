use bevy::prelude::*;

pub mod damage;
pub mod damage_over_time;

pub fn plugin(app: &mut App) {
    app.add_plugins(damage::plugin);
    app.add_plugins(damage_over_time::plugin);
}
