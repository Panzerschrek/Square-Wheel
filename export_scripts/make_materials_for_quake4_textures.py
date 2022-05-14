import argparse
import json
import os
import sys

def generate_trivial_material_json(diffuse_texture, normal_map_texture):
	result = dict()
	result["diffuse"] = diffuse_texture;
	if not normal_map_texture is None:
		result["normal_map"] = normal_map_texture;
	return result


def main():
	parser= argparse.ArgumentParser(description= 'Converter script.')
	parser.add_argument("--input-dir", help= "input dir with Quake textures", type=str)
	parser.add_argument("--output-file", help= "output material file", type=str)

	args= parser.parse_args()

	result = dict()
	# Currently just generate trivial materials for textures with "_d" (diffuse) postfix.
	for dir_path, dirs, files in os.walk(args.input_dir):
		for file in files:
			file_path = os.path.join(os.path.relpath(dir_path, args.input_dir), file)
			if file_path.endswith("_d.tga"):
				material_name = "textures/" + file_path.replace("_d.tga", "")
				normal_map_file_path = file_path.replace("_d.tga", "_local.tga")
				if not os.path.exists(os.path.join(args.input_dir, normal_map_file_path)):
					normal_map_file_path = None
				result[material_name] = generate_trivial_material_json(file_path, normal_map_file_path)

	with open(args.output_file, mode = "w") as f:
		f.write(json.dumps(result, indent="\t"))

	return 0

if __name__ == "__main__":
	sys.exit(main())
