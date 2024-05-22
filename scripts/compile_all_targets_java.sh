#!/bin/bash

# 
# Compile dynamic libraries for all relevant targets
# and place them in the `./bindings/java/java_code/src/main/resources` directory

OUT_DIR="./bindings/java/java_code/src/main/resources"
LIB_TYPE="dynamic"
LIB_NAME="java_peerdas_kzg"
./scripts/compile_to_native.sh Darwin arm64 $LIB_NAME $LIB_TYPE $OUT_DIR
./scripts/compile_to_native.sh Darwin x86_64 $LIB_NAME $LIB_TYPE $OUT_DIR
./scripts/compile_to_native.sh Windows x86_64 $LIB_NAME $LIB_TYPE $OUT_DIR
./scripts/compile_to_native.sh Linux x86_64 $LIB_NAME $LIB_TYPE $OUT_DIR
./scripts/compile_to_native.sh Linux arm64 $LIB_NAME $LIB_TYPE $OUT_DIR