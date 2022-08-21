use super::{frame_info::*, test_game_physics};
use square_wheel_lib::common::{bbox::*, camera_rotation_controller::*, math_types::*};

// This file contains various ECS components.
//

// Various components for test things identification.

pub struct TestModelComponent {}
pub struct TestDecalComponent {}
pub struct TestLightComponent {}

pub struct TestSubmodelComponent
{
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

// Component for physics object, which location will be updated according to location component.
pub struct LocationKinematicPhysicsObjectComponent
{
	pub phys_handle: test_game_physics::ObjectHandle,
}

// Calculate location relative other entity.
pub struct OtherEntityLocationComponent
{
	pub entity: hecs::Entity,
	pub relative_position: Vec3f,
	pub relative_rotation: QuaternionF,
}

// Component that sets ModelEntity position/rotation using LocationComponent.
pub struct ModelEntityLocationLinkComponent {}

// Component that sets SubmodelEntityWithIndex position/rotation using LocationComponent.
pub struct SubmodelEntityWithIndexLocationLinkComponent {}

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

// Trigger than can be activated by touching.
pub struct TriggerComponent
{
	pub bbox: BBox,
}

// Component of trigger entity to trigger single target.
pub struct TrggerSingleTargetComponent
{
	pub target: hecs::Entity,
}

// Component for entities that may be activated.
pub struct EntityActivationComponent
{
	pub activated: bool,
}

pub struct PlateComponent
{
	pub speed: f32,
	pub position_lower: Vec3f,
	pub position_upper: Vec3f,
	pub state: PlateState,
}

pub enum PlateState
{
	TargetUp,
	TargetDown,
	StayTop
	{
		down_time_s: f32,
	},
}
