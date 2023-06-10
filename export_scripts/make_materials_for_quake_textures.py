import argparse
import json
import os
import sys

def generate_material_json(textures_dir, texture_file_name):
	# TODO - process sky textures specially.
	res = { "diffuse": texture_file_name }

	if texture_file_name.startswith("*"):
		res["special_effect"] = { "Turb" : { "amplitude" : 4.0, "wave_length" : 64.0, "frequency" : 0.1875 } }
		if texture_file_name.find("WATER") != -1:
			res["roughness"] = 1.0 / 128.0
			res["blending_mode"] = "Average"
			res["blocks_view"] = False

	fb_image_name = os.path.splitext(texture_file_name)[0] + "_fb.tga"
	if os.path.exists(os.path.join(textures_dir, fb_image_name)):
		res["emissive_layer"] = { "image" : fb_image_name, "light" : [4.0, 4.0, 4.0] }

	for i in range(0, 9):
		if texture_file_name.startswith("+" + str(i)):
			frame_base_image_name = os.path.splitext(texture_file_name)[0][2:]
			next_frame_material_name = "+" + str(i + 1) + frame_base_image_name
			if not os.path.exists(os.path.join(textures_dir, next_frame_material_name + ".tga")):
				next_frame_material_name = "+0" + frame_base_image_name
			res["framed_animation"] = { "duration" : 0.5, "next_material_name": next_frame_material_name }
	
	return res


def main():
	parser= argparse.ArgumentParser(description= 'Converter script.')
	parser.add_argument("--input-dir", help= "input dir with Quake textures", type=str)
	parser.add_argument("--output-file", help= "output material file", type=str)

	args= parser.parse_args()

	result = dict()
	for file_name in os.listdir(args.input_dir):
		name_without_extension = file_name.replace(".tga", "")
		material_json = generate_material_json(args.input_dir, file_name)
		result[name_without_extension] = material_json

	with open(args.output_file, mode = "w") as f:
		f.write(json.dumps(result, indent="\t"))

	return 0

if __name__ == "__main__":
	sys.exit(main())
