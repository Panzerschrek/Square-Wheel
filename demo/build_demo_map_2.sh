# Performing CSG helps for demo2.
# But for other maps this option may not gove good results.
../src/target/release/map_compiler \
	-i maps/demo2.map \
	-o maps/demo2.sqwm \
	--materials-dir materials \
	--textures-dir textures \
	--perform-csg \
	--perform-advanced-splitter-plane-selection \
	&&\
../src/target/release/lightmapper \
	--num-threads 4 \
	-i maps/demo2.sqwm \
	-o maps/demo2.sqwm \
	--materials-dir materials \
	--textures-dir textures \
	--num-passes 3 \
	--sample-grid-size 4 \
	--light-grid-cell-width 64 \
	--light-grid-cell-height 64 \
