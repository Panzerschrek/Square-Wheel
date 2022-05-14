import argparse
import json
import os
import sys

def generate_trivial_material_json(texture_file_name):
	return { "diffuse": texture_file_name }


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
				result[material_name] = generate_trivial_material_json(file_path)

	with open(args.output_file, mode = "w") as f:
		f.write(json.dumps(result, indent="\t"))

	return 0

if __name__ == "__main__":
	sys.exit(main())
