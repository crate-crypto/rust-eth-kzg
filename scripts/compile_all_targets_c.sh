#!/bin/bash


# Compile dynamic libraries for all relevant targets
# and place them in the bindings/c/build directory
./scripts/compile_to_native.sh Darwin arm64 c_peerdas_kzg dynamic ./bindings/c/build
./scripts/compile_to_native.sh Darwin x86_64 c_peerdas_kzg dynamic ./bindings/c/build
./scripts/compile_to_native.sh Windows x86_64 c_peerdas_kzg dynamic ./bindings/c/build
./scripts/compile_to_native.sh Linux x86_64 c_peerdas_kzg dynamic ./bindings/c/build
./scripts/compile_to_native.sh Linux arm64 c_peerdas_kzg dynamic ./bindings/c/build
