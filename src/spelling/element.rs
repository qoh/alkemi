use bevy::reflect::Reflect;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
// #[reflect(from_reflect = false)]
pub enum Element {
    // Base
    Water,
    Life,
    Shield,
    Cold,
    Lightning,
    Arcane,
    Earth,
    Fire,
    // Hybrid
    Steam,
    Ice,
    Poison, // (hybrid only in Magicka 2)
    // Special
    Lok,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BaseElement {
    Water,
    Life,
    Shield,
    Cold,
    Lightning,
    Arcane,
    Earth,
    Fire,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HybridElement {
    Steam,
    Ice,
    Poison, // (hybrid only in Magicka 2)
}

impl Element {
    pub fn all() -> &'static [Element] {
        use Element::*;
        &[
            Water, Life, Shield, Cold, Lightning, Arcane, Earth, Fire, Steam, Ice, Poison, Lok,
        ]
    }
}

impl From<BaseElement> for Element {
    fn from(value: BaseElement) -> Self {
        match value {
            BaseElement::Water => Element::Water,
            BaseElement::Life => Element::Life,
            BaseElement::Shield => Element::Shield,
            BaseElement::Cold => Element::Cold,
            BaseElement::Lightning => Element::Lightning,
            BaseElement::Arcane => Element::Arcane,
            BaseElement::Earth => Element::Earth,
            BaseElement::Fire => Element::Fire,
        }
    }
}

impl TryFrom<Element> for BaseElement {
    type Error = WrongElementKind;

    fn try_from(value: Element) -> Result<Self, Self::Error> {
        Ok(match value {
            Element::Water => BaseElement::Water,
            Element::Life => BaseElement::Life,
            Element::Shield => BaseElement::Shield,
            Element::Cold => BaseElement::Cold,
            Element::Lightning => BaseElement::Lightning,
            Element::Arcane => BaseElement::Arcane,
            Element::Earth => BaseElement::Earth,
            Element::Fire => BaseElement::Fire,
            _ => return Err(WrongElementKind),
        })
    }
}

impl From<HybridElement> for Element {
    fn from(value: HybridElement) -> Self {
        match value {
            HybridElement::Steam => Element::Steam,
            HybridElement::Ice => Element::Ice,
            HybridElement::Poison => Element::Poison,
        }
    }
}

impl TryFrom<Element> for HybridElement {
    type Error = WrongElementKind;

    fn try_from(value: Element) -> Result<Self, Self::Error> {
        Ok(match value {
            Element::Steam => HybridElement::Steam,
            Element::Ice => HybridElement::Ice,
            Element::Poison => HybridElement::Poison,
            _ => return Err(WrongElementKind),
        })
    }
}

#[derive(Debug)]
pub struct WrongElementKind;

// Rules for opposites & combinations

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reaction {
    Opposite,
    Combine(HybridElement),
    Breakdown(BaseElement),
}

pub fn reaction(into: Element, add: BaseElement) -> Option<Reaction> {
    if let Ok(into_base) = BaseElement::try_from(into)
        && opposes(into_base, add)
    {
        Some(Reaction::Opposite)
    } else if let Ok(into_base) = BaseElement::try_from(into)
        && let Some(hybrid) = combine(into_base, add)
    {
        Some(Reaction::Combine(hybrid))
    } else if let Ok(into_hybrid) = HybridElement::try_from(into)
        && let Some(result) = breakdown(into_hybrid, add)
    {
        Some(Reaction::Breakdown(result))
    } else {
        None
    }
}

fn opposes(a: BaseElement, b: BaseElement) -> bool {
    fn inner(a: BaseElement, b: BaseElement) -> bool {
        use BaseElement::*;
        matches!(
            (a, b),
            (Water, Lightning)
                | (Life, Arcane)
                | (Shield, Shield)
                | (Cold, Fire)
                | (Lightning, Earth)
        )
    }
    inner(a, b) || inner(b, a)
}

fn combine(element: BaseElement, into: BaseElement) -> Option<HybridElement> {
    fn inner(a: BaseElement, b: BaseElement) -> Option<HybridElement> {
        use {BaseElement::*, HybridElement::*};
        match (a, b) {
            (Water, Cold) => Some(Ice),
            (Water, Fire) => Some(Steam),
            (Water, Arcane) => Some(Poison),
            _ => None,
        }
    }
    inner(element, into).or(inner(into, element))
}

fn breakdown(hybrid: HybridElement, with: BaseElement) -> Option<BaseElement> {
    use {BaseElement::*, HybridElement::*};
    match (hybrid, with) {
        // (these all go to water, not a copy error)
        (Ice, Fire) => Some(Water),
        (Steam, Cold) => Some(Water),
        (Poison, Life) => Some(Water),
        _ => None,
    }
}

/// Element multiset - unordered magnitudes. Saturates on overflow.
#[derive(Debug, Clone, Default, PartialEq, Eq, Reflect)]
pub struct Magnitudes {
    #[reflect(ignore)]
    counts: std::collections::HashMap<Element, u8>,
}

impl Magnitudes {
    pub fn get(&self, element: Element) -> u8 {
        self.counts.get(&element).copied().unwrap_or_default()
    }

    pub fn contains(&self, element: Element) -> bool {
        self.counts.contains_key(&element)
    }

    pub fn total(&self) -> usize {
        self.counts.values().map(|x| *x as usize).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.counts.is_empty()
    }
}

impl FromIterator<Element> for Magnitudes {
    fn from_iter<T: IntoIterator<Item = Element>>(iter: T) -> Self {
        let mut mset = Self {
            counts: Default::default(),
        };
        for item in iter {
            let count = mset.counts.entry(item).or_default();
            *count = count.saturating_add(1);
        }
        mset
    }
}
