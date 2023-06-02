import argparse
import os
import shutil
import subprocess
import sys

material_maker_executable = "material_maker"
gimp_executable = "gimp"

def generate_textures(in_files_list, out_dir):
	args = [material_maker_executable, "--export", "-t", "godot", "-o", out_dir, "-i"]
	args.extend(in_files_list)
	subprocess.run(args)


# Textures are generated in large resolution (to achieve better quality).
# Downsample textures to size, used by the engine.
# Flip also image vertically in order to compensate difference in coordinate system. This is important for normal maps.
def finalize_image(in_file, out_file):
	export_script_template = """
in_file_name = "{}"
image = gimp.pdb.gimp_file_load(in_file_name, 0)
w = gimp.pdb.gimp_image_width(image)
h = gimp.pdb.gimp_image_height(image)
layer = gimp.pdb.gimp_image_get_active_layer(image)
gimp.pdb.gimp_layer_scale(layer, w / 4, h / 4, True)
gimp.pdb.gimp_flip(layer, 1)
out_file_name = "{}"
gimp.pdb.gimp_file_save(image, layer, out_file_name, out_file_name)
gimp.pdb.gimp_quit(0)
	"""
	export_script = export_script_template.format(in_file.replace("\\", "/"), out_file.replace("\\", "/"))
	args = [gimp_executable, "-idfsc", "--batch-interpreter", "python-fu-eval", "-b", export_script ]
	subprocess.run(args)


# Perform scale and flip but also extract roughness (n green channel) and put it inot other channels.
def finalize_image_roughness(in_file, out_file):
	export_script_template = """
in_file_name = "{}"
image = gimp.pdb.gimp_file_load(in_file_name, 0)
w = gimp.pdb.gimp_image_width(image)
h = gimp.pdb.gimp_image_height(image)
layer = gimp.pdb.gimp_image_get_active_layer(image)
gimp.pdb.gimp_layer_scale(layer, w / 4, h / 4, True)
gimp.pdb.gimp_flip(layer, 1)
gimp.pdb.plug_in_colors_channel_mixer(image, layer, 0,  0, 1, 0,  0, 1, 0,  0, 1, 0)
out_file_name = "{}"
gimp.pdb.gimp_file_save(image, layer, out_file_name, out_file_name)
gimp.pdb.gimp_quit(0)
	"""
	export_script = export_script_template.format(in_file.replace("\\", "/"), out_file.replace("\\", "/"))
	args = [gimp_executable, "-idfsc", "--batch-interpreter", "python-fu-eval", "-b", export_script ]
	subprocess.run(args)


def main():
	parser= argparse.ArgumentParser(description= 'Textures export escript.')
	parser.add_argument("--input", help= "Directory with source textures or single file", type=str)
	parser.add_argument("--output-dir", help= "Directory with output textures", type=str)

	args= parser.parse_args()

	input_dir_or_file = os.path.abspath(args.input)
	output_dir = os.path.abspath(args.output_dir)
	intermediate_dir = os.path.abspath("textures_intermediate")

	if os.path.isdir(input_dir_or_file):
		print("Collecting list of material files", flush = True)
		in_files_list = []
		for file_name in os.listdir(input_dir):
			in_files_list.append(os.path.join(input_dir, file_name))
	else:
		in_files_list = [input_dir_or_file]

	print("Generate textures", flush = True)
	os.makedirs(intermediate_dir, exist_ok= True)
	generate_textures(in_files_list, intermediate_dir)

	print("Finalize textures", flush = True)
	os.makedirs(output_dir, exist_ok= True)
	for file_name in os.listdir(intermediate_dir):
		if not file_name.endswith(".tres"):
			continue

		base_texture_name = file_name.replace(".tres", "")

		albedo_file_name = base_texture_name + "_albedo.png"
		albedo_file_path = os.path.join(intermediate_dir, albedo_file_name)
		if os.path.exists(albedo_file_path):
			finalize_image(albedo_file_path, os.path.join(output_dir, base_texture_name + ".png"))

		normal_file_name = base_texture_name + "_normal.png"
		normal_file_path = os.path.join(intermediate_dir, normal_file_name)
		if os.path.exists(normal_file_path):
			finalize_image(normal_file_path, os.path.join(output_dir, base_texture_name + "_normal.png"))

		orm_file_name = base_texture_name + "_orm.png"
		orm_file_path = os.path.join(intermediate_dir, orm_file_name)
		if os.path.exists(orm_file_path):
			finalize_image_roughness(orm_file_path, os.path.join(output_dir, base_texture_name + "_roughness.png"))

	print("Remove intermediate directory", flush = True)
	shutil.rmtree(intermediate_dir)


if __name__ == "__main__":
	sys.exit(main())
