../src/target/release/map_compiler.exe \
	-i maps/demo2.map \
	-o maps/demo2.sqwm \
	--materials-dir materials \
	--textures-dir textures \
	&&\
../src/target/release/lightmapper.exe \
	--num-threads 4 \
	-i maps/demo2.sqwm \
	-o maps/demo2.sqwm \
	--materials-dir materials \
	--textures-dir textures \
	--num-passes 3 \
	--sample-grid-size 4 \
	--light-grid-cell-width 64 \
	--light-grid-cell-height 128 \
