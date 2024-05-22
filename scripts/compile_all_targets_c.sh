#!/bin/bash


# Compile dynamic libraries for all relevant targets
# and place them in the bindings/c/build directory

OUT_DIR="./bindings/c/build"
LIB_TYPE="dynamic"
LIB_NAME="c_peerdas_kzg"
./scripts/compile_to_native.sh Darwin arm64 $LIB_NAME $LIB_TYPE $OUT_DIR
./scripts/compile_to_native.sh Darwin x86_64 $LIB_NAME $LIB_TYPE $OUT_DIR
./scripts/compile_to_native.sh Windows x86_64 $LIB_NAME $LIB_TYPE $OUT_DIR
./scripts/compile_to_native.sh Linux x86_64 $LIB_NAME $LIB_TYPE $OUT_DIR
./scripts/compile_to_native.sh Linux arm64 $LIB_NAME $LIB_TYPE $OUT_DIR
