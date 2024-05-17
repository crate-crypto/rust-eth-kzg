#!/bin/bash

# First build in release mode, this should build the c crate
cargo build --release

# Determine the operating system
OS="$(uname)"
ARCH="$(uname -m)"
echo "Detected OS: $OS"
echo "Detected architecture: $ARCH"

case "$OS" in
    "Darwin")
        echo "Running on macOS"
        case "$ARCH" in
            "arm64")
                echo "Detected macOS on ARM (Apple Silicon)"
                # Copy static and shared libraries for macOS ARM
                mkdir -p ./bindings/c/build/darwin-aarch64
                cp -R target/release/libc_peerdas_kzg.dylib ./bindings/c/build/darwin-aarch64/
                cp -R target/release/libc_peerdas_kzg.a ./bindings/c/build/darwin-aarch64/
                ;;
            "x86_64")
                echo "Detected macOS on Intel"
                # Copy static and shared libraries for macOS Intel
                mkdir -p ./bindings/c/build/darwin-amd64
                cp -R target/release/libc_peerdas_kzg.dylib ./bindings/c/build/darwin-amd64/
                cp -R target/release/libc_peerdas_kzg.a ./bindings/c/build/darwin-amd64/
                ;;
            *)
                echo "Unsupported macOS architecture: $ARCH"
                exit 1
                ;;
        esac
        ;;
    "Linux")
        echo "Running on Linux"
        case "$ARCH" in
            "arm64")
                echo "Detected Linux on ARM"
                # Copy static and shared libraries for Linux ARM
                mkdir -p ./bindings/c/build/linux-aarch64
                cp target/release/libc_peerdas_kzg.a ./bindings/c/build/linux-aarch64/
                cp target/release/libc_peerdas_kzg.so ./bindings/c/build/linux-aarch64/
                ;;
            "x86_64")
                echo "Detected Linux on Intel"
                # Copy static and shared libraries for Linux Intel
                mkdir -p ./bindings/c/build/linux-amd64
                cp target/release/libc_peerdas_kzg.a ./bindings/c/build/linux-amd64/
                cp target/release/libc_peerdas_kzg.so ./bindings/c/build/linux-amd64/
                ;;
            *)
                echo "Unsupported Linux architecture: $ARCH"
                exit 1
                ;;
        esac
        ;;
    "MINGW64_NT"|"CYGWIN_NT")
        echo "Running on Windows"
        mkdir -p ./bindings/c/build/windows/
        # Copy static and shared libraries for Windows
        cp target/release/libc_peerdas_kzg.lib ./bindings/c/build/windows/
        cp target/release/libc_peerdas_kzg.dll ./bindings/c/build/windows/
        ;;
    *)
        echo "Unsupported OS: $OS"
        exit 1
        ;;
esac
