import argparse
import json
import os
import sys

def generate_material_json(texture_file_name):
	# TODO - process sky textures specially. Also generate proper materials for animated textures and deal wirh fullbrights.
	res = { "diffuse": texture_file_name }
	if texture_file_name.startswith("*"):
		res["turb"] = { "amplitude" : 4.0, "wave_length" : 64.0, "frequency" : 0.1875 }
		if texture_file_name.find("WATER") != -1:
			res["glossiness"] = 1.0
	return res


def main():
	parser= argparse.ArgumentParser(description= 'Converter script.')
	parser.add_argument("--input-dir", help= "input dir with Quake textures", type=str)
	parser.add_argument("--output-file", help= "output material file", type=str)

	args= parser.parse_args()

	result = dict()
	for file_name in os.listdir(args.input_dir):
		name_without_extension = file_name.replace(".tga", "")
		material_json = generate_material_json(file_name)
		result[name_without_extension] = material_json

	with open(args.output_file, mode = "w") as f:
		f.write(json.dumps(result, indent="\t"))

	return 0

if __name__ == "__main__":
	sys.exit(main())
