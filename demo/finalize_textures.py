image = gimp.pdb.gimp_file_load("/home/panzerschrek/Projects/Square-Wheel/other/textures_exported/roof_tiles_albedo.png", 0)
w = gimp.pdb.gimp_image_width(image)
h = gimp.pdb.gimp_image_height(image)
layer = gimp.pdb.gimp_image_get_active_layer(image)
gimp.pdb.gimp_layer_scale(layer, w / 4, h / 4, True)
out_file_name = "/home/panzerschrek/Projects/Square-Wheel/other/textures_exported/roof_tiles_resized.png"
gimp.pdb.gimp_file_save(image, layer, out_file_name, out_file_name)

gimp.pdb.gimp_quit(0)