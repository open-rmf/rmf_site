/*
 * Copyright (C) 2026 Open Source Robotics Foundation
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 *
*/

use crate::{
    interaction::{CategoryVisibility, CursorHoverVisualization, OutlineVisualization},
    layers::ZLayer,
    site::*,
};
use bevy::prelude::*;
use geo::{
    line_intersection::{line_intersection, LineIntersection},
    Area, CoordsIter, LineString, Polygon, TriangulateSpade,
};
use rmf_site_mesh::{line_stroke_mesh, make_closed_path_outline, MeshBuffer};
use rmf_site_picking::{Hovered, Hovering, Selectable, Selected, Selection};
use std::collections::HashSet;

#[derive(Component, Clone, Copy, Debug, Default)]
pub(crate) struct ZoneMarker;

#[derive(Component)]
pub(crate) struct ZoneSetMarker;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ZoneSetFilter {
    #[default]
    All,
    Unassigned,
    Set(Entity),
}

#[derive(Default, Resource)]
pub(crate) struct ZoneFilter {
    pub(crate) site: Option<Entity>,
    pub(crate) selection: ZoneSetFilter,
}

impl ZoneSetFilter {
    fn matches(self, sets: &ZoneSets<Entity>) -> bool {
        match self {
            Self::All => true,
            Self::Unassigned => sets.0.is_empty(),
            Self::Set(set) => sets.0.contains(&set),
        }
    }
}

pub(super) fn update_zone_sets(
    mut deleted: RemovedComponents<ZoneSetMarker>,
    mut zones: Query<&mut ZoneSets<Entity>>,
    mut filter: ResMut<ZoneFilter>,
    workspace: Res<crate::CurrentWorkspace>,
) {
    if filter.site != workspace.root {
        filter.site = workspace.root;
        filter.selection = ZoneSetFilter::All;
    }
    for set in deleted.read() {
        for mut sets in &mut zones {
            if sets.0.contains(&set) {
                sets.0.remove(&set);
            }
        }
        if filter.selection == ZoneSetFilter::Set(set) {
            filter.selection = ZoneSetFilter::All;
        }
    }
}

#[derive(Bundle)]
pub(crate) struct ZoneBundle {
    anchors: Path<Entity>,
    name: NameInSite,
    sets: ZoneSets<Entity>,
    marker: ZoneMarker,
}

impl From<Zone<Entity>> for ZoneBundle {
    fn from(zone: Zone<Entity>) -> Self {
        Self {
            anchors: zone.anchors,
            name: zone.name,
            sets: zone.sets,
            marker: ZoneMarker,
        }
    }
}

impl From<Path<Entity>> for ZoneBundle {
    fn from(anchors: Path<Entity>) -> Self {
        Self::from(Zone::from(anchors))
    }
}

#[derive(Component)]
pub(super) struct ZoneMesh(Handle<Mesh>);

fn zone_mesh(entity: Entity, path: &Path<Entity>, anchors: &AnchorParams) -> Mesh {
    let positions: Result<Vec<[f32; 3]>, _> = path
        .0
        .iter()
        .map(|anchor| {
            anchors
                .point_in_parent_frame_of(*anchor, Category::General, entity)
                .map(|p| [p.x, p.y, 0.0])
        })
        .collect();
    match positions {
        Ok(positions) => make_zone_mesh(&positions),
        Err(error) => {
            warn!("Unable to render zone {entity:?}: {error}");
            // Skipping a missing anchor would change the polygon.
            make_zone_mesh(&[])
        }
    }
}

/// Reject degenerate and self-intersecting boundaries before triangulation.
fn valid_polygon(positions: &[[f32; 3]]) -> Option<Polygon<f64>> {
    if positions.len() < 3 || !positions.iter().flatten().all(|v| v.is_finite()) {
        return None;
    }
    let polygon = Polygon::new(
        LineString::from(
            positions
                .iter()
                .map(|p| [f64::from(p[0]), f64::from(p[1])])
                .collect::<Vec<_>>(),
        ),
        vec![],
    );
    if polygon.unsigned_area() == 0.0 {
        return None;
    }
    let edges: Vec<_> = polygon.exterior().lines().collect();
    for (i, a) in edges.iter().enumerate() {
        if a.start == a.end {
            return None;
        }
        for (j, b) in edges.iter().enumerate().skip(i + 1) {
            let adjacent = j == i + 1 || (i == 0 && j == edges.len() - 1);
            match line_intersection(*a, *b) {
                None => {}
                Some(LineIntersection::SinglePoint {
                    is_proper: false, ..
                }) if adjacent => {}
                _ => return None,
            }
        }
    }
    Some(polygon)
}

fn make_zone_mesh(positions: &[[f32; 3]]) -> Mesh {
    // Preview vertices can overlap; zero-length edges break outline normals.
    let mut positions = if positions.iter().flatten().all(|v| v.is_finite()) {
        positions.to_vec()
    } else {
        Vec::new()
    };
    positions.dedup();
    if positions.len() > 1 && positions.first() == positions.last() {
        positions.pop();
    }
    let mut vertices = Vec::new();
    if let Some(polygon) = valid_polygon(&positions) {
        if let Ok(triangles) = polygon.constrained_triangulation(Default::default()) {
            for triangle in triangles {
                vertices.extend(
                    triangle
                        .coords_iter()
                        .map(|p| [p.x as f32, p.y as f32, 0.0]),
                );
            }
        }
    }
    let normals = vec![[0.0, 0.0, 1.0]; vertices.len()];
    let indices = (0..vertices.len() as u32).collect();
    let mut buffer = MeshBuffer::new(vertices, normals, indices);
    if positions.len() >= 2 {
        // Edge strips keep unfilled boundaries visible and pickable.
        let edge_count = if positions.len() == 2 {
            1
        } else {
            positions.len()
        };
        for i in 0..edge_count {
            buffer = buffer.merge_with(line_stroke_mesh(
                positions[i].into(),
                positions[(i + 1) % positions.len()].into(),
                0.03,
            ));
        }
        buffer
            .merge_with(make_closed_path_outline(positions))
            .into()
    } else {
        buffer.into()
    }
}

pub(super) fn add_zone_visuals(
    mut commands: Commands,
    mut zones: Query<
        (
            Entity,
            &Path<Entity>,
            &mut ZoneSets<Entity>,
            Option<&SiteID>,
        ),
        Added<ZoneMarker>,
    >,
    anchors: AnchorParams,
    mut dependents: Query<&mut Dependents, With<Anchor>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    visibility: Res<CategoryVisibility<ZoneMarker>>,
    filter: Res<ZoneFilter>,
) {
    for (entity, path, mut sets, id) in &mut zones {
        if id.is_none() {
            if let ZoneSetFilter::Set(set) = filter.selection {
                sets.0.insert(set);
            }
        }
        let mesh = meshes.add(zone_mesh(entity, path, &anchors));
        let visual = commands
            .spawn((
                Mesh3d(mesh.clone()),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgba(1.0, 0.55, 0.1, 0.35),
                    alpha_mode: AlphaMode::Blend,
                    unlit: true,
                    cull_mode: None,
                    ..default()
                })),
                Transform::default(),
                Visibility::Inherited,
                Selectable::new(entity),
            ))
            .id();
        commands
            .entity(entity)
            .insert((
                Transform::from_xyz(0.0, 0.0, ZLayer::Zone.to_z()),
                if visibility.0 && filter.selection.matches(&sets) {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                },
                ZoneMesh(mesh),
                Category::Custom("Zone".into()),
                PathBehavior::for_floor(),
                OutlineVisualization::default(),
                CursorHoverVisualization,
                Selectable::new(entity),
            ))
            .add_child(visual);
        for anchor in &path.0 {
            if let Ok(mut deps) = dependents.get_mut(*anchor) {
                deps.insert(entity);
            }
        }
    }
}

pub(super) fn update_zone_visuals(
    zones: Query<(Entity, &ZoneMesh, &Path<Entity>), With<ZoneMarker>>,
    changed_paths: Query<Entity, (With<ZoneMarker>, Changed<Path<Entity>>)>,
    changed_anchors: Query<
        &Dependents,
        (
            With<Anchor>,
            Or<(Changed<Anchor>, Changed<GlobalTransform>)>,
        ),
    >,
    anchors: AnchorParams,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let mut changed: HashSet<Entity> = changed_paths.iter().collect();
    for dependents in &changed_anchors {
        changed.extend(dependents.iter().copied());
    }
    for entity in changed {
        let Ok((entity, handle, path)) = zones.get(entity) else {
            continue;
        };
        if let Some(mesh) = meshes.get_mut(&handle.0) {
            *mesh = zone_mesh(entity, path, &anchors);
        }
    }
}

/// Clear selection and hover cues when zones are hidden.
pub(crate) fn clear_hidden_zone_selection(
    mut zones: Query<
        (
            Entity,
            &mut Visibility,
            &ZoneSets<Entity>,
            Option<&Pending>,
            Option<&mut Selected>,
            Option<&mut Hovered>,
        ),
        With<ZoneMarker>,
    >,
    category: Res<CategoryVisibility<ZoneMarker>>,
    filter: Res<ZoneFilter>,
    mut selection: ResMut<Selection>,
    mut hovering: ResMut<Hovering>,
) {
    for (entity, mut visibility, sets, pending, selected, hovered) in &mut zones {
        let visible = category.0 && (pending.is_some() || filter.selection.matches(sets));
        let next = if visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *visibility != next {
            *visibility = next;
        }
        if *visibility != Visibility::Hidden {
            continue;
        }
        selection.selected.remove(&entity);
        if hovering.0 == Some(entity) {
            hovering.0 = None;
        }
        if let Some(mut selected) = selected {
            if selected.cue() {
                *selected = Selected::default();
            }
        }
        if let Some(mut hovered) = hovered {
            if hovered.cue() {
                *hovered = Hovered::default();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interaction::{CategoryVisibilityPlugin, InteractionState, SetCategoryVisibility};
    use bevy::{
        ecs::system::RunSystemOnce, render::mesh::VertexAttributeValues, state::app::StatesPlugin,
    };

    #[test]
    fn zone_set_filter_preserves_membership_and_global_visibility() {
        let mut app = visual_test_app();
        let first = app.world_mut().spawn(ZoneSetMarker).id();
        let second = app.world_mut().spawn(ZoneSetMarker).id();
        let (zone, _) = spawn_zone(app.world_mut(), 0.0);
        app.world_mut()
            .get_mut::<ZoneSets<Entity>>(zone)
            .unwrap()
            .0
            .extend([first, second]);
        let (unassigned, _) = spawn_zone(app.world_mut(), 0.0);
        app.update();
        app.world_mut().resource_mut::<ZoneFilter>().selection = ZoneSetFilter::Set(first);
        app.world_mut()
            .run_system_once(clear_hidden_zone_selection)
            .unwrap();
        assert_eq!(
            *app.world().get::<Visibility>(zone).unwrap(),
            Visibility::Inherited
        );
        assert_eq!(
            *app.world().get::<Visibility>(unassigned).unwrap(),
            Visibility::Hidden
        );
        app.world_mut().resource_mut::<ZoneFilter>().selection = ZoneSetFilter::Set(second);
        app.world_mut()
            .run_system_once(clear_hidden_zone_selection)
            .unwrap();
        assert_eq!(
            *app.world().get::<Visibility>(zone).unwrap(),
            Visibility::Inherited
        );
        app.world_mut()
            .resource_mut::<Selection>()
            .selected
            .insert(zone);
        app.world_mut().entity_mut(zone).insert(Selected {
            is_selected: true,
            ..default()
        });
        app.world_mut().resource_mut::<ZoneFilter>().selection = ZoneSetFilter::Unassigned;
        app.world_mut()
            .run_system_once(clear_hidden_zone_selection)
            .unwrap();
        assert_eq!(
            *app.world().get::<Visibility>(zone).unwrap(),
            Visibility::Hidden
        );
        assert_eq!(
            *app.world().get::<Visibility>(unassigned).unwrap(),
            Visibility::Inherited
        );
        assert!(app.world().resource::<Selection>().selected.is_empty());
        assert!(!app.world().get::<Selected>(zone).unwrap().cue());
        app.world_mut().entity_mut(zone).insert(Pending);
        app.world_mut()
            .run_system_once(clear_hidden_zone_selection)
            .unwrap();
        assert_eq!(
            *app.world().get::<Visibility>(zone).unwrap(),
            Visibility::Inherited
        );
        app.world_mut()
            .resource_mut::<CategoryVisibility<ZoneMarker>>()
            .0 = false;
        app.world_mut()
            .run_system_once(clear_hidden_zone_selection)
            .unwrap();
        assert_eq!(
            *app.world().get::<Visibility>(zone).unwrap(),
            Visibility::Hidden
        );
        assert_eq!(
            *app.world().get::<Visibility>(unassigned).unwrap(),
            Visibility::Hidden
        );
        assert_eq!(
            app.world().get::<ZoneSets<Entity>>(zone).unwrap().0,
            std::collections::BTreeSet::from([first, second])
        );
    }

    #[test]
    fn deleting_a_zone_set_preserves_zones_and_other_memberships() {
        let mut app = visual_test_app();
        app.init_resource::<crate::CurrentWorkspace>()
            .add_systems(Update, update_zone_sets);
        let first = app.world_mut().spawn(ZoneSetMarker).id();
        let second = app.world_mut().spawn(ZoneSetMarker).id();
        let (zone, _) = spawn_zone(app.world_mut(), 0.0);
        app.world_mut()
            .get_mut::<ZoneSets<Entity>>(zone)
            .unwrap()
            .0
            .extend([first, second]);
        app.update();
        app.world_mut().resource_mut::<ZoneFilter>().selection = ZoneSetFilter::Set(first);
        app.world_mut().despawn(first);
        app.update();
        assert_eq!(
            app.world().get::<ZoneSets<Entity>>(zone).unwrap().0,
            std::collections::BTreeSet::from([second])
        );
        assert_eq!(
            app.world().resource::<ZoneFilter>().selection,
            ZoneSetFilter::All
        );
    }

    #[test]
    fn drawing_in_a_filtered_set_assigns_new_zones_but_preserves_loaded_membership() {
        let mut app = visual_test_app();
        let set = app.world_mut().spawn(ZoneSetMarker).id();
        app.world_mut().resource_mut::<ZoneFilter>().selection = ZoneSetFilter::Set(set);
        let (new_zone, _) = spawn_zone(app.world_mut(), 0.0);
        let (loaded_zone, _) = spawn_zone(app.world_mut(), 0.0);
        app.world_mut().entity_mut(loaded_zone).insert(SiteID(50));
        app.update();
        assert!(app
            .world()
            .get::<ZoneSets<Entity>>(new_zone)
            .unwrap()
            .0
            .contains(&set));
        assert!(app
            .world()
            .get::<ZoneSets<Entity>>(loaded_zone)
            .unwrap()
            .0
            .is_empty());
        assert_eq!(
            *app.world().get::<Visibility>(loaded_zone).unwrap(),
            Visibility::Hidden
        );
    }

    #[test]
    fn zone_geometry_accepts_concavity_but_rejects_invalid_boundaries() {
        let concave = [
            [0.0, 0.0, 0.0],
            [4.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 4.0, 0.0],
        ];
        assert!(valid_polygon(&concave).is_some());
        let crossing = [
            [0.0, 0.0, 0.0],
            [4.0, 3.0, 0.0],
            [0.0, 3.0, 0.0],
            [2.0, 0.0, 0.0],
        ];
        // An area check alone misses crossings with nonzero signed area.
        for invalid in [
            crossing.to_vec(),
            vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0]],
            vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 0.0, 0.0]],
            vec![[0.0, 0.0, 0.0], [f32::NAN, 0.0, 0.0], [0.0, 1.0, 0.0]],
        ] {
            assert!(valid_polygon(&invalid).is_none());
        }
        // Invalid boundaries remain editable as outlines.
        let mesh = make_zone_mesh(&crossing);
        let Some(VertexAttributeValues::Float32x3(vertices)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("missing positions");
        };
        assert!(
            vertices.iter().any(|p| !crossing.contains(p)),
            "invalid paths need nonzero-width edges for picking"
        );
    }

    fn visual_test_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin))
            .insert_state(InteractionState::Enable)
            .add_plugins((
                CategoryVisibilityPlugin::<ZoneMarker>::visible(true),
                CategoryVisibilityPlugin::<FloorMarker>::visible(true),
            ))
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<ZoneFilter>()
            .init_resource::<Selection>()
            .init_resource::<Hovering>()
            .add_systems(
                PostUpdate,
                (add_zone_visuals, ApplyDeferred, update_zone_visuals).chain(),
            );
        app
    }

    fn spawn_zone(world: &mut World, elevation: f32) -> (Entity, Vec<Entity>) {
        let level_tf = GlobalTransform::from_translation(Vec3::new(0.0, 0.0, elevation));
        let level = world
            .spawn((level_tf, Transform::from_xyz(0.0, 0.0, elevation)))
            .id();
        let anchors = [[0.0, 0.0], [4.0, 0.0], [0.0, 4.0]]
            .into_iter()
            .map(|p| {
                world
                    .spawn((
                        AnchorBundle::new(Anchor::from(p)).parent_transform(&level_tf),
                        ChildOf(level),
                    ))
                    .id()
            })
            .collect::<Vec<_>>();
        let zone = world
            .spawn((ZoneBundle::from(Path(anchors.clone())), ChildOf(level)))
            .id();
        (zone, anchors)
    }

    fn mesh_positions(world: &World, zone: Entity) -> Vec<[f32; 3]> {
        let visual = world.get::<Children>(zone).unwrap()[0];
        let handle = &world.get::<Mesh3d>(visual).unwrap().0;
        let mesh = world.resource::<Assets<Mesh>>().get(handle).unwrap();
        match mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap() {
            VertexAttributeValues::Float32x3(values) => values.clone(),
            _ => panic!("unexpected position format"),
        }
    }

    #[test]
    fn zone_tracks_elevated_anchors_and_missing_references() {
        let mut app = visual_test_app();
        let (zone, anchors) = spawn_zone(app.world_mut(), 7.5);
        app.update();
        let before = mesh_positions(app.world(), zone);
        assert!(!before.is_empty());
        assert!(before.iter().all(|p| p[2] == 0.0));
        assert_eq!(
            app.world().get::<Transform>(zone).unwrap().translation.z,
            ZLayer::Zone.to_z()
        );
        assert!(app
            .world()
            .get::<Dependents>(anchors[0])
            .unwrap()
            .contains(&zone));
        assert!(app.world().get::<FloorMarker>(zone).is_none());
        assert!(app.world().get::<TextureNeedsAssignment>(zone).is_none());
        app.world_mut().entity_mut(anchors[1]).insert((
            Anchor::from([6.0, 0.0]),
            GlobalTransform::from_translation(Vec3::new(6.0, 0.0, 7.5)),
        ));
        app.update();
        assert_ne!(mesh_positions(app.world(), zone), before);
        // Hide the mesh if an anchor cannot be resolved.
        let missing = app.world_mut().spawn_empty().id();
        app.world_mut()
            .get_mut::<Path<Entity>>(zone)
            .unwrap()
            .0
            .push(missing);
        app.update();
        assert!(mesh_positions(app.world(), zone).is_empty());
    }

    #[test]
    fn zone_visibility_is_independent_and_clears_selection() {
        let mut app = visual_test_app();
        let (zone, _) = spawn_zone(app.world_mut(), 0.0);
        app.update();
        let original = app.world().get::<Path<Entity>>(zone).unwrap().clone();
        let floor = app
            .world_mut()
            .spawn((FloorMarker, Visibility::Inherited))
            .id();
        app.world_mut().entity_mut(zone).insert((
            Selected {
                is_selected: true,
                ..default()
            },
            Hovered {
                is_hovered: true,
                ..default()
            },
        ));
        app.world_mut()
            .resource_mut::<Selection>()
            .selected
            .insert(zone);
        app.world_mut().resource_mut::<Hovering>().0 = Some(zone);
        app.world_mut()
            .send_event(SetCategoryVisibility::<ZoneMarker>::from(false));
        app.update();
        app.world_mut()
            .run_system_once(clear_hidden_zone_selection)
            .unwrap();
        assert_eq!(
            *app.world().get::<Visibility>(zone).unwrap(),
            Visibility::Hidden
        );
        assert_eq!(
            *app.world().get::<Visibility>(floor).unwrap(),
            Visibility::Inherited
        );
        assert_eq!(*app.world().get::<Path<Entity>>(zone).unwrap(), original);
        assert!(!app.world().get::<Selected>(zone).unwrap().cue());
        assert!(!app.world().get::<Hovered>(zone).unwrap().cue());
        assert!(app.world().resource::<Selection>().selected.is_empty());
        assert!(app.world().resource::<Hovering>().0.is_none());

        let (new_zone, _) = spawn_zone(app.world_mut(), 4.0);
        app.update();
        assert_eq!(
            *app.world().get::<Visibility>(new_zone).unwrap(),
            Visibility::Hidden
        );
        app.world_mut()
            .send_event(SetCategoryVisibility::<ZoneMarker>::from(true));
        app.update();
        assert_eq!(
            *app.world().get::<Visibility>(zone).unwrap(),
            Visibility::Inherited
        );
        assert_eq!(
            *app.world().get::<Visibility>(new_zone).unwrap(),
            Visibility::Inherited
        );
    }

    #[test]
    fn zone_deletion_preserves_shared_anchors() {
        let mut app = visual_test_app();
        app.init_resource::<CurrentLevel>()
            .init_resource::<Trashcan>()
            .add_event::<rmf_site_picking::Select>()
            .add_event::<crate::log::Log>()
            .add_plugins(DeletionPlugin);
        let (zone, anchors) = spawn_zone(app.world_mut(), 0.0);
        let other = app
            .world_mut()
            .spawn(ZoneBundle::from(Path(anchors.clone())))
            .id();
        app.update();
        app.world_mut().send_event(Delete::new(zone));
        app.update();
        app.world_mut().run_system_once(clear_trashcan).unwrap();
        assert!(app.world().get_entity(zone).is_err());
        for anchor in anchors {
            let deps = app.world().get::<Dependents>(anchor).unwrap();
            assert!(!deps.contains(&zone));
            assert!(deps.contains(&other));
        }
    }

    #[test]
    fn zone_is_excluded_from_simulation_meshes() {
        let mut app = visual_test_app();
        app.init_resource::<Assets<Image>>();
        let (zone, _) = spawn_zone(app.world_mut(), 0.0);
        let level = app.world().get::<ChildOf>(zone).unwrap().parent();
        let site = app.world_mut().spawn_empty().id();
        app.world_mut().entity_mut(level).insert((
            NameInSite("Annotation level".into()),
            LevelElevation(0.0),
            SiteID(1),
            ChildOf(site),
        ));
        app.update();
        assert!(!mesh_positions(app.world(), zone).is_empty());
        let folder = testdir::testdir!();
        collect_site_meshes(app.world_mut(), site, &folder).unwrap();
        for kind in ["collision", "visual"] {
            let bytes = std::fs::read(folder.join(format!("level_1_{kind}.glb"))).unwrap();
            // The JSON payload follows the 12-byte GLB header and 8-byte chunk header.
            let size = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
            let json: serde_json::Value = serde_json::from_slice(&bytes[20..20 + size]).unwrap();
            assert!(json
                .get("meshes")
                .and_then(|v| v.as_array())
                .map_or(true, Vec::is_empty));
        }
        std::fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn zone_preview_handles_repeated_vertices() {
        for positions in [
            vec![],
            vec![[0.0, 0.0, 0.0]],
            vec![[0.0, 0.0, 0.0], [0.0, 0.0, 0.0]],
            vec![[0.0, 0.0, 0.0], [2.0, 0.0, 0.0]],
            vec![
                [0.0, 0.0, 0.0],
                [2.0, 0.0, 0.0],
                [0.0, 2.0, 0.0],
                [0.0, 0.0, 0.0],
            ],
        ] {
            let mesh = make_zone_mesh(&positions);
            for attribute in [Mesh::ATTRIBUTE_POSITION, Mesh::ATTRIBUTE_NORMAL] {
                let Some(VertexAttributeValues::Float32x3(values)) = mesh.attribute(attribute)
                else {
                    panic!("missing mesh attribute");
                };
                assert!(values.iter().flatten().all(|v| v.is_finite()));
            }
        }
    }
}
