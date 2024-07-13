# Erasure Codes

## Overview

This crate provides a Rust API for Erasure codes. It uses Reed solomon encoding, however the decoding algorithm and the API in general is tailored the particular use case of Data Availability sampling in the Ethereum Blockchain. It is not a general purpose crate for unique decoding.

## Installation

It is not advised to install this crate as part of an independent project. It is only published to crates.io so
that we can publish the eip7594 crate to crates.io. Nevertheless, installation of this crate can be done by adding this to your `Cargo.toml`:

```toml
[dependencies]
crate_crypto_internal_peerdas_erasure_codes = "0.1.0"
```
