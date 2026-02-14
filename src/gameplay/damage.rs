use crate::spelling::element::Element;
use bevy::prelude::*;

pub(super) fn plugin(app: &mut App) {
    app.add_observer(apply_damage);
}

#[derive(Component, Debug, Reflect)]
pub struct Health {
    limit: f32,
    damage: f32,
}

impl Health {
    pub fn full(limit: f32) -> Self {
        Self { limit, damage: 0. }
    }

    pub fn dead(limit: f32) -> Self {
        Self {
            limit,
            damage: limit,
        }
    }

    pub fn limit(&self) -> f32 {
        self.limit
    }

    pub fn damage(&self) -> f32 {
        self.damage
    }

    pub fn alive(&self) -> bool {
        self.damage < self.limit
    }

    pub fn current(&self) -> f32 {
        self.limit - self.damage
    }
}

/// Triggers when [`damage`] should be dealt to [`recipient`].
#[derive(EntityEvent)]
pub struct Damage {
    #[event_target]
    pub recipient: Entity,
    pub damage: DamagePayload,
}

/// Triggers when [`subject`] is damaged and has no health remaining.
#[derive(EntityEvent)]
pub struct Die {
    #[event_target]
    pub subject: Entity,
    pub cause: Option<FinalDamage>,
}

/// The damage that caused death.
#[derive(Debug)]
pub struct FinalDamage {
    pub overkill: f32,
    pub damage: DamagePayload,
}

#[derive(Debug, Clone)]
pub struct DamagePayload {
    pub damage_type: DamageType,
    pub amount: f32,
    pub source: DamageSource,
    /// Is this damage so insignificant or frequent that it shouldn't trigger per-hit effects?
    pub silent: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum DamageType {
    Element(Element),
    True, // AKA Tachyon
}

pub type DamageSource = Option<Entity>;

fn apply_damage(event: On<Damage>, mut recipients: Query<&mut Health>, mut commands: Commands) {
    let Damage {
        recipient,
        ref damage,
    } = *event;

    let Ok(mut health) = recipients.get_mut(recipient) else {
        return;
    };
    if !health.alive() {
        return;
    }

    if !damage.silent {
        debug!("damage {recipient:?} for {damage:?}");
    }

    let can_overheal = false;

    let mut result_damage = health.damage + damage.amount;

    if !can_overheal {
        result_damage = result_damage.max(0.);
    };

    if result_damage >= health.limit {
        let overkill = result_damage - health.limit;
        result_damage = health.limit;

        // To implement multiple healthbars, we can trigger a WillDie event first
        // that has the chance to heal before death

        commands.trigger(Die {
            subject: recipient,
            cause: Some(FinalDamage {
                overkill,
                damage: damage.clone(),
            }),
        });
    }

    health.damage = result_damage;
}
