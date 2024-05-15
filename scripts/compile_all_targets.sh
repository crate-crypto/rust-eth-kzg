#!/bin/bash

# Install zig build and compile for all the relevant targets
# This should likely only be ran in the CI

# TODO move this cross compile to another lib
# cargo install cargo-zigbuild

# TODO: Add require_install method to check that necessary tools are installed

# For now we only do it for java
cd bindings/java

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

# mkdir -p ./bindings/java/java-code/src/main/resources/x86_64-unknown-linux-gnu
# mkdir -p ./bindings/java/java-code/src/main/resources/aarch64-unknown-linux-gnu
mkdir -p ./bindings/java/java-code/src/main/resources/aarch64-apple-darwin
mkdir -p ./bindings/java/java-code/src/main/resources/x86_64-apple-darwin
mkdir -p ./bindings/java/java-code/src/main/resources/x86_64-pc-windows-gnu

# cp -R ./target/x86_64-unknown-linux-gnu/debug/libjava_peerdas_kzg.so ./bindings/java/java-code/src/main/resources/x86_64-unknown-linux-gnu/
# cp -R ./target/aarch64-unknown-linux-gnu/debug/libjava_peerdas_kzg.so ./bindings/java/java-code/src/main/resources/aarch64-unknown-linux-gnu/
cp -R ./target/aarch64-apple-darwin/debug/libjava_peerdas_kzg.dylib ./bindings/java/java-code/src/main/resources/aarch64-apple-darwin/
cp -R ./target/x86_64-apple-darwin/debug/libjava_peerdas_kzg.dylib ./bindings/java/java-code/src/main/resources/x86_64-apple-darwin/
cp -R ./target/x86_64-pc-windows-gnu/debug/java_peerdas_kzg.dll ./bindings/java/java-code/src/main/resources/x86_64-pc-windows-gnu/
