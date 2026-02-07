use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;

use crate::spelling::element::BaseElement;

#[derive(Component, TypePath)]
pub struct SpellingInput;

#[derive(InputAction, Default, TypePath)]
#[action_output(bool)]
pub struct ConjureElement<Elem: ElementType> {
    _elem: std::marker::PhantomData<Elem>,
}
pub trait ElementType: Sized + 'static {
    const ELEMENT: BaseElement;
}
#[derive(TypePath)]
pub struct WaterElement;
impl ElementType for WaterElement {
    const ELEMENT: BaseElement = BaseElement::Water;
}
pub struct LifeElement;
impl ElementType for LifeElement {
    const ELEMENT: BaseElement = BaseElement::Life;
}
pub struct ShieldElement;
impl ElementType for ShieldElement {
    const ELEMENT: BaseElement = BaseElement::Shield;
}
pub struct ColdElement;
impl ElementType for ColdElement {
    const ELEMENT: BaseElement = BaseElement::Cold;
}
pub struct LightningElement;
impl ElementType for LightningElement {
    const ELEMENT: BaseElement = BaseElement::Lightning;
}
pub struct ArcaneElement;
impl ElementType for ArcaneElement {
    const ELEMENT: BaseElement = BaseElement::Arcane;
}
pub struct EarthElement;
impl ElementType for EarthElement {
    const ELEMENT: BaseElement = BaseElement::Earth;
}
pub struct FireElement;
impl ElementType for FireElement {
    const ELEMENT: BaseElement = BaseElement::Fire;
}

// M1: Left click / left stick
// M2: Left click / left stick
//Move

// M1, M2: Cursor / right stick
//Aim

// M1: Shift + left click / a (green) ~ with no elements
// M2: Shift + right click
//WeaponAttack

// M1: Ctrl / right shoulder
//Block

// M1: Space
//BreakFree

// M1: Right click / right trigger
// M2: (NOT Shift) + Right click
#[derive(InputAction)]
#[action_output(bool)]
pub struct CastForward;

// M1: Shift + right click / left trigger
// M2: Shift + middle click / left trigger
#[derive(InputAction)]
#[action_output(bool)]
pub struct CastArea;

// M1: Middle click / y (yellow)
// M2: (NOT Shift) + Middle click / right shoulder
#[derive(InputAction)]
#[action_output(bool)]
pub struct CastSelf;

// M1: Shift + left click / a (green) ~ with elements
// M2: Shift + right click
#[derive(InputAction)]
#[action_output(bool)]
pub struct CastImbue;

// M1: Space / b (red)
// M2: Space / right trigger
#[derive(InputAction)]
#[action_output(bool)]
pub struct CastMagick;

// M1: Mouse wheel / d-pad
#[derive(InputAction)]
#[action_output(bool)]
pub struct CycleMagick;

// M1: Space / b
// M2: Space
#[derive(InputAction)]
#[action_output(bool)]
pub struct BoostShield;

// M1: Middle click / y (yellow) ~ with no elements
//StaffAbility

// M1, M2: Left click / right trigger
//BreakFree

pub fn bindings_m1() -> impl Bundle {
    actions!(SpellingInput[
        (
            Action::<CastForward>::new(),
            bindings![
                MouseButton::Right,
                GamepadButton::RightTrigger2,
            ],
        ),
        (
            Action::<CastArea>::new(),
            bindings![
                MouseButton::Right.with_mod_keys(ModKeys::SHIFT),
                GamepadButton::LeftTrigger2,
            ],
            ActionSettings {
                consume_input: true,
                ..default()
            }
        ),
        (
            Action::<CastSelf>::new(),
            bindings![
                MouseButton::Middle,
                GamepadButton::North,
            ],
        ),
        (
            Action::<CastImbue>::new(),
            bindings![
                MouseButton::Left.with_mod_keys(ModKeys::SHIFT),
                GamepadButton::South,
            ],
            ActionSettings {
                consume_input: true,
                ..default()
            }
        ),
        (
            Action::<CastMagick>::new(),
            bindings![
                KeyCode::Space,
                GamepadButton::East,
            ],
        ),
        (
            Action::<BoostShield>::new(),
            bindings![
                KeyCode::Space,
                GamepadButton::East,
            ],
        ),
        (
            Action::<ConjureElement<WaterElement>>::new(),
            bindings![KeyCode::KeyQ],
        ),
        (
            Action::<ConjureElement<LifeElement>>::new(),
            bindings![KeyCode::KeyW],
        ),
        (
            Action::<ConjureElement<ShieldElement>>::new(),
            bindings![KeyCode::KeyE],
        ),
        (
            Action::<ConjureElement<ColdElement>>::new(),
            bindings![KeyCode::KeyR],
        ),
        (
            Action::<ConjureElement<LightningElement>>::new(),
            bindings![KeyCode::KeyA],
        ),
        (
            Action::<ConjureElement<ArcaneElement>>::new(),
            bindings![KeyCode::KeyS],
        ),
        (
            Action::<ConjureElement<EarthElement>>::new(),
            bindings![KeyCode::KeyD],
        ),
        (
            Action::<ConjureElement<FireElement>>::new(),
            bindings![KeyCode::KeyF],
        ),
    ])
}
