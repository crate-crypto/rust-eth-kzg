#!/bin/bash

chmod a+x scripts/compile_c.sh

# First compile the c code to make sure we have the libraries
sh scripts/compile_c.sh

# Copy the static lib + headers into the golang bindings folder
cp -R ./bindings/c/build ./bindings/golang/c/build

# Build the golang code
cd bindings/golang && go build -x