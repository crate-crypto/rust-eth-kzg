#!/bin/bash

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
                mkdir -p ./bindings/java/java_code/src/main/resources/aarch64-apple-darwin
                cp -R target/release/libjava_peerdas_kzg.dylib ./bindings/java/java_code/src/main/resources/aarch64-apple-darwin/
                ;;
            "x86_64")
                echo "Detected macOS on Intel"
                mkdir -p ./bindings/java/java_code/src/main/resources/x86_64-apple-darwin
                cp -R target/release/libjava_peerdas_kzg.dylib ./bindings/java/java_code/src/main/resources/x86_64-apple-darwin/
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
                mkdir -p ./bindings/java/java_code/src/main/resources/aarch64-unknown-linux-gnu
                cp target/release/libjava_peerdas_kzg.so ./bindings/java/java_code/src/main/resources/aarch64-unknown-linux-gnu/
                ;;
            "x86_64")
                echo "Detected Linux on Intel"
                mkdir -p ./bindings/java/java_code/src/main/resources/x86_64-unknown-linux-gnu
                cp target/release/libjava_peerdas_kzg.so ./bindings/java/java_code/src/main/resources/x86_64-unknown-linux-gnu/
                ;;
            *)
                echo "Unsupported Linux architecture: $ARCH"
                exit 1
                ;;
        esac
        ;;
    "MINGW64_NT"|"CYGWIN_NT")
        echo "Running on Windows"
        mkdir -p ./bindings/java/java_code/src/main/resources/x86_64-pc-windows-gnu/
        cp target/release/libjava_peerdas_kzg.dll ./bindings/java/java_code/src/main/resources/x86_64-pc-windows-gnu/
        ;;
    *)
        echo "Unsupported OS: $OS"
        exit 1
        ;;
esac
