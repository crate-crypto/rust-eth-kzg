#!/bin/bash

# When developing locally, one should call this script to 
# build the necessary binaries needed for the other languages 
# to interact with the rust library.

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
OUT_DIR="$PROJECT_ROOT/bindings/csharp/runtimes"
LIB_TYPE="dynamic"
LIB_NAME="c_peerdas_kzg"
$PROJECT_ROOT/scripts/compile_to_native.sh $OS $ARCH $LIB_NAME $LIB_TYPE $OUT_DIR