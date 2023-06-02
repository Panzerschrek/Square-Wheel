# SquareWheel

This is a project of software renderer for video games, that uses power of modern CPUs.


## Why?

Because it's cool to write your own software renderer.


## Features

SquareWheel may draw maps (using own map format) with some dynamic objects.
This is (mostly) enough for simple old-style FPS game, like Quake or Unreal.

Main features are related to world static geometry:

* Rendering of static world with textured and lightmapped polygons with efficient invisible surfaces and objects rejecting
* Directional lightmaps
* Normal-mapping (directional lightmap-based)
* Specular lighting (directional lightmap-based) for metals and non-metals
* Dynamic lights
* Translucent surfaces
* Alpha-blending and alpha-test
* Skyboxes
* "Turb" effects for textures
* Decals (for bullet holes, blood spots, etc.)
* Portals and mirrors

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

Install Rust and [SDL2](https://crates.io/crates/sdl2) library.
Build project, using "cargo" or by using one of build scripts in _src_ directory.

Windows and GNU/Linux OS are supported.
But other platforms may be supported too (this is not tested).

SquareWheel projects consists of several components:

* _map_compiler_ - compiler for maps. Quake MAP format is used as source for maps.
* _lightmapper_ - utility for lightmaps/light grid generation.
* _square_wheel_lib_ - main library of this project. May be used by your game project.
* _test_game_ (_square_wheel_ executable) - test game project, used for development.

Test game projects includes _Rapier_ physical engine and _hecs_ library.
You may use them too in your game code and use test game code as base for your game.

It's recommended to build your own game using Rust and including SquareWheel library as dependency.
But you can also use other languages, like C++ or even script languages like Lua.
In order to do this you need to write proper FFI bindings.


## Mapping

For building maps for SquareWheel you may use any map editor with support of Quake MAP format, like TrenchBroom, GTKRadiant, J.A.C.K., etc.

Mapping rules are almost like in Quake - you should avoid leaked maps (but this is not enforced).
It's better to use material with "bsp=false" flag for invisible sides of brushes in order to simplify work of map compiler's work to use better (balanced) BSP tree.

Lightmapper supports simple point and projector lights, surfaces with emissive lights (like sky or lamps), sun light, semitransparent surfaces.

Entities are preserved by map compiler as is, so, you may use any keys and values, specific for your game.


## Technical details


### BSP Tree

Leaf BSP tree is used to organize map polygons.
Each leaf contains convex set of polygons (facing each other).
BSP leafs are connected together via portals.

Portal-based algorithm is used to determine visibility (set of visible leafs) for given camera position.
Visibility determination algorithm is the same as in [Thief](https://nothings.org/gamedev/thief_rendering.html) game.

BSP tree-based algorithm is used to place dynamic objects (models, decals, lights) in world and determine visibility for them.


### Lighting

Lightmaps are used for lighting of world polygons.
Lightmap is a unique texture for each polygon.

There are two set of lightmaps - simple and directional.
Simple lightmaps contain just RGB lighting (HDR).
Directional lightmaps contain ambient and directional light terms of light for each texel.

Models lighting uses pre-computed light grid.
Each texel of light grid contains information about ambient light (using light cube) and dominant light direction.

During models rendering light grid fetch (with linear interpolation) is performed for each model.
Fetch result is later used to calculate lighting for each model vertex (based on its normal).

Static light data (lightmap and light grid) is calculated by _lightmapper_ utility.
Depending on lightmapper settings, map size, details and textures density light calculation may take from one minute up to several hours.


### Surfaces

SquareWheel renderer applies lighting to polygons in separate step prior to rasterization.

Each frame for each visible polygon unique texture is created, with proper mip-level.
This texture is generated based on polygon lightmap and polygon regular texture (normal texture, usually tileable).
For surfaces with specular camera position is used to calculate proper lighting.

Additionaly emissive texture may be added atop of the surface.
This texture is modulated by light, specified in material properties.

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
Triangles within one model are sorted by depth.
View models (like player's weapon) are always drawn atop of any other geometry.

Because of lack of Z-buffer it's possible to see some sorting artifacts.
First, models ordering doesn't work well for intersecting models.
So, if you need to draw several models in same place (like character/monster and its gun) consider to make combined model.
Second, triangles sorting within single models doesn't work well with large triangles. Consider tessellating some triangles of your models if you notice badly-looking sorting artifacts.


### Models rendering

Triangle models needs to be animated and light must be calculated before performing rasterization.
Triangles must be transformed and sorted.
These  computations are performed in separate step.
During rasterization rasterizer uses prepared later vertices/triangles.

Models pre-processing as separate step is needed to avoid duplication of this calculations because same models may be drawn multiple times in several BSP leafs.


### Decals

Decal is just a stretched box, in which volume each world polygon is affected by decal texture.

Decals applying is performed just after rendering of BSP leaf polygons.
Each polygon potentially affected by decal is clipped by 6 planes of decal box.
Remaining polygon is rasterized with decal texture and decal texture coordinates equation projected to polygon plane.

Same triangles rasterization functions are used as for triangle models.
This means per-vertex lighting, no perspective correction or normal-mapping.
Large decals triangles may be tessellated in order to reduce affine texture coordinates interpolation artifacts.

Lightmap data is used to calculate light for decals.
Lightmap fetch is performed per-vertex and with linear interpolation.
Such approach may produce bad results for large decals, so, avoid using large decals, unless you disable lightmap fetch for decal at all and use constant light instead.
But for small decals (like bullet holes) which size is comparable to lightmap texel size per-vertex lighting looks fine.


### Dynamic lighting

The engine supports also dynamic lights.
There are three types of lights - spherical with no shadowmap, spherical with cube shadowmap and conical with projector shadowmap.
Lights radius is limited in order to reduce complexity of light calculations.

Dynamic lighting is not so fast as static and exists mostly in order to implement effects such as muzzle flash, explosions, glowing projectiles, player's flashlight, etc.
It is important to avoid large number of dynamic lights with large radius and/or shadows in order to avoid performance impact.
But it is fine to use small number (up to 10) of dynamic lights without shadows and one or two dynamic lights with large radius and shadows.

Shadowmaps are prepared for each visible light with shadowmap.
Visibility is determined via same leaf BSP tree-based approach, as for other dynamic objects.
Shadowmap resolution is dependent on distance from light source to camera and radius of light source.

During surfaces preparation for each surface influencing dynamic lights list is prepared.
For each of these dynamic lights per-texel light is calculated, using proper texel normal (from normal map) and material properties (roughness/metallicity).

Triangle models are affected by dynamic lights too.
For each model dynamic light cube is prepared (similar as precalculated light grid element), including proper attenuation and shadowing.
Then dynamic lighting is applied using this cube for all vertices of the model.

Dynamic lights light cube is also prepared for decals.
Decal segment light is dependent on this cube and normal of polygon for which this decal segment is applied.


### Skybox

If material of a polygon contains "skybox" property, this polygon will not be drawn as usual.
Instead it is just used to calculate visible sky region bounds (in screen space).
Skybox is rendered prior to all world geometry.
It is clipped with region of all skybox-marked polygons in order to reduce overdraw.

There are some skyboxes limitations.
First, it's not possible to see more than one skybox in one frame.
But it is possible to use different skyboxes on different parts of the map.
Second, it's not possible to draw properly polygons directly behind skybox polygons.
Skybox brush in the middle of a room will not be displayed properly.
But it's still possible to mark as skybox wall polygon of a room with other room behind it, as soon, as visibility determination code rejects all polygons of other room.


### Portals and mirrors

Portals and mirrors are just polygons with special generated on-fly texture.
Separate rendering pass is used to render image in portal or mirror.

Because of that approach the image look pixelated when looking too close.
But this approach is relatively simple and requires no modification of main rendering code, which can slow down it.

Depth of rendering is limited in order to avoid infinite recursion in cases like two mirrors facing each other.
When depth is reached  black texture is used for portal or mirror image.

It is also important to limit portals rendering depth, because for each recursive step of portals rendering separate Renderer class instance is created - including some intermediate structs.
So, if portals rendering limit is too hight, too much memory for such structs will be wasted.

View models (weapon in player hands) are not shown in portals and mirrors.
Models with special flag (like player model) are only shown in portals and mirrors.


### Multithreading

It's absolutely necessary to use multithreading in order to achieve acceptable performance.
So, SquareWheel uses multithreading a lot.

_rayon_ library is used to perform simple multithreaded computations.

Surfaces are generated independent on each other, that allows to parallelize surfaces generation.
Models are independent on each other too, so, models preparation code (animation, lighting, triangles sorting) is multithreaded.
Rasterization itself is (obviously) multithreaded, Each thread performs rasterization into its own rectangular screen region.
HDR postprocessor uses multithreading too, but it gives very little performance increase, since postprocessing is mostly memory-bounding operation.
Lastly, game code may be executed in parallel with final screen update (BitBlt/SwapBuffers call) for previous frame.

Some non-rendering code parts use multithreading too.
This includes map textures loading (in order to speed-up decoding of png/jpg images).
Renderer loading and map-dependent game logic loading is also performing in parallel.


### Materials

Renderer uses not just only raw textures, but materials.
Material is a combination of surface properties, affecting surface appearance.
This includes albedo/normal map texture, glossiness (constant or from glossiness map), metallic flag, blending mode, skybox properties, etc.
Some properties of materials are used not only by renderer, but also by map compiler and lightmapper.

JSON files are used to store materials.
Each JSON file may contain several materials.
SquareWheel loads all materials from current material directory.


### CPU-specific code

SquareWheel uses x86_64 intrinsics in order to speed-up some computations.
This includes integer/floating point vector instructions, some fast approximate instructions.

_f32::mul_add_ function is used in critical for performance places, because it is faster than combination of separate _mul_ and _add_ instructions.

In order to achieve maximum performance you need to build the project targeting modern processors with support of instructions sets SSE2, SSE3, SSE4.1, SSE4.2, FMA.
These instructions are supported by Intel processors starting with Haswell and AMD processors starting with Ryzen.


## Authors

SquareWheel code:
Copyright © 2022-2023 Artöm "Panzerscrek" Kunç.

IQM library:
Copyright (c) 2010-2019 Lee Salzman.

SquareWheel uses a lot of third-party libraries, like _SDL2_, _Rapier_, _hecs_, _serde_, _cgmath_ and others.
See list of dependencies in _src/Carto.toml_ file for more details.
