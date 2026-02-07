use bevy::{
    color::{Alpha as _, ColorToComponents as _, LinearRgba},
    math::Vec3,
};

use crate::spelling::element::{Element, Magnitudes};

pub fn element_color(element: Element) -> LinearRgba {
    LinearRgba::from_vec3(element_color_rgb(element))
}

fn element_color_rgb(element: Element) -> Vec3 {
    use Element::*;
    match element {
        Water => (0., 0.7, 1.3),
        Life => (0.2, 1.6, 0.2),
        Shield => (2., 1.5, 1.),
        Cold => (1., 1., 1.4),
        Lightning => (0.75, 0.5, 1.),
        Arcane => (2., 0.4, 0.6),
        Earth => (0.3, 0.2, 0.1),
        Fire => (1.8, 0.6, 0.4),
        Steam => (1., 1., 1.),
        Ice => (0.8, 0.9, 1.4),
        Poison => (1., 1.2, 0.),
        // Made up value
        Lok => (0.2, 0.3, 0.3),
    }
    .into()
}

/// Returns None if Magnitudes is empty.
pub fn spell_color(elements: &Magnitudes) -> Option<LinearRgba> {
    let mut rgb = Vec3::ZERO;
    let mut components = 0;
    for element in Element::all().iter().copied() {
        if matches!(element, Element::Lightning | Element::Fire) {
            let count = elements.get(element);
            if count != 0 {
                rgb += element_color_rgb(element);
                components += count as usize;
            }
        } else if elements.contains(element) {
            rgb += element_color_rgb(element);
            components += 1;
        }
    }
    if components == 0 {
        None
    } else {
        Some(LinearRgba::from_vec3(rgb / (components as f32)))
    }
}

pub fn normalize_color(color: LinearRgba) -> (LinearRgba, f32) {
    let (rgb, magnitude) = color.to_vec3().normalize_and_length();
    let color = LinearRgba::from_vec3(rgb).with_alpha(color.alpha);
    (color, magnitude)
}
