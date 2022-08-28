use super::{frame_info::*, test_game_physics};
use square_wheel_lib::common::{bbox::*, camera_rotation_controller::*, math_types::*};

// This file contains various ECS components.
//

// Various components for test things identification.

pub struct TestModelComponent {}
pub struct TestDecalComponent {}
pub struct TestLightComponent {}

pub struct TestProjectileComponent
{
	pub velocity: Vec3f,
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

// Despawn entities with such component when despawn time is reached.
pub struct TimedDespawnComponent
{
	pub despawn_time: f32,
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

// Take location from player controller camera.
pub struct PlayerControllerCameraLocationComponent
{
	pub entity: hecs::Entity,
	pub camera_view_offset: Vec3f,
	pub relative_position: Vec3f,
	pub relative_rotation: QuaternionF,
}

// Component that sets ModelEntity position/rotation using LocationComponent.
pub struct ModelEntityLocationLinkComponent {}

// Component that sets SubmodelEntityWithIndex position/rotation using LocationComponent.
pub struct SubmodelEntityWithIndexLocationLinkComponent {}

// Component that sets Decal position/rotation using LocationComponent.
pub struct DecalLocationLinkComponent {}

// Component that sets DynamicLight position/rotation using LocationComponent.
pub struct DynamicLightLocationLinkComponent {}

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
pub struct TouchTriggerComponent
{
	pub bbox: BBox,
}

// Component of trigger entity to trigger single target.
pub struct TriggerSingleTargetComponent
{
	pub target: hecs::Entity,
}

// Name of triggerable object(s) for buttons, triggers.
pub struct TargetNameComponent
{
	pub name: String,
}

// Name of triggerable object, used by buttons, triggers.
pub struct NamedTargetComponent
{
	pub name: String,
}

// Used for various entites with "wait" field.
pub struct WaitComponent
{
	pub wait: f32,
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

pub struct DoorComponent
{
	pub speed: f32,
	pub wait: f32,
	pub position_closed: Vec3f,
	pub position_opened: Vec3f,
	pub state: DoorState,
	pub slave_doors: Vec<hecs::Entity>,
}

pub enum DoorState
{
	TargetOpened,
	TargetClosed,
	StayOpened
	{
		down_time_s: f32,
	},
}

pub struct ButtonComponent
{
	pub speed: f32,
	pub wait: f32,
	pub position_released: Vec3f,
	pub position_pressed: Vec3f,
	pub state: ButtonState,
}

pub enum ButtonState
{
	TargetPressed,
	TargetReleased,
	StayPressed
	{
		down_time_s: f32,
	},
}

pub struct TrainComponent
{
	pub speed: f32,
	pub target_shift: Vec3f,
	pub state: TrainState,
	pub target: hecs::Entity,
}

pub enum TrainState
{
	SearchForInitialPosition,
	WaitForActivation,
	SearchForNextTarget,
	Move,
	Wait
	{
		continue_time_s: f32,
	},
}
