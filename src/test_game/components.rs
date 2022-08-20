use super::{frame_info::*, test_game_physics};
use square_wheel_lib::common::{camera_rotation_controller::*, math_types::*};

// This file contains various ECS components.
//

// Various components for test things identification.

pub struct TestModelComponent {}
pub struct TestDecalComponent {}
pub struct TestLightComponent {}

pub struct TestSubmodelComponent
{
	pub phys_handle: test_game_physics::ObjectHandle,
	pub index: usize,
}

// Drawable submodel with index.
// Store index in order to fill result vector of submodels.
pub struct SubmodelEntityWithIndex
{
	pub index: usize,
	pub submodel_entity: SubmodelEntity,
}

// Component for entities that have some location.
pub struct LocationComponent
{
	pub position: Vec3f,
	pub rotation: QuaternionF,
}

// Take location from player controller.
pub struct PlayerControllerLocationComponent {}

// Location will be taken from physics engine.
pub type PhysicsLocationComponent = test_game_physics::ObjectHandle;

// Calculate location relative other entity.
pub struct OtherEntityLocationComponent
{
	pub entity: hecs::Entity,
	pub relative_position: Vec3f,
	pub relative_rotation: QuaternionF,
}

// Component that sets ModelEntity position/rotation using LocationComponent.
pub struct ModelEntityLocationLinkComponent {}

// Just play animation consisting of all model frames.
pub struct SimpleAnimationComponent {}

// General pplayer component.
pub struct PlayerComponent
{
	pub view_model_entity: hecs::Entity,
}

// Component for player controlling.
pub struct PlayerControllerComponent
{
	pub rotation_controller: CameraRotationController,
	pub position_source: PlayerPositionSource,
}

// Variants of player controlling.
pub enum PlayerPositionSource
{
	Noclip(Vec3f),
	Phys(test_game_physics::ObjectHandle),
}