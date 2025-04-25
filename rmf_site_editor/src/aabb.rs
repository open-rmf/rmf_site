// The code for this module is lifted from https://github.com/bevyengine/bevy/pull/4944/files

// There is currently an issue in Bevy when it comes to entities whose mesh
// changes, either by changing the handle or by the mesh asset itself being
// modified. The issue is that the Aabb component of the entity does not get
// updated. That can lead to misbehavior in systems related to rendering and
// picking. This module fixes that with a system for automatically updating Aabb
// components. This system was left out of mainline Bevy because in some
// situations it may have an undesirable amount of overhead, but it is critical
// for the Site Editor because we frequently modify meshes.

use bevy::{
    prelude::*,
    render::{mesh::MeshAabb, primitives::Aabb, view::VisibilitySystems},
    utils::HashMap,
};
use smallvec::SmallVec;

/// Tracks which [`Entities`](Entity) have which meshes for entities whose [`Aabb`]s are managed by
/// the [`calculate_bounds`][1] and [`update_bounds`] systems. This is needed because `update_bounds`
/// recomputes `Aabb`s for entities whose mesh has been mutated. These mutations are visible via
/// [`AssetEvent<Mesh>`](AssetEvent) which tells us which mesh was changed but not which entities
/// have that mesh.
///
/// [1]: bevy::render::view::calculate_bounds
#[derive(Debug, Default, Clone, Resource)]
pub struct EntityMeshMap {
    entities_with_mesh: HashMap<AssetId<Mesh>, SmallVec<[Entity; 1]>>,
    mesh_for_entity: HashMap<Entity, AssetId<Mesh>>,
}

impl EntityMeshMap {
    /// Register the passed `entity` as having the passed `mesh_handle`.
    fn register(&mut self, entity: Entity, mesh_handle: &Handle<Mesh>) {
        // Note that this list can have duplicates if an entity is registered for a mesh multiple
        // times. This should be rare and only cause an additional `Aabb.clone()` in
        // `update_bounds` so it is preferable to a `HashSet` for now.
        self.entities_with_mesh
            .entry(mesh_handle.into())
            .or_default()
            .push(entity);
        self.mesh_for_entity.insert(entity, mesh_handle.into());
    }

    /// Deregisters the mapping between an `Entity` and `Mesh`. Used so [`update_bounds`] can
    /// track which mappings are still active so `Aabb`s are updated correctly.
    fn deregister(&mut self, entity: Entity) {
        let mut inner = || {
            let mesh = self.mesh_for_entity.remove(&entity)?;

            // This lookup failing is _probably_ an error.
            let entities = self.entities_with_mesh.get_mut(&mesh)?;

            // There could be duplicate entries in here if an entity was registered with a mesh
            // multiple times. It's important to remove all references so that if an entity gets a
            // new mesh and its old mesh is mutated, the entity doesn't get its old mesh's new
            // `Aabb`. Note that there _should_ only be one entity.
            for i in (0..entities.len()).rev() {
                if entities[i] == entity {
                    entities.swap_remove(i);
                }
            }
            Some(())
        };
        inner();
    }
}

pub fn register_bounds(
    new_aabb: Query<(Entity, &Mesh3d), Added<Aabb>>,
    mut entity_mesh_map: ResMut<EntityMeshMap>,
) {
    for (e, mesh) in &new_aabb {
        entity_mesh_map.register(e, mesh);
    }
}

/// Updates [`Aabb`]s for [`Entities`](Entity) with [`Mesh`]es. This includes `Entities` that have
/// been assigned new `Mesh`es as well as `Entities` whose `Mesh` has been directly mutated.
///
/// NOTE: This system needs to remove entities from their collection in
/// [`EntityMeshMap`] whenever a mesh handle is reassigned or an entity's mesh handle is
/// removed. This may impact performance if meshes with many entities are frequently
/// reassigned/removed.
pub fn update_bounds(
    mut commands: Commands,
    meshes: Res<Assets<Mesh>>,
    mut mesh_reassigned: Query<(Entity, &Mesh3d, &mut Aabb), Changed<Mesh3d>>,
    mut entity_mesh_map: ResMut<EntityMeshMap>,
    mut mesh_events: EventReader<AssetEvent<Mesh>>,
    mut entities_lost_mesh: RemovedComponents<Mesh3d>,
) {
    for entity in entities_lost_mesh.read() {
        entity_mesh_map.deregister(entity);
    }

    for (entity, mesh_handle, mut aabb) in mesh_reassigned.iter_mut() {
        entity_mesh_map.deregister(entity);
        if let Some(mesh) = meshes.get(&mesh_handle.0) {
            if let Some(new_aabb) = mesh.compute_aabb() {
                entity_mesh_map.register(entity, mesh_handle);
                *aabb = new_aabb;
            }
        }
    }

    let to_update = |event: &AssetEvent<Mesh>| {
        let id = match event {
            AssetEvent::Modified { id } => id,
            _ => return None,
        };
        let mesh = meshes.get(*id)?;
        let entities_with_handle = entity_mesh_map.entities_with_mesh.get(id)?;
        let aabb = mesh.compute_aabb()?;
        Some((aabb, entities_with_handle))
    };
    for (aabb, entities_with_handle) in mesh_events.read().filter_map(to_update) {
        for entity in entities_with_handle {
            commands.entity(*entity).insert(aabb.clone());
        }
    }
}

pub struct AabbUpdatePlugin;

impl Plugin for AabbUpdatePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EntityMeshMap>()
            .add_systems(
                PostUpdate,
                register_bounds.after(VisibilitySystems::CalculateBounds),
            )
            .add_systems(PostUpdate, update_bounds);
    }
}
