mod casting;
mod chanting;
mod chanting_ui;
mod chanting_vfx;
mod color;
mod element;
mod input;
mod spells;
mod status;

pub use input::bindings_m1;

use bevy::prelude::*;
use bevy_enhanced_input::prelude::InputContextAppExt as _;

pub fn plugin(app: &mut App) {
    app.add_input_context::<input::SpellingInput>();
    app.register_type::<bevy_enhanced_input::prelude::Actions<input::SpellingInput>>();

    app.add_plugins((chanting::plugin, chanting_ui::plugin, chanting_vfx::plugin));
    app.add_plugins(casting::plugin);
    app.add_plugins(spells::beam::plugin);
}

pub fn bundle_m1() -> impl Bundle {
    (
        chanting::ElementQueue {
            queued_elements: Default::default(),
            limit: 5,
            combine_in_queue: true,
            combine_poison: false,
            lightning_cancels_water_first: false,
        },
        casting::Caster::default(),
        input::SpellingInput,
    )
}
