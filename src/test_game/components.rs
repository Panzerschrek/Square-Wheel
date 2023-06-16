use super::{frame_info::*, test_game_physics};
use serde::{Deserialize, Serialize};
use square_wheel_lib::common::{bbox::*, camera_rotation_controller::*, math_types::*};

// This file contains various ECS components.
// Do not forget to update save/load code while adding new components!
//

// Various components for test things identification.
#[derive(Serialize, Deserialize)]
pub struct TestModelComponent {}
#[derive(Serialize, Deserialize)]
pub struct TestDecalComponent {}
#[derive(Serialize, Deserialize)]
pub struct TestSpriteComponent {}
#[derive(Serialize, Deserialize)]
pub struct TestLightComponent {}
#[derive(Serialize, Deserialize)]
pub struct TestMirrorComponent {}

#[derive(Serialize, Deserialize)]
pub struct TestProjectileComponent
{
	pub velocity: Vec3f,
	// For rotation around velocity axis.
	pub angular_velocity: f32,
}

// Drawable submodel with index.
// Store index in order to fill result vector of submodels.
#[derive(Serialize, Deserialize)]
pub struct SubmodelEntityWithIndex
{
	pub index: usize,
	pub submodel_entity: SubmodelEntity,
}

// Component for entities that have some location.
#[derive(Copy, Clone, Serialize, Deserialize)]
pub struct LocationComponent
{
	pub position: Vec3f,
	pub rotation: QuaternionF,
}

// Despawn entities with such component when despawn time is reached.
#[derive(Serialize, Deserialize)]
pub struct TimedDespawnComponent
{
	pub despawn_time: f32,
}

// Take location from player controller.
#[derive(Serialize, Deserialize)]
pub struct PlayerControllerLocationComponent {}

// Location will be taken from physics engine.
pub type PhysicsLocationComponent = test_game_physics::ObjectHandle;

// Component for physics object, which location will be updated according to location component.
#[derive(Serialize, Deserialize)]
pub struct LocationKinematicPhysicsObjectComponent
{
	pub phys_handle: test_game_physics::ObjectHandle,
}

// Calculate location relative other entity.
#[derive(Serialize, Deserialize)]
pub struct OtherEntityLocationComponent
{
	pub entity: hecs::Entity,
	pub relative_position: Vec3f,
	pub relative_rotation: QuaternionF,
}

// Take location from player controller camera.
#[derive(Serialize, Deserialize)]
pub struct PlayerControllerCameraLocationComponent
{
	pub entity: hecs::Entity,
	pub camera_view_offset: Vec3f,
	pub relative_position: Vec3f,
	pub relative_rotation: QuaternionF,
}

// Component that sets ModelEntity position/rotation using LocationComponent.
#[derive(Serialize, Deserialize)]
pub struct ModelEntityLocationLinkComponent {}

// Component that sets SubmodelEntityWithIndex position/rotation using LocationComponent.
#[derive(Serialize, Deserialize)]
pub struct SubmodelEntityWithIndexLocationLinkComponent {}

// Component that sets Decal position/rotation using LocationComponent.
#[derive(Serialize, Deserialize)]
pub struct DecalLocationLinkComponent {}

// Component that sets Sprite position using LocationComponent.
#[derive(Serialize, Deserialize)]
pub struct SpriteLocationLinkComponent {}

// Component that sets DynamicLight position/rotation using LocationComponent.
#[derive(Serialize, Deserialize)]
pub struct DynamicLightLocationLinkComponent {}

// Component that sets ViewPortal position/rotation using target location.
#[derive(Serialize, Deserialize)]
pub struct ViewPortalTargetLocationLinkComponent {}

// Just play animation consisting of all model frames.
#[derive(Serialize, Deserialize)]
pub struct SimpleAnimationComponent {}

// Just play specified animation in loop.
#[derive(Serialize, Deserialize)]
pub struct SpecificAnimationComponent
{
	pub animation_index: usize,
	pub cur_animation_time: f32,
}

// General player component.
#[derive(Serialize, Deserialize)]
pub struct PlayerComponent
{
	pub view_model_entity: hecs::Entity,
	pub flashlight_entity: hecs::Entity,
}

// Component for player controlling.
#[derive(Serialize, Deserialize)]
pub struct PlayerControllerComponent
{
	pub rotation_controller: CameraRotationController,
	pub position_source: PlayerPositionSource,
}

// Variants of player controlling.
#[derive(Serialize, Deserialize)]
pub enum PlayerPositionSource
{
	Noclip(Vec3f),
	Phys(test_game_physics::ObjectHandle),
}

// Component for entity, that can activate touch triggers.
#[derive(Serialize, Deserialize)]
pub struct TouchTriggerActivatorComponent {}

// Component for entity, that can be teleported.
#[derive(Serialize, Deserialize)]
pub struct TeleportableComponent
{
	pub destination: Option<LocationComponent>,
}

// Trigger than can be activated by touching.
#[derive(Serialize, Deserialize)]
pub struct TouchTriggerComponent
{
	pub bbox: BBox,
}

#[derive(Serialize, Deserialize)]
pub struct TouchTriggerTeleportComponent
{
	pub bbox: BBox,
}

// Component of trigger entity to trigger single target.
#[derive(Serialize, Deserialize)]
pub struct TriggerSingleTargetComponent
{
	pub target: hecs::Entity,
}

// Name of triggerable object(s) for buttons, triggers.
#[derive(Serialize, Deserialize)]
pub struct TargetNameComponent
{
	pub name: String,
}

// Name of triggerable object, used by buttons, triggers.
#[derive(Serialize, Deserialize)]
pub struct NamedTargetComponent
{
	pub name: String,
}

// Used for various entites with "wait" field.
#[derive(Serialize, Deserialize)]
pub struct WaitComponent
{
	pub wait: f32,
}

// Component for entities that may be activated.
#[derive(Serialize, Deserialize)]
pub struct EntityActivationComponent
{
	pub activated: bool,
}

// Component for entities (like rockets) that explode while touching some geometry (world, other entities).
#[derive(Serialize, Deserialize)]
pub struct GeometryTouchExplodeComponent
{
	pub ignore_entity: hecs::Entity,
	pub radius: f32,
}

// Spawn a decal at explosion point.
// TODO - make this component serializable.
pub struct GeometryTouchExplodeDecalSpawnComponent
{
	pub decal: Decal,
	// If zero - lives forever, else - despawn in given amount time.
	pub lifetime_s: f32,
}

#[derive(Serialize, Deserialize)]
pub struct PlateComponent
{
	pub speed: f32,
	pub position_lower: Vec3f,
	pub position_upper: Vec3f,
	pub state: PlateState,
}

#[derive(Serialize, Deserialize)]
pub enum PlateState
{
	TargetUp,
	TargetDown,
	StayTop
	{
		down_time_s: f32,
	},
}

#[derive(Serialize, Deserialize)]
pub struct DoorComponent
{
	pub speed: f32,
	pub wait: f32,
	pub position_closed: Vec3f,
	pub position_opened: Vec3f,
	pub state: DoorState,
	pub slave_doors: Vec<hecs::Entity>,
}

#[derive(Serialize, Deserialize)]
pub enum DoorState
{
	TargetOpened,
	TargetClosed,
	StayOpened
	{
		down_time_s: f32,
	},
}

#[derive(Serialize, Deserialize)]
pub struct ButtonComponent
{
	pub speed: f32,
	pub wait: f32,
	pub position_released: Vec3f,
	pub position_pressed: Vec3f,
	pub state: ButtonState,
}

#[derive(Serialize, Deserialize)]
pub enum ButtonState
{
	TargetPressed,
	TargetReleased,
	StayPressed
	{
		down_time_s: f32,
	},
}

#[derive(Serialize, Deserialize)]
pub struct TrainComponent
{
	pub speed: f32,
	pub target_shift: Vec3f,
	pub state: TrainState,
	pub target: hecs::Entity,
}

#[derive(Serialize, Deserialize)]
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
