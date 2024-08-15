# C

## Overview

This directory contains the bindings for Rust methods that use a C ABI. We do not publish any of its artifacts explicitly, instead it is used for other high level languages that need
to communicate with the rust code via a C ABI.

##  Building

You can view this as a regular Rust crate, so to build:

```
cargo build
```

> Note: Due to the crate-type in `Cargo.toml` being specified as `staticlib` and `cdylib`, building this crate will produce a static library and a dynamic library.

## Testing

To test:

```
cargo test
```
