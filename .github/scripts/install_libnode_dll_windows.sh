#!/bin/bash

curl -L -o libnode.zip "https://github.com/metacall/libnode/releases/download/v23.0.0/libnode-amd64-windows.zip"
unzip libnode.zip libnode.dll
pwd_output=$(pwd)
echo "LIBNODE_PATH=$pwd_output" >> $GITHUB_ENV