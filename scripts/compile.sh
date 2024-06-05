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

# Compile rust code for java library
OUT_DIR="$PROJECT_ROOT/bindings/java/java_code/src/main/resources"
LIB_TYPE="dynamic"
LIB_NAME="java_peerdas_kzg"
$PROJECT_ROOT/scripts/compile_to_native.sh $OS $ARCH $LIB_NAME $LIB_TYPE $OUT_DIR

# Compile Rust code for c sharp
OUT_DIR="$PROJECT_ROOT/bindings/csharp/csharp_code/PeerDASKZG.bindings/runtimes"
LIB_TYPE="dynamic"
LIB_NAME="c_peerdas_kzg"
$PROJECT_ROOT/scripts/compile_to_native.sh $OS $ARCH $LIB_NAME $LIB_TYPE $OUT_DIR

# Compile Rust code for golang
OUT_DIR="$PROJECT_ROOT/bindings/golang/build"
LIB_TYPE="static"
LIB_NAME="c_peerdas_kzg"
$PROJECT_ROOT/scripts/compile_to_native.sh $OS $ARCH $LIB_NAME $LIB_TYPE $OUT_DIR
# Copy header file
cp $PROJECT_ROOT/bindings/c/build/c_peerdas_kzg.h $OUT_DIR

# Compile Rust code for nimble
OUT_DIR="$PROJECT_ROOT/bindings/nim/nim_code/build"
LIB_TYPE="static"
LIB_NAME="c_peerdas_kzg"

# Check if the OS is Darwin (macOS) and set ARCH_MODIFIED to universal if true.
# nim has issues with M1s where they will install an x86_64 version of nim 
# on an arm chip, so we need compile a universal binary so that nim can use whichever
# architecture it needs.
if [[ "$OS" == "Darwin" ]]; then
    # Install both targets for mac, so that it won't fail in CI
    rustup target add x86_64-apple-darwin
    rustup target add aarch64-apple-darwin
    ARCH_MODIFIED="universal"
else
    ARCH_MODIFIED=$ARCH
fi
$PROJECT_ROOT/scripts/compile_to_native.sh $OS $ARCH_MODIFIED $LIB_NAME $LIB_TYPE $OUT_DIR