ROOT_DIR := $(dir $(realpath $(lastword $(MAKEFILE_LIST))))

.PHONY: build
build:
# compile the c ffi rust crate 
	@cd ../c && cargo build --release
# Copy the static c lib from target to c libraries lib folder
# TODO: move this to the c crate 
	@cp ../../target/release/libc_peerdas_kzg.a ../c/lib
	@cp -R ../c/lib/ ./lib/
	go build main.go

.PHONY: run
run: build
	@./main

