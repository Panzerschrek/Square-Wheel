use super::{triangle_model::*, triangle_model_loading::*};
use crate::common::{bbox::*, math_types::*};
use std::io::{Read, Seek};

pub fn load_model_iqm(file_path: &std::path::Path) -> Result<Option<TriangleModel>, std::io::Error>
{
	let mut file = std::fs::OpenOptions::new()
		.read(true)
		.write(false)
		.create(false)
		.open(file_path)?;
	load_model_iqm_from_reader(&mut file)
}

pub fn load_model_iqm_from_reader<F: Read + Seek>(reader: &mut F) -> Result<Option<TriangleModel>, std::io::Error>
{
	let header_size = std::mem::size_of::<IQMHeader>();
	let mut header = unsafe { std::mem::zeroed::<IQMHeader>() };
	let header_bytes =
		unsafe { std::slice::from_raw_parts_mut((&mut header) as *mut IQMHeader as *mut u8, header_size) };

	if reader.read(header_bytes)? != header_size
	{
		println!("Can't read IQM header");
		return Ok(None);
	}

	if header.magic != IQM_MAGIC
	{
		println!("File is not a valid IQM model");
		return Ok(None);
	}
	if header.version != IQM_VERSION
	{
		println!(
			"Can't load incompatible IQM model version: {}, expected {}",
			header.version, IQM_VERSION
		);
		return Ok(None);
	}

	if header.num_meshes == 0
	{
		println!("Invalid IQM model with no meshes");
		return Ok(None);
	}
	if header.num_joints != header.num_poses
	{
		println!(
			"Invalid IQM model: joints/poses mismatch ({}/{})",
			header.num_joints, header.num_poses
		);
		return Ok(None);
	}

	let texts = load_texts(reader, &header)?;
	let meshes = load_meshes(reader, &header)?;
	let triangles = load_triangles(reader, &header)?;
	let vertices = load_vertices(reader, &header)?;
	let bounds = load_bounds(reader, &header)?;
	let joints = load_joints(reader, &header)?;
	let poses = load_poses(reader, &header)?;
	let animations = load_animations(reader, &header)?;

	let bones = joints
		.iter()
		.map(|j| TriangleModelBoneInfo {
			name: get_text_str(&texts, j.name).to_string(),
			parent: j.parent,
		})
		.collect();

	let frame_bones = create_frames(reader, &header, &joints, &poses)?;

	let mut frames_info: Vec<_> = bounds
		.iter()
		.map(|b| TriangleModelFrameInfo {
			bbox: BBox::from_min_max(Vec3f::from(b.bbmins), Vec3f::from(b.bbmaxs)),
		})
		.collect();

	// Create at least one frame for non-animated models.
	if frames_info.is_empty()
	{
		let mut bbox = BBox::from_point(&vertices[0].position);
		for v in &vertices
		{
			bbox.extend_with_point(&v.position);
		}
		frames_info = vec![TriangleModelFrameInfo { bbox }];
	}

	let triangles_transformed = triangles
		.iter()
		.map(|t| {
			[
				t.vertex[0] as VertexIndex,
				t.vertex[1] as VertexIndex,
				t.vertex[2] as VertexIndex,
			]
		})
		.collect();

	let animations_transfomed = animations
		.iter()
		.map(|a| TriangleModelAnimation {
			name: get_text_str(&texts, a.name).to_string(),
			start_frame: a.first_frame,
			num_frames: a.num_frames,
			frames_per_second: a.framerate,
			looped: (a.flags & IQM_LOOP) != 0,
		})
		.collect();

	// TODO - export all meshes separately.
	// Now just load single mesh.
	let single_mesh = TriangleModelMesh {
		name: get_text_str(&texts, meshes[0].name).to_string(),
		material_name: get_text_str(&texts, meshes[0].material).to_string(),
		triangles: triangles_transformed,
		vertex_data: VertexData::SkeletonAnimated(vertices),
		num_frames: header.num_frames,
	};

	// Add extra shift because we use texture coordinates floor, instead of linear OpenGL interpolation as Quake III does.
	let tc_shift = -Vec2f::new(0.5, 0.5);

	Ok(Some(TriangleModel {
		animations: animations_transfomed,
		frames_info,
		tc_shift,
		bones,
		frame_bones,
		meshes: vec![single_mesh],
	}))
}

fn load_texts<F: Read + Seek>(reader: &mut F, header: &IQMHeader) -> Result<Vec<u8>, std::io::Error>
{
	read_vector(reader, header.ofs_text as u64, header.num_text)
}

fn get_text_str(texts: &[u8], offset: u32) -> &str
{
	let start = offset as usize;
	let mut end = start;
	while end < texts.len() && texts[end] != 0
	{
		end += 1;
	}
	std::str::from_utf8(&texts[start .. end]).unwrap_or("")
}

fn load_meshes<F: Read + Seek>(reader: &mut F, header: &IQMHeader) -> Result<Vec<IQMMesh>, std::io::Error>
{
	read_vector(reader, header.ofs_meshes as u64, header.num_meshes)
}

fn load_triangles<F: Read + Seek>(reader: &mut F, header: &IQMHeader) -> Result<Vec<IQMTriangle>, std::io::Error>
{
	read_vector(reader, header.ofs_triangles as u64, header.num_triangles)
}

fn load_vertices<F: Read + Seek>(
	reader: &mut F,
	header: &IQMHeader,
) -> Result<Vec<SkeletonAnimatedVertex>, std::io::Error>
{
	let vertex_arrays = load_vertex_arrays(reader, header)?;

	let mut vertices = vec![
		SkeletonAnimatedVertex {
			tex_coord: [0.0, 0.0],
			position: Vec3f::zero(),
			normal: Vec3f::zero(),
			bones_description: [VertexBoneDescription {
				bone_index: 0,
				weight: 0
			}; 4]
		};
		header.num_vertexes as usize
	];

	// TODO - improve this, support other attributes and types.

	for vertex_array in &vertex_arrays
	{
		match vertex_array.type_
		{
			IQM_POSITION =>
			{
				if vertex_array.size == 3 && vertex_array.format == IQM_FLOAT
				{
					// TODO - use uninitialized memory.
					let mut positions = vec![Vec3f::zero(); vertices.len()];
					read_chunk(reader, vertex_array.offset as u64, &mut positions)?;
					for (v, p) in vertices.iter_mut().zip(positions.iter())
					{
						v.position = *p;
					}
				}
			},
			IQM_TEXCOORD =>
			{
				if vertex_array.size == 2 && vertex_array.format == IQM_FLOAT
				{
					// TODO - use uninitialized memory.
					let mut tex_coords = vec![[0.0, 0.0]; vertices.len()];
					read_chunk(reader, vertex_array.offset as u64, &mut tex_coords)?;
					for (v, tc) in vertices.iter_mut().zip(tex_coords.iter())
					{
						v.tex_coord = *tc;
					}
				}
			},
			IQM_NORMAL =>
			{
				if vertex_array.size == 3 && vertex_array.format == IQM_FLOAT
				{
					// TODO - use uninitialized memory.
					let mut normals = vec![Vec3f::zero(); vertices.len()];
					read_chunk(reader, vertex_array.offset as u64, &mut normals)?;
					for (v, n) in vertices.iter_mut().zip(normals.iter())
					{
						v.normal = *n;
					}
				}
			},
			IQM_TANGENT =>
			{},
			IQM_BLENDINDEXES =>
			{
				if vertex_array.size == 4 && vertex_array.format == IQM_UBYTE
				{
					let zero_indexes: [u8; 4] = [0, 0, 0, 0];
					// TODO - use uninitialized memory.
					let mut indexes = vec![zero_indexes; vertices.len()];
					read_chunk(reader, vertex_array.offset as u64, &mut indexes)?;
					for (v, index) in vertices.iter_mut().zip(indexes.iter())
					{
						for i in 0 .. 4
						{
							v.bones_description[i].bone_index = index[i];
						}
					}
				}
			},
			IQM_BLENDWEIGHTS =>
			{
				if vertex_array.size == 4 && vertex_array.format == IQM_UBYTE
				{
					let zero_weights: [u8; 4] = [0, 0, 0, 0];
					// TODO - use uninitialized memory.
					let mut weights = vec![zero_weights; vertices.len()];
					read_chunk(reader, vertex_array.offset as u64, &mut weights)?;
					for (v, weight) in vertices.iter_mut().zip(weights.iter())
					{
						for i in 0 .. 4
						{
							v.bones_description[i].weight = weight[i];
						}
					}
				}
			},
			_ =>
			{},
		}
	}

	Ok(vertices)
}

fn load_vertex_arrays<F: Read + Seek>(reader: &mut F, header: &IQMHeader)
	-> Result<Vec<IQMVertexArray>, std::io::Error>
{
	read_vector(reader, header.ofs_vertexarrays as u64, header.num_vertexarrays)
}

fn load_bounds<F: Read + Seek>(reader: &mut F, header: &IQMHeader) -> Result<Vec<IQMBounds>, std::io::Error>
{
	read_vector(reader, header.ofs_bounds as u64, header.num_frames)
}

fn load_joints<F: Read + Seek>(reader: &mut F, header: &IQMHeader) -> Result<Vec<IQMJoint>, std::io::Error>
{
	read_vector(reader, header.ofs_joints as u64, header.num_joints)
}

fn load_poses<F: Read + Seek>(reader: &mut F, header: &IQMHeader) -> Result<Vec<IQMPose>, std::io::Error>
{
	read_vector(reader, header.ofs_poses as u64, header.num_poses)
}

fn load_animations<F: Read + Seek>(reader: &mut F, header: &IQMHeader) -> Result<Vec<IQMAnim>, std::io::Error>
{
	read_vector(reader, header.ofs_anims as u64, header.num_anims)
}

fn create_frames<F: Read + Seek>(
	reader: &mut F,
	header: &IQMHeader,
	joints: &[IQMJoint],
	poses: &[IQMPose],
) -> Result<Vec<TriangleModelBoneFrame>, std::io::Error>
{
	// Prepare pairs of base frame and inverted base frame matrices for each joint.
	let mut base_frame_mats = vec![(Mat4f::identity(), Mat4f::identity()); joints.len()];
	for (index, joint) in joints.iter().enumerate()
	{
		let mat = get_bone_matrix(
			Vec3f::from(joint.scale),
			QuaternionF::from_sv(
				joint.rotate[3],
				Vec3f::new(joint.rotate[0], joint.rotate[1], joint.rotate[2]),
			),
			Vec3f::from(joint.translate),
		);
		let inverse_mat = mat.invert().unwrap(); // TODO - avoid unwrap
		let parent_index = joint.parent as usize;
		if parent_index < joints.len()
		{
			let parent_mats = &base_frame_mats[parent_index];
			base_frame_mats[index] = (parent_mats.0 * mat, inverse_mat * parent_mats.1);
		}
		else
		{
			base_frame_mats[index] = (mat, inverse_mat);
		}
	}

	let mut frame_bones = vec![
		TriangleModelBoneFrame {
			matrix: Mat4f::identity()
		};
		poses.len() * (header.num_frames as usize)
	];

	reader.seek(std::io::SeekFrom::Start(header.ofs_frames as u64))?;
	let mut frame_data = Vec::new();
	reader.read_to_end(&mut frame_data)?;

	// TODO - what if alignment is wrong?
	let frame_data_u16 = unsafe { frame_data.align_to::<u16>().1 };
	let mut frame_data_pos = 0;

	for frame_index in 0 .. header.num_frames
	{
		let frame_pose_matrices =
			&mut frame_bones[(frame_index as usize) * poses.len() .. ((frame_index + 1) as usize) * poses.len()];
		for (pose_index, pose) in poses.iter().enumerate()
		{
			let mut translate = Vec3f::new(pose.channeloffset[0], pose.channeloffset[1], pose.channeloffset[2]);
			if (pose.channelmask & 0x01) != 0
			{
				translate.x += frame_data_u16[frame_data_pos] as f32 * pose.channelscale[0];
				frame_data_pos += 1;
			}
			if (pose.channelmask & 0x02) != 0
			{
				translate.y += frame_data_u16[frame_data_pos] as f32 * pose.channelscale[1];
				frame_data_pos += 1;
			}
			if (pose.channelmask & 0x04) != 0
			{
				translate.z += frame_data_u16[frame_data_pos] as f32 * pose.channelscale[2];
				frame_data_pos += 1;
			}

			let mut rotate = QuaternionF::from_sv(
				pose.channeloffset[6],
				Vec3f::new(pose.channeloffset[3], pose.channeloffset[4], pose.channeloffset[5]),
			);
			if (pose.channelmask & 0x08) != 0
			{
				rotate.v.x += frame_data_u16[frame_data_pos] as f32 * pose.channelscale[3];
				frame_data_pos += 1;
			}
			if (pose.channelmask & 0x10) != 0
			{
				rotate.v.y += frame_data_u16[frame_data_pos] as f32 * pose.channelscale[4];
				frame_data_pos += 1;
			}
			if (pose.channelmask & 0x20) != 0
			{
				rotate.v.z += frame_data_u16[frame_data_pos] as f32 * pose.channelscale[5];
				frame_data_pos += 1;
			}
			if (pose.channelmask & 0x40) != 0
			{
				rotate.s += frame_data_u16[frame_data_pos] as f32 * pose.channelscale[6];
				frame_data_pos += 1;
			}

			let mut scale = Vec3f::new(pose.channeloffset[7], pose.channeloffset[8], pose.channeloffset[9]);
			if (pose.channelmask & 0x80) != 0
			{
				scale.x += frame_data_u16[frame_data_pos] as f32 * pose.channelscale[7];
				frame_data_pos += 1;
			}
			if (pose.channelmask & 0x100) != 0
			{
				scale.y += frame_data_u16[frame_data_pos] as f32 * pose.channelscale[8];
				frame_data_pos += 1;
			}
			if (pose.channelmask & 0x200) != 0
			{
				scale.z += frame_data_u16[frame_data_pos] as f32 * pose.channelscale[9];
				frame_data_pos += 1;
			}

			let mat = get_bone_matrix(scale, rotate, translate);
			let mat1 = mat * base_frame_mats[pose_index].1;

			let dst_mat = &mut frame_pose_matrices[pose_index].matrix;

			let parent_pose_index = pose.parent as usize;
			if parent_pose_index < poses.len()
			{
				*dst_mat = base_frame_mats[parent_pose_index].0 * mat1;
			}
			else
			{
				*dst_mat = mat1;
			}
		}
	}

	Ok(frame_bones)
}

fn get_bone_matrix(scale: Vec3f, rotate: QuaternionF, translate: Vec3f) -> Mat4f
{
	let translate = Mat4f::from_translation(translate);
	let rotate = Mat4f::from(rotate / rotate.magnitude());
	let scale = Mat4f::from_nonuniform_scale(scale.x, scale.y, scale.z);

	translate * scale * rotate
}

#[repr(C)]
#[derive(Debug)]
struct IQMHeader
{
	magic: [u8; 16],
	version: u32,
	filesize: u32,
	flags: u32,
	num_text: u32,
	ofs_text: u32,
	num_meshes: u32,
	ofs_meshes: u32,
	num_vertexarrays: u32,
	num_vertexes: u32,
	ofs_vertexarrays: u32,
	num_triangles: u32,
	ofs_triangles: u32,
	ofs_adjacency: u32,
	num_joints: u32,
	ofs_joints: u32,
	num_poses: u32,
	ofs_poses: u32,
	num_anims: u32,
	ofs_anims: u32,
	num_frames: u32,
	num_framechannels: u32,
	ofs_frames: u32,
	ofs_bounds: u32,
	num_comment: u32,
	ofs_comment: u32,
	num_extensions: u32,
	ofs_extensions: u32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
struct IQMMesh
{
	name: u32,
	material: u32,
	first_vertex: u32,
	num_vertexes: u32,
	first_triangle: u32,
	num_triangles: u32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
struct IQMBounds
{
	bbmins: [f32; 3],
	bbmaxs: [f32; 3],
	xyradius: f32,
	radius: f32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
struct IQMTriangle
{
	vertex: [u32; 3],
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
struct IQMVertexArray
{
	type_: u32,
	flags: u32,
	format: u32,
	size: u32,
	offset: u32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
struct IQMJoint
{
	name: u32,
	parent: u32,
	translate: [f32; 3],
	rotate: [f32; 4],
	scale: [f32; 3],
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
struct IQMPose
{
	parent: u32,
	channelmask: u32,
	channeloffset: [f32; 10],
	channelscale: [f32; 10],
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
struct IQMAnim
{
	name: u32,
	first_frame: u32,
	num_frames: u32,
	framerate: f32,
	flags: u32,
}

// const IQM_BYTE: u32 = 0;
const IQM_UBYTE: u32 = 1;
// const IQM_SHORT: u32 = 2;
// const IQM_USHORT: u32 = 3;
// const IQM_INT: u32 = 4;
// const IQM_UINT: u32 = 5;
// const IQM_HALF: u32 = 6;
const IQM_FLOAT: u32 = 7;
// const IQM_DOUBLE: u32 = 8;

const IQM_POSITION: u32 = 0;
const IQM_TEXCOORD: u32 = 1;
const IQM_NORMAL: u32 = 2;
const IQM_TANGENT: u32 = 3;
const IQM_BLENDINDEXES: u32 = 4;
const IQM_BLENDWEIGHTS: u32 = 5;
// const IQM_COLOR: u32 = 6;

const IQM_LOOP: u32 = 1;

const IQM_MAGIC: [u8; 16] = *b"INTERQUAKEMODEL\0";
const IQM_VERSION: u32 = 2;
