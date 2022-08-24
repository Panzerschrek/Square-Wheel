# SquareWheel

This is a project of software renderer for video games, that utilizes a lot of features of modern CPUs.


## Why?

Because it's cool to write your own software renderer.


## Features

SquareWheel may draw maps (using own map format) with some dynamic objects.
This is (mostly) enough for simple old-style FPS game, like Quake or Unreal.

Main features are related to world static geometry:
* Rendering of static world with textured and lightmapped polygons with efficient invisible surfaces and objects rejecting
* Directional lightmaps
* Normal-mapping (directional liggtmap-based)
* Specular lighting (directional lightmap-based) for metalls and non-metals
* Translucent surfaces
* Alpha-blending and alpha-test
* Skyboxes
* "Turb" effects for textures
* Decals (for bullet holes, blood spots, etc.)

Triangle models rendering is supported too, including:
* MD3 format support (per-vertex animation)
* IQM format support (skeleton animation)
* Per-vertex lighting, based on static light data (light grid)

Besides static world and triangle models there is a way to draw dynamic objects, using polygons like in static world, using so-called brush models (inline models).
This is mostly used for doors, plates, elevators, buttons, etc.

SquareWheel has support of HDR rendering.
The renderer may work in 64-bit mode to produce HDR picture and separate postprocessor component will perform tonemapping to produce result 32-bit picture.


## Non-renderer related stuff

SquareWheel project contains some helper functionality, that may be used not only by renderer, but also by game code.
This includes:
* Console (Quake-style) with auto-completion and possibility to execute various commands and change config values
* Resources management class
* Config management code


## Non-existing functionality

SquareWheel is still mostly a raw renderer with no good game code upport.
It is not a true general game engine, but more a base framework with software renderer as its heart.

List on missing features:
* Sound code
* Network code
* AI, path-finding and other game-related coe


## Development

SquareWheel is written on Rust (1.60.0 or newer).
SDL2 library is used for window creation/input processing.

Install Rust and [https://crates.io/crates/sdl2](SDL2) library.
Build project, using "cargo" or by using one of build scripts in _src_ directory.

Windows and GNU/Linux OS are supported. But other platforms may be supported too (this is not tested).

SquareWheel projects consists of several components:
* _map_compiler_ - compiler for maps. Quake MAP format is used as source for maps.
* _lightmapper_ - utility for lightmaps/light grid generation.
* _square_wheel_lib_ - main library of this project. May be used by your game project.
* _test_game_ (_square_wheel _executable) - test game project, used for development.

Test game projects includes _Rapier_ physical engine and _hecs_ library.
You may use them too in your game code and use test game code as base for your game.


## Technical details


### BSP Tree

Leaf BSP tree is used to organize map polygons.
Each leaf contains convex set of polygons (facing each other).
BSP leafs are connected together via portals.

Portal-based algorithm is used to determine visibility (set of visible leafs) for given camera position.
Visibility definition algorithm is the same as in [https://nothings.org/gamedev/thief_rendering.html](Thief) game.

BSP tree-based algorithm is used to place dynamic objects (models, decals) in world and determine visibility for them.


### Lighting

Lightmaps are used for lighting of world polygons.
Lightmap is a unique texture for each polygon.

There are two set of lightmaps - simple and regular.
Simple lightmaps contains just RGB lighting (HDR).
Directional lightmaps contains ambient and directional light terms of light for given texel.

Models lighting useds pre-computed light grid.
Each texel of light grid contains information about ambient light (using light cube) and dominan light direction.

During models rendering light grid fetch (with linear interpolation) is performed for each model.
Fetch result is later used to calculate lighting for each model vertex (based on its normal).

Static light data (lightmap and light grid) is calculated by _lightmapper_ utility.
Depending on lightmapper settings, map size, details and textures density light calculation may take from one minute up to several hours.


### Surfaces

SquareWheel renderer applies lightting to polygons in separate step prior to rasterization.

Each frame for each visible polygon unique texture is created, with proper mip-level.
This texture is generated based on polygon lightmap and polygon regular texture (normal texture, usually tileable).
For surfaces with specular camera position is used to calculate proper lighting.

Result polygon texture (aka "surface") is used later for rasterization.

Approach with lighting as separate step allows to simplify rendering code and reduce computational cost, because usually there are less surfaces texels than screen pixels.


### Ordering

SquareWheel uses no Z-buffer or even span-buffer to order objects.
Instead it uses painter algorithm - draws all polygons/objects in back-to-front order.

Skybox is always drawn first.
Than renderer performs world rendering, using BSP tree to sort BSP leafs in required order.
Polygons of leaf are drawn first.
Than renderer performs decals drawing.
Dynamic objects (models) are drawn in each leaf where they are located with clipping by leaf portals and polygons.
Dynamic objects within single leaf are sorted using some sort of painter algorithm (with some improvements).
Triangles within one model are soprted by depth.
View models (like player's weapon) are always drawn atop of any other geometry.

Because of lack of Z-buffer it's possible to see some sorting artifacts.
First, models ordering doesn't work well for intersecting models.
So, if you need to draw several models in same place (like character/monster and its gun) consider to make combined model.
Second, triangles sorting within single models doesn't work well with large triangles. Consider tesselating some triangles of your models if you notice badly-looking sorrting artifacts.


### Models rendering

Triangle models needs to be animated and light must be calculated before performing rasterization.
Triangles must be transformed and sorted.
These  computations are performed in separate step.
During rasterization rasterizer uses prepared later vertices/triangles.

Models pre-processing as separate step is needed because same models may be drawn multiple times in several BSP leafs.


### Multithreading

It's absolutely necessary to use multithreading in order to achive acceptible performance.
So, SquareWheel uses multithreading a lot.

_rayon_ library is used to perform simple multithreaded computations.

Surfaces are generated independent on each other, that allows to parallelize surfaces generation.
Models are independent on each other too, so, models preparation code (animation, lighting, triangles sorting) is multithreaded.
Rasterization itself is (obviously) multithreaded, Each thread performs rasterization into its own rectangular screen region.
HDR postprocessor uses multithreading too, but it gives very little performance increase, since postprocessing is mostly memory-bounding operation.
Lastly, game code may be executed in parallel with final screen update (BitBlt/SwapBuffers call) for previous frame.


### Materials

Renderer uses not just only raw textures, but materials.
Material is a combination of surface properties, affecting surface appearance.
This includes albedo/normal map texture, glossiness (constant or from glossiness map), metallic flag, blending mode, skybox properties, etc.
Some properties of materials are used not only by renderer, but also by map compiler and lightmapper.

JSON files are used to store materials.
Each JSON file may contain several materials.
SquareWheel loads all materials from current material directory.