SquareWheel Engine demo

### About

SquareWheel software renderer demo.
It contains single demo map with demonstration of various engine features.


### System requirements

* CPU  - Intel Haswell or newer, AMD Bulldozer or newer.
* OS - 64bit Windows 7 or newer or any modern GNU/Linux distribution with SDL2 libraries installed. But it's possible to build the engine from source code for other platforms.
* 1Gb of RAM.
* Keyboard, mouse.


### Running

Use one of "run_demo" scripts.
Use script with "i686" suffix on 32-bit system, use script with "_generic" suffix on old x86_64 CPUs.


### Controls

* "W" - move forward
* "S" - move backward
* "A" - move left
* "D" - move right
* "SPACE" - jump
* "C" - fly down (in noclip mode)
* "↑" - turn up
* "↓" - turn down
* "←" - turn left
* "→" - turn right
* "~" - toggle console
* Mouse - move camera
* "ESC" - quit


### Useful consloe commands

* "resize_window" - set specific window size.
* "quit" - quit demo.
* "map" - load specified map.
* "noclip" - toggle noclip mode.


### Useful consloe variables

* "host.fullscreen_mode". 0 - windowed mode, 1 - borderless window with current desktop resolution, 2 - fullscreen mode with current window resolution.
* "host.num_threads" - number of CPU threads, used for rendering. 0 - auto.
* "postprocessor.hdr_rendering" (true/false) - Enable/disable HDR rendering.
* "renderer.use_directional_lightmaps" (true/false) - Enable/disable directional lightmaps.
* "renderer.textures_mip_bias". Affects textures quality. Default value is 0, negative value for overdetailed textures, positive value for lower quality.


### How to build this demo

* Build engine itself. Rust compiler and SDL2 libraries are required.
* Generate textures, using "export_textures.py" script. MaterialMaker v 0.99 (https://github.com/RodZill4/material-maker/) and GIMP 2.10 are required.
* Compile demo map using "map_compiler" executable and build lightmaps, using "lightmapper" executable.
* Export models, using blender and newest IQM export script (see iqm directory in repository root).
* Package engine executables, demo map, textures, materials and config file together.


### Authors

(c) Copyright 2022 "Panzerschrek"
Source code: https://github.com/Panzerschrek/Square-Wheel

Models: Trym Horgen, quaternius
