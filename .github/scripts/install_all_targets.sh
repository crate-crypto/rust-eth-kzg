#!/bin/bash

# This assumes we are on mac, for the CI we will be running this in Mac
brew install SergioBenitez/osxct/x86_64-unknown-linux-gnu
brew tap messense/homebrew-macos-cross-toolchains
brew install aarch64-unknown-linux-gnu

rustup target add x86_64-unknown-linux-gnu
rustup target add aarch64-unknown-linux-gnu
rustup target add x86_64-pc-windows-gnu
rustup target add x86_64-apple-darwin
rustup target add aarch64-apple-darwin