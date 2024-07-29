#!/bin/bash

# Determine the script's directory and the project root directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

OUT_DIR="$PROJECT_ROOT/bindings/csharp/csharp_code/EthKZG.bindings/runtimes"
LIB_TYPE="dynamic"
LIB_NAME="c_eth_kzg"
$PROJECT_ROOT/scripts/compile_to_native.sh Darwin arm64 $LIB_NAME $LIB_TYPE $OUT_DIR
$PROJECT_ROOT/scripts/compile_to_native.sh Darwin x86_64 $LIB_NAME $LIB_TYPE $OUT_DIR
$PROJECT_ROOT/scripts/compile_to_native.sh Windows x86_64 $LIB_NAME $LIB_TYPE $OUT_DIR
$PROJECT_ROOT/scripts/compile_to_native.sh Linux x86_64 $LIB_NAME $LIB_TYPE $OUT_DIR
$PROJECT_ROOT/scripts/compile_to_native.sh Linux arm64 $LIB_NAME $LIB_TYPE $OUT_DIR
