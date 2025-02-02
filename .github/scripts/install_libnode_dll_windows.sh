#!/bin/bash

# Check if required environment variables are set
if [ -z "$RUNNER_TOOL_CACHE" ] || [ -z "$NODE_VERSION" ]; then
    echo "Error: RUNNER_TOOL_CACHE and NODE_VERSION must be set"
    exit 1
fi

# Set libnode version
LIBNODE_VERSION="v23.0.0"

echo "Downloading libnode..."
curl -L -o libnode.zip "https://github.com/metacall/libnode/releases/download/${LIBNODE_VERSION}/libnode-amd64-windows.zip"

echo "Extracting libnode..."
unzip libnode.zip

NODE_DIR="$RUNNER_TOOL_CACHE/node/$NODE_VERSION/x64"
echo "Creating directory if not exists..."
mkdir -p "$NODE_DIR"

echo "Moving libnode.dll to Node installation directory..."
cp libnode.dll "$NODE_DIR/"

echo "Verify file was copied:"
ls -la "$NODE_DIR/libnode.dll"

# Set environment variable
echo "LIBNODE_PATH=$NODE_DIR" >> $GITHUB_ENV

echo "Setup completed successfully"