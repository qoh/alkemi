use bevy::{ecs::system::SystemState, prelude::*};

#[derive(Clone, Debug)]
pub struct SpawnCharacter {
    pub area: String,
    pub type_name: String,
    pub nr: u16,
    pub id: Option<String>,
}

pub fn execute_spawn_character(
    action: In<impl AsRef<SpawnCharacter>>,
    world: &mut World,
    q_scenes: &mut QueryState<(Entity, &crate::scene::Scene)>,
    q_locators: &mut QueryState<(Entity, &Name), With<crate::magicka_level_model::Locator>>,
    q_areas: &mut QueryState<(Entity, &Name), With<crate::magicka_level_model::TriggerArea>>,
    transform_helper: &mut SystemState<TransformHelper>,
) -> Result {
    let SpawnCharacter {
        area: area_name,
        type_name,
        id,
        nr,
    } = action.as_ref();

    let (spawner_entity, sample_space) = if let Some((locator, _name)) = q_locators
        .query(world)
        .iter()
        .find(|(_, n)| n.eq_ignore_ascii_case(area_name))
    {
        (locator, false)
    } else if let Some((area, _name)) = q_areas
        .query(world)
        .iter()
        .find(|(_, n)| n.eq_ignore_ascii_case(area_name))
    {
        (area, true)
    } else {
        warn!("Spawn character trigger action can't find locator/area {area_name:?}");
        return Ok(());
    };
    let spawner_transform = transform_helper
        .get(world)
        .compute_global_transform(spawner_entity)
        .unwrap();
    let (current_scene_entity, _current_scene) = q_scenes.single(world).unwrap();
    let id = id.clone();
    let type_name = type_name.clone();
    for _ in 0..*nr {
        let spawn_transform = if sample_space {
            use rand::{SeedableRng as _, distr::uniform::SampleRange as _};
            use std::f32::consts::TAU;
            let mut rng = rand::rngs::SmallRng::from_os_rng();
            let local_point = vec3(
                (-1.0..=1.0).sample_single(&mut rng).unwrap(),
                (-1.0..=1.0).sample_single(&mut rng).unwrap(),
                (-1.0..=1.0).sample_single(&mut rng).unwrap(),
            );
            let yaw = (0.0..TAU).sample_single(&mut rng).unwrap();
            let translation = spawner_transform.transform_point(local_point);
            GlobalTransform::from(
                Transform::from_translation(translation).with_rotation(Quat::from_rotation_y(yaw)),
            )
        } else {
            spawner_transform
        };
        let character = world
            .run_system_cached_with::<_, Result<_>, _, _>(
                crate::character::spawn_character,
                &crate::character::CharacterArgs {
                    type_name: type_name.clone(),
                    spawn_transform: spawn_transform.into(),
                    spawn_anchor: default(),
                    scene_entity: Some(current_scene_entity),
                    model_index: None,
                    start_as_agent: true,
                },
            )
            .unwrap()
            .unwrap();
        if let Some(ref id) = id {
            world.entity_mut(character).insert(Name::new(id.clone()));
        }
    }

    Ok(())
}

pub(crate) fn from_xml(
    attributes: Vec<xml::attribute::OwnedAttribute>,
) -> Result<SpawnCharacter, ()> {
    let mut area: Option<String> = None;
    let mut type_name: Option<String> = None;
    let mut id: Option<String> = None;
    let mut nr: u16 = 1;
    for xml::attribute::OwnedAttribute { name, value } in attributes {
        if name.local_name.eq_ignore_ascii_case("area") {
            area = Some(value);
        } else if name.local_name.eq_ignore_ascii_case("type") {
            type_name = Some(value);
        } else if name.local_name.eq_ignore_ascii_case("id") {
            id = Some(value);
        } else if name.local_name.eq_ignore_ascii_case("nr") {
            nr = value.parse().unwrap();
        } else {
            warn!(
                "Unhandled trigger spawn action attribute {:?}",
                name.local_name
            );
        }
    }
    Ok(SpawnCharacter {
        area: area.ok_or(())?,
        type_name: type_name.ok_or(())?,
        id,
        nr,
    })
}
