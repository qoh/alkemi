use crate::spelling::{
    element::{self, BaseElement, Element, HybridElement, Reaction},
    input,
    status::Wet,
};

use bevy::prelude::*;
use bevy_enhanced_input::prelude::Start;

pub fn plugin(app: &mut App) {
    app.add_observer(conjure_element::<input::WaterElement>)
        .add_observer(conjure_element::<input::LifeElement>)
        .add_observer(conjure_element::<input::ShieldElement>)
        .add_observer(conjure_element::<input::ColdElement>)
        .add_observer(conjure_element::<input::LightningElement>)
        .add_observer(conjure_element::<input::ArcaneElement>)
        .add_observer(conjure_element::<input::EarthElement>)
        .add_observer(conjure_element::<input::FireElement>);
    app.add_observer(shock_on_become_wet);
}

/// Lightning queued with wet status
// TODO: Do something with this
#[derive(EntityEvent)]
pub struct ShockSelf(pub Entity);

#[derive(Component, Debug, Reflect)]
// #[reflect(from_reflect = false)]
pub struct ElementQueue {
    // #[reflect(default)]
    pub(super) queued_elements: Vec<element::Element>,
    pub(super) limit: u8,
    pub(super) combine_in_queue: bool,
    pub(super) combine_poison: bool,
    pub(super) lightning_cancels_water_first: bool,
}

fn conjure_element<Elem: input::ElementType>(
    action: On<Start<input::ConjureElement<Elem>>>,
    mut chanters: Query<(&mut ElementQueue, Has<Wet>)>,
    mut commands: Commands,
) {
    let Ok((mut element_queue, is_wet)) = chanters.get_mut(action.context) else {
        debug!("no element_queue on chant input");
        return;
    };

    let element = Elem::ELEMENT;

    // TODO: Per-element cooldown
    // In M1 it is 0.125 s
    // In MWW it... ?
    // In M2 it seems to depends on artifacts?
    let element_on_cooldown = false;
    if element_on_cooldown {
        trace!("element chanted is on cooldown");
        return;
    }

    if element == BaseElement::Lightning && is_wet {
        trace!("element chanted triggers self shock");
        commands.trigger(ShockSelf(action.context));
        return;
    }

    // Try to cancel out a base opposite
    if element_queue.lightning_cancels_water_first {
        // In Magicka 2, lightning prefers to cancel water before earth
        if element == BaseElement::Lightning && element_queue.remove_last_of(Element::Water) {
            return;
        }
    }
    if let Some(last_opposite_index) = element_queue
        .queued_elements
        .iter()
        .enumerate()
        .rev()
        .filter_map(|(i, queued)| {
            element::reaction(*queued, element)
                .and_then(|react| matches!(react, Reaction::Opposite).then_some(i))
        })
        .next()
    {
        element_queue.queued_elements.remove(last_opposite_index);
        return;
    }

    // Try to break down a hybrid
    if let Some((breakdown_index, breakdown_into)) = element_queue
        .queued_elements
        .iter()
        .enumerate()
        .rev()
        .filter_map(|(i, queued)| {
            element::reaction(*queued, element).and_then(|react| {
                if let Reaction::Breakdown(into) = react {
                    Some((i, into))
                } else {
                    None
                }
            })
        })
        .next()
    {
        // We're replacing breakdown_index with breakdown_into
        // Would this cause breakdown_into to be queued with an opposite?
        // element_queue.queued_elements.remove(last_opposite_index);
        if let Some(last_opposite_index) = element_queue
            .queued_elements
            .iter()
            .enumerate()
            .rev()
            .filter_map(|(i, queued)| {
                element::reaction(*queued, breakdown_into)
                    .and_then(|react| matches!(react, Reaction::Opposite).then_some(i))
            })
            .next()
        {
            debug_assert_ne!(last_opposite_index, breakdown_index);
            // Remove the higher index first, since removing the lower index will shift the higher index
            element_queue
                .queued_elements
                .remove(last_opposite_index.max(breakdown_index));
            element_queue
                .queued_elements
                .remove(last_opposite_index.min(breakdown_index));
        } else {
            element_queue.queued_elements[breakdown_index] = breakdown_into.into();
            if element_queue.combine_in_queue {
                // TODO: Should this check if breakdown_into would combine with a queued base into a different hybrid?
            }
        }
        return;
    }

    // Try to combine with another base into a hybrid
    if element_queue.combine_in_queue
        && let Some((other_index, combine_into)) = element_queue
            .queued_elements
            .iter()
            .enumerate()
            .rev()
            .filter_map(|(i, queued)| {
                element::reaction(*queued, element).and_then(|react| {
                    if let Reaction::Combine(into) = react
                        && (into != HybridElement::Poison || element_queue.combine_poison)
                    {
                        Some((i, into))
                    } else {
                        None
                    }
                })
            })
            .next()
    {
        element_queue.queued_elements[other_index] = combine_into.into();
        return;
    }

    // Try to add an element
    if element_queue.queued_elements.len() >= element_queue.limit as usize {
        debug!("element queue is full");
        // Error: The queue is full
        return;
    }

    element_queue.queued_elements.push(element.into());
}

fn shock_on_become_wet(
    action: On<Add, Wet>,
    mut chanters: Query<&mut ElementQueue>,
    mut commands: Commands,
) {
    let Ok(mut element_queue) = chanters.get_mut(action.entity) else {
        return;
    };

    // Remove all lightning, checking if there was any
    let mut had_lightning = false;
    element_queue.queued_elements.retain(|e| {
        let is_lightning = *e == Element::Lightning;
        had_lightning |= is_lightning;
        !is_lightning
    });
    if had_lightning {
        commands.trigger(ShockSelf(action.entity));
    }
}

impl ElementQueue {
    /// Dequeue one instance of `elem`. Returns `true` if there was at least one `elem` to remove.
    fn remove_last_of(&mut self, elem: Element) -> bool {
        if let Some(index_of_last_elem) = self.queued_elements.iter().rposition(|e| *e == elem) {
            self.queued_elements.remove(index_of_last_elem);
            true
        } else {
            false
        }
    }
}
