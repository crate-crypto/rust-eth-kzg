#!/bin/bash

# Determine the script's directory and the project root directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

cd $PROJECT_ROOT/bindings/node

mv $PROJECT_ROOT/.cargo/config.node.cross.toml $PROJECT_ROOT/.cargo/config.toml

npm install -g @napi-rs/cli

napi build --platform --release --target x86_64-unknown-linux-gnu
napi build --platform --release --target aarch64-unknown-linux-gnu
napi build --platform --release --target x86_64-pc-windows-gnu
napi build --platform --release --target x86_64-apple-darwin
napi build --platform --release --target aarch64-apple-darwin

mv $PROJECT_ROOT/.cargo/config.toml $PROJECT_ROOT/.cargo/config.node.cross.toml