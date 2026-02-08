use bevy::prelude::*;

pub mod beam;

// XXX: Consider using relationships here

#[derive(Component, Debug, Reflect)]
pub struct Spell {
    pub caster: Entity,
}

#[derive(EntityEvent, Debug)]
pub struct Release {
    #[event_target]
    pub spell: Entity,
}

#[derive(EntityEvent, Debug)]
pub struct Complete {
    #[event_target]
    pub spell: Entity,
}
