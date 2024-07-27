#!/bin/bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Compile dynamic libraries for all relevant targets
# and place them in the `./bindings/java/java_code/src/main/resources` directory

OUT_DIR="$PROJECT_ROOT/bindings/java/java_code/src/main/resources"
LIB_TYPE="dynamic"
LIB_NAME="java_eth_kzg"
$PROJECT_ROOT/scripts/compile_to_native.sh Darwin arm64 $LIB_NAME $LIB_TYPE $OUT_DIR zigbuild
$PROJECT_ROOT/scripts/compile_to_native.sh Darwin x86_64 $LIB_NAME $LIB_TYPE $OUT_DIR zigbuild
$PROJECT_ROOT/scripts/compile_to_native.sh Windows x86_64 $LIB_NAME $LIB_TYPE $OUT_DIR zigbuild
$PROJECT_ROOT/scripts/compile_to_native.sh Linux x86_64 $LIB_NAME $LIB_TYPE $OUT_DIR zigbuild
$PROJECT_ROOT/scripts/compile_to_native.sh Linux arm64 $LIB_NAME $LIB_TYPE $OUT_DIR zigbuild