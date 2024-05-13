java_run: lib
	javac LibPeerDASKZG.java && java -Djava.library.path=../../target/debug/ LibPeerDASKZG

.PHONY: lib

# TODO This shouldn't be needed to be called maually
# TODO: since cargo will do it.
javah:
	javac -h . LibPeerDASKZG.java

lib:
	cargo build

# Copy the dynamic library from the target directory to 
# TODO: This is for mac right now
copy-from-target:
	@cp -R ../../target/debug/libjava_peerdas_kzg.dylib ./lib/