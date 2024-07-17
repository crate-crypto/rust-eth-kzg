#!/bin/bash

# Note: we assume you are calling this script from the root of the project
#  ie .github/scripts/compile_all_targets_c.sh

# Compile dynamic libraries for all relevant targets
# and place them in the bindings/c/build directory

# Determine the script's directory and the project root directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

OUT_DIR="$PROJECT_ROOT/bindings/c/build"
LIB_TYPE="dynamic"
LIB_NAME="c_eth_kzg"

$PROJECT_ROOT/scripts/compile_to_native.sh Darwin arm64 $LIB_NAME $LIB_TYPE $OUT_DIR
$PROJECT_ROOT/scripts/compile_to_native.sh Darwin x86_64 $LIB_NAME $LIB_TYPE $OUT_DIR
$PROJECT_ROOT/scripts/compile_to_native.sh Windows x86_64 $LIB_NAME $LIB_TYPE $OUT_DIR
$PROJECT_ROOT/scripts/compile_to_native.sh Linux x86_64 $LIB_NAME $LIB_TYPE $OUT_DIR
$PROJECT_ROOT/scripts/compile_to_native.sh Linux arm64 $LIB_NAME $LIB_TYPE $OUT_DIR
