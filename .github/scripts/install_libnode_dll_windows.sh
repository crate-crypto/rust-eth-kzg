#!/bin/bash

# TODO: napi-build requires libnode.dll to be present on the system.
# Most builds on nodejs do not come with libnode.dll, instead they package
# everything in one binary. We download the libnode.dll file from the below releases
# for nodeJs 23.0.0. In the future, we might want to compile from nodeJs source itself and
# also only make this trigger on the compilation for the node bindings.

curl -L -o libnode.zip "https://github.com/metacall/libnode/releases/download/v23.0.0/libnode-amd64-windows.zip"
unzip libnode.zip libnode.dll
pwd_output=$(pwd)
echo "LIBNODE_PATH=$pwd_output" >> $GITHUB_ENV