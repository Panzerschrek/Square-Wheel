pub type Vec2f = cgmath::Vector2<f32>;
pub type Vec3f = cgmath::Vector3<f32>;
pub type Vec4f = cgmath::Vector4<f32>;

pub type Vec2d = cgmath::Vector2<f64>;
pub type Vec3d = cgmath::Vector3<f64>;
pub type Vec4d = cgmath::Vector4<f64>;

pub type Mat3f = cgmath::Matrix3<f32>;
pub type Mat3d = cgmath::Matrix3<f64>;

pub type Mat4f = cgmath::Matrix4<f32>;
pub type Mat4d = cgmath::Matrix4<f64>;

pub type QuaternionF = cgmath::Quaternion<f32>;

pub type RadiansF = cgmath::Rad<f32>;
pub type DegreesF = cgmath::Rad<f32>;

pub type RadiansD = cgmath::Rad<f64>;
pub type DegreesD = cgmath::Rad<f64>;

pub type EulerAnglesF = cgmath::Euler<RadiansF>;

pub use cgmath::{Angle, ElementWise, InnerSpace, Matrix, Rad, Rotation, Rotation3, SquareMatrix, Zero};
