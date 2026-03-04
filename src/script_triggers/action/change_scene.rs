use bevy::prelude::*;

#[derive(Clone, Debug)]
pub struct ChangeScene {
    pub scene: String,
    pub transition: SceneChangeTransition,
    pub transition_time: f32,
    pub spawn_players: bool,
    pub spawn_point: Option<String>,
    pub save_npcs: bool,
}

#[derive(Clone, Debug)]
pub enum SceneChangeTransition {
    None,
    Fade,
    CrossFade,
}

pub fn execute_change_scene(
    action: In<impl AsRef<ChangeScene>>,
    world: &mut World,
    q_scenes: &mut QueryState<(Entity, &crate::scene::Scene)>,
) -> Result {
    let ChangeScene {
        scene,
        transition: _,
        transition_time: _,
        spawn_players,
        spawn_point,
        save_npcs: _,
    } = action.as_ref();

    let (_current_scene_entity, current_scene) = q_scenes.single(world).unwrap();
    // XXX: It would be nicer to attach a scene transition component to current_scene_entity
    // than add something global
    let change = crate::scene::ChangeScene {
        level: current_scene.level.clone(),
        scene: scene.clone(),
        spawn_players: *spawn_players,
        spawn_point: spawn_point.clone(),
    };
    world.run_system_cached_with(crate::scene::queue_delayed_scene_change, change)?;
    Ok(())
}

pub(crate) fn from_xml(attributes: Vec<xml::attribute::OwnedAttribute>) -> Result<ChangeScene, ()> {
    let mut scene: Option<String> = None;
    let mut transition = SceneChangeTransition::Fade;
    let mut transition_time = 1.0f32;
    let mut spawn_players = false;
    let mut spawn_point: Option<String> = None;
    let mut save_npcs = false;
    for xml::attribute::OwnedAttribute { name, value } in attributes {
        if name.local_name.eq_ignore_ascii_case("scene") {
            scene = Some(value);
        } else if name.local_name.eq_ignore_ascii_case("transition") {
            if value.eq_ignore_ascii_case("none") {
                transition = SceneChangeTransition::None;
            } else if value.eq_ignore_ascii_case("fade") {
                transition = SceneChangeTransition::Fade;
            } else if value.eq_ignore_ascii_case("crossfade") {
                transition = SceneChangeTransition::CrossFade;
            } else {
                todo!("invalid changescene action transition error")
            }
        } else if name.local_name.eq_ignore_ascii_case("transitiontime") {
            transition_time = value.parse().unwrap();
        } else if name.local_name.eq_ignore_ascii_case("spawnplayers") {
            spawn_players = value.parse().unwrap();
        } else if name.local_name.eq_ignore_ascii_case("spawnpoint") {
            spawn_point = Some(value);
        } else if name.local_name.eq_ignore_ascii_case("savenpcs") {
            save_npcs = value.parse().unwrap();
        }
    }
    Ok(ChangeScene {
        scene: scene.ok_or(())?,
        transition,
        transition_time,
        spawn_players,
        spawn_point,
        save_npcs,
    })
}
