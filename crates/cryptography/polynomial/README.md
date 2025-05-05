# Polynomial

## Overview

This crate provides a Rust API for operations on a polynomial/vector of bls122-381 scalars. The API is tailored towards providing the necessary
functionality for the KZG multi-opening protocol that is present in the workspace that this crate is situated, so no guarantees are made regarding general purpose.

## Installation

It is not advised to install this crate as part of an independent project. It is only published to crates.io so
that we can publish the multi-opening protocol to crates.io. Nevertheless, installation of this crate can be done by adding this to your `Cargo.toml`:

```toml
[dependencies]
ekzg-polynomial = "0.1.0"
```
