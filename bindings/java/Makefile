java_run: lib
	javac LibPeerDASKZG.java && java -Djava.library.path=../../target/debug/ LibPeerDASKZG

.PHONY: lib

javah:
	javac -h . LibPeerDASKZG.java

lib:
	cargo build