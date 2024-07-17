#!/bin/bash
# When developing locally, one should call this script to
# build the necessary binaries needed for the other languages
# to interact with the rust library.
# Note: This is specifically for libraries that need to have a compiled
# dynamic or static library.

# Determine the script's directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
OS=$(uname)
ARCH=$(uname -m)

# Function to compile for Java
compile_java() {
    echo "Compiling for Java..."
    OUT_DIR="$PROJECT_ROOT/bindings/java/java_code/src/main/resources"
    LIB_TYPE="dynamic"
    LIB_NAME="java_peerdas_kzg"
    $PROJECT_ROOT/scripts/compile_to_native.sh $OS $ARCH $LIB_NAME $LIB_TYPE $OUT_DIR
}

# Function to compile for C#
compile_csharp() {
    echo "Compiling for C#..."
    OUT_DIR="$PROJECT_ROOT/bindings/csharp/csharp_code/PeerDASKZG.bindings/runtimes"
    LIB_TYPE="dynamic"
    LIB_NAME="c_eth_kzg"
    $PROJECT_ROOT/scripts/compile_to_native.sh $OS $ARCH $LIB_NAME $LIB_TYPE $OUT_DIR
}

# Function to compile for Golang
compile_golang() {
    echo "Compiling for Golang..."
    OUT_DIR="$PROJECT_ROOT/bindings/golang/build"
    LIB_TYPE="static"
    LIB_NAME="c_eth_kzg"
    $PROJECT_ROOT/scripts/compile_to_native.sh $OS $ARCH $LIB_NAME $LIB_TYPE $OUT_DIR
    # Copy header file
    cp $PROJECT_ROOT/bindings/c/build/c_eth_kzg.h $OUT_DIR
}

# Function to compile for Nim
compile_nim() {
    echo "Compiling for Nim..."
    OUT_DIR="$PROJECT_ROOT/bindings/nim/nim_code/build"
    LIB_TYPE="static"
    LIB_NAME="c_eth_kzg"
    # Check if the OS is Darwin (macOS) and set ARCH_MODIFIED to universal if true.
    if [[ "$OS" == "Darwin" ]]; then
        # Install both targets for mac, so that it won't fail in CI
        rustup target add x86_64-apple-darwin
        rustup target add aarch64-apple-darwin
        ARCH_MODIFIED="universal"
    else
        ARCH_MODIFIED=$ARCH
    fi
    $PROJECT_ROOT/scripts/compile_to_native.sh $OS $ARCH_MODIFIED $LIB_NAME $LIB_TYPE $OUT_DIR
}

# Function to compile for all languages
compile_all() {
    compile_java
    compile_csharp
    compile_golang
    compile_nim
}

# If no argument is provided, compile for all languages
if [ $# -eq 0 ]; then
    compile_all
    exit 0
fi

# Compile based on the argument
case $1 in
    java)
        compile_java
        ;;
    csharp)
        compile_csharp
        ;;
    golang)
        compile_golang
        ;;
    nim)
        compile_nim
        ;;
    *)
        echo "Invalid argument. Use java, csharp, golang, nim, or run without arguments to compile for all languages."
        exit 1
        ;;
esac