use crate::spelling::element::{Element, Magnitudes};

pub fn spell_forward(elements: &Magnitudes) -> Spell {
    use Element::*;
    match spell_kind(elements) {
        _ if elements.contains(Lok) => Spell::Beam,
        SpellKind::Shield => shield_spell(RegionWithWeapon::Arc, elements),
        SpellKind::Solid if elements.contains(Ice) && !elements.contains(Earth) => {
            Spell::ProjectileRepeater
        }
        SpellKind::Solid => Spell::ProjectileCharge,
        SpellKind::Energy => Spell::Beam,
        SpellKind::Lightning => Spell::Lightning(RegionWithoutWeapon::Arc),
        SpellKind::Volume => Spell::Spray,
        SpellKind::None => Spell::Push(RegionWithoutWeapon::Arc),
        // This doesn't make sense but matches M2
        // It's unreachable unless more elements are added, because Lok is checked first
        SpellKind::Undefined => Spell::Spray,
    }
}

pub fn spell_area(elements: &Magnitudes) -> Spell {
    match spell_kind(elements) {
        SpellKind::Shield => shield_spell(RegionWithWeapon::Circle, elements),
        SpellKind::Solid | SpellKind::Energy | SpellKind::Volume | SpellKind::Undefined => {
            Spell::Nova
        }
        SpellKind::Lightning => Spell::Lightning(RegionWithoutWeapon::Circle),
        SpellKind::None => Spell::Push(RegionWithoutWeapon::Circle),
    }
}

pub fn spell_self(elements: &Magnitudes) -> Option<Spell> {
    Some(match spell_kind(elements) {
        SpellKind::Shield if contains_any_except(elements, Element::Shield) => {
            Spell::SelfShieldToggleNeutral
        }
        SpellKind::Shield => Spell::SelfShieldSet,
        SpellKind::Solid => Spell::ProjectileDrop,
        // XXX: Could introduce a new spell here
        SpellKind::None => return None,
        SpellKind::Energy | SpellKind::Lightning | SpellKind::Volume | SpellKind::Undefined => {
            if elements.contains(Element::Life) {
                Spell::SelfHeal
            } else {
                Spell::SelfDirect
            }
        }
    })
}

pub fn spell_weapon(elements: &Magnitudes) -> Option<Spell> {
    match spell_kind(elements) {
        SpellKind::Shield => Some(shield_spell(RegionWithWeapon::Line, elements)),
        SpellKind::Solid => Some(Spell::WeaponFissure),
        SpellKind::Energy => Some(Spell::WeaponHeavy),
        SpellKind::Lightning => Some(Spell::WeaponRegular),
        SpellKind::Volume => Some(Spell::WeaponRegular),
        SpellKind::Undefined => None,
        // There are no elements imbued to the weapon
        SpellKind::None => None,
    }
}

fn shield_spell(region: RegionWithWeapon, elements: &Magnitudes) -> Spell {
    if contains_solid(elements) {
        Spell::Barrier(region)
    } else if !elements.contains(Element::Lightning) && contains_energy(elements) {
        Spell::Mine(region)
    } else if contains_any_except(elements, Element::Shield) {
        Spell::Storm(region)
    } else {
        Spell::Shield(region)
    }
}

#[derive(Debug, PartialEq)]
pub enum SpellKind {
    Shield,
    Solid,
    Energy,
    Lightning,
    Volume,
    /// i.e. only Lok
    Undefined,
    /// There are no elements
    None,
}

impl SpellKind {
    pub fn from(elements: &Magnitudes) -> Self {
        spell_kind(elements)
    }
}

fn spell_kind(elements: &Magnitudes) -> SpellKind {
    use Element::*;
    if elements.is_empty() {
        SpellKind::None
    } else if elements.contains(Shield) {
        SpellKind::Shield
    } else if contains_solid(elements) {
        SpellKind::Solid
    } else if contains_energy(elements) {
        SpellKind::Energy
    } else if elements.contains(Steam) {
        SpellKind::Volume
    } else if elements.contains(Lightning) {
        SpellKind::Lightning
    } else if contains_volume(elements) {
        // At this point, this condition is equivalent to contains_any_except(Lok)
        SpellKind::Volume
    } else {
        SpellKind::Undefined
    }
}

fn contains_solid(elements: &Magnitudes) -> bool {
    elements.contains(Element::Earth) || elements.contains(Element::Ice)
}

fn contains_energy(elements: &Magnitudes) -> bool {
    elements.contains(Element::Arcane) || elements.contains(Element::Life)
}

fn contains_volume(elements: &Magnitudes) -> bool {
    use Element::*;
    elements.contains(Water)
        || elements.contains(Cold)
        || elements.contains(Fire)
        || elements.contains(Steam)
        || elements.contains(Poison)
}

fn contains_any_except(elements: &Magnitudes, element: Element) -> bool {
    elements.total() > (elements.get(element) as usize)
}

#[derive(Debug, Clone)]
pub enum Spell {
    Push(RegionWithoutWeapon),
    Beam,
    Lightning(RegionWithoutWeapon),
    Spray,
    Nova,
    /// Charge up projectile(s) launched forward.
    ProjectileCharge,
    /// Channel to continuously fire projectiles. Magicka 1 does [`ProjectileCharge`] instead.
    ProjectileRepeater,
    /// Immediately launch projectile(s) at yourself from above.
    ProjectileDrop,
    SelfDirect,
    SelfShieldToggleNeutral,
    SelfShieldSet,
    /// Channel to heal. Magicka 1 does [`SelfDirect`] instead.
    SelfHeal,
    Shield(RegionWithWeapon),
    Storm(RegionWithWeapon),
    Mine(RegionWithWeapon),
    Barrier(RegionWithWeapon),
    /// Imbue the elements into the weapon.
    Imbue,
    WeaponFissure,
    WeaponRegular,
    WeaponHeavy,
    Magick(Magick),
}

#[derive(Debug, Clone)]
pub enum Magick {}

#[derive(Debug, Clone, Copy)]
pub enum RegionWithWeapon {
    Arc,
    Circle,
    Line,
}

#[derive(Debug, Clone, Copy)]
pub enum RegionWithoutWeapon {
    Arc,
    Circle,
}
