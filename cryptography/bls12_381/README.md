# BLS12-381

## Overview

This crate provides a Rust API for the bls12-381 elliptic curve. The API is tailored towards providing the necessary
functionality for the KZG multi-opening protocol that is present in the workspace that this crate is situated, so no guarantees are made regarding general purpose.

## Installation

It is not advised to install this crate as part of an independent project. It is only published to crates.io so
that we can publish the multi-opening protocol to crates.io. Nevertheless, installation of this crate can be done by adding this to your `Cargo.toml`:

```toml
[dependencies]
crate_crypto_internal_eth_kzg_bls12_381 = "0.1.0"
```
