#!/bin/bash

chmod a+x scripts/compile_all_targets_c.sh

# First compile the c code to make sure we have the libraries
sh scripts/compile_all_targets_c.sh

# Copy the static lib + headers into the c sharp bindings folder
cp -R ./bindings/c/build ./bindings/csharp/runtimes