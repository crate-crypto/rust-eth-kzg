#!/bin/bash

# Install zig build and compile for all the relevant targets
# This should likely only be ran in the CI
# TODO: This is a duplicate of the compile_all_targets_java.sh script -- deduplicate

# TODO move this cross compile to another lib
# cargo install cargo-zigbuild

# TODO: Add require_install method to check that necessary tools are installed

# For now we only do it for java
cd bindings/c

# rustup target add x86_64-unknown-linux-gnu
# cargo build --target=x86_64-unknown-linux-gnu

# rustup target add aarch64-unknown-linux-gnu
# cargo build --target=aarch64-unknown-linux-gnu

rustup target add aarch64-apple-darwin
cargo build --target=aarch64-apple-darwin

rustup target add x86_64-apple-darwin
cargo build --target=x86_64-apple-darwin

rustup target add x86_64-pc-windows-gnu
cargo build --target=x86_64-pc-windows-gnu

cd ../..

# mkdir -p ./bindings/c/build/x86_64-unknown-linux-gnu
# mkdir -p ./bindings/c/build/aarch64-unknown-linux-gnu
mkdir -p ./bindings/c/build/aarch64-apple-darwin
mkdir -p ./bindings/c/build/x86_64-apple-darwin
mkdir -p ./bindings/c/build/x86_64-pc-windows-gnu
# TODO: NOTE: The static libs are not being copied over for C. Currently only needed for golang

# cp -R ./target/x86_64-unknown-linux-gnu/debug/libjava_peerdas_kzg.so ./bindings/c/build/x86_64-unknown-linux-gnu/
# cp -R ./target/aarch64-unknown-linux-gnu/debug/libjava_peerdas_kzg.so ./bindings/c/build/aarch64-unknown-linux-gnu/
cp -R ./target/aarch64-apple-darwin/debug/libc_peerdas_kzg.dylib ./bindings/c/build/aarch64-apple-darwin/
cp -R ./target/x86_64-apple-darwin/debug/libc_peerdas_kzg.dylib ./bindings/c/build/x86_64-apple-darwin/
cp -R ./target/x86_64-pc-windows-gnu/debug/c_peerdas_kzg.dll ./bindings/c/build/x86_64-pc-windows-gnu/
