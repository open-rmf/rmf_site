use bevy_asset::Handle;
use bevy_ecs::prelude::*;
use bevy_pbr::prelude::*;
use bevy_render::prelude::*;

#[derive(Resource)]
pub struct CursorHaloMesh(pub Handle<Mesh>);

#[derive(Resource)]
pub struct CursorHaloMaterial(pub Handle<StandardMaterial>);

#[derive(Resource)]
pub struct CursorDaggerMesh(pub Handle<Mesh>);

#[derive(Resource)]
pub struct CursorDaggerMaterial(pub Handle<StandardMaterial>);