SquareWheel Engine demo

### About

SquareWheel software renderer demo.
It contains single demo map with demonstration of various engine features.


### System requirements

* CPU  - Intel Haswell or newer, AMD Ryzen or newer for default executable version, any other x86_64 processor for "_generic" executable version, Pentium 4 or newer for 32-bit version.
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
* "F" - toggle flashlight
* "↑" - turn up
* "↓" - turn down
* "←" - turn left
* "→" - turn right
* "~" - toggle console
* Mouse - move camera
* Mouse left button - launch test projectile
* "ESC" - quit
* "F12" - make screenshot


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
* "renderer.portals_depth". Affects rendering depth of portals and mirrors. Use 0 to disable rendering of portals and mirrors at all.
* "host.frame_scale". Use value greater than 1 to make image pixilated and increase performance.
* "host.max_fps". Set specific value if demo runs too fast. Set 0 in order to remove FPS limit at all.


### How to build this demo

* Build engine itself. Rust compiler and SDL2 libraries are required. Use build scripts inside "src" directory in order to obtain diffirent executable versions.
* Generate textures, using "export_textures.py" script. MaterialMaker v 0.99 (https://github.com/RodZill4/material-maker/) and GIMP 2.10 are required. Use directory "textures" as destination.
* Copy png textures from "textures_src" into "textures" directory.
* Copy models textures into "textures" directory.
* Export skybox textures, using "sky.blend".
* Export models into "models" directory, using Blender and newest IQM export script (see "iqm" directory in repository root).
* Compile demo map and calculate light for it using "build_demo_map_2.sh" script.
* Package engine executables, launch scripts, demo map, textures, materials, models and config file together.


### Authors

(c) Copyright 2022-2023 "Panzerschrek"
Source code: https://github.com/Panzerschrek/Square-Wheel

Models: Trym Horgen, quaternius, Gladius.s, Danimal
