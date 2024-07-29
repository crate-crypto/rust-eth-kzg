# KZG Multi Open

## Overview

This crate provides a Rust API for the [FK20](https://github.com/khovratovich/Kate/blob/master/Kate_amortized.pdf) polynomial commitment scheme. FK20 allows you to commit to a polynomial over some field with prime characteristics, and later on reveal multiple evaluations of that polynomial, along with an (opening) proof that attests to the correctness of those evaluations.  

The API is opinionated and although it is generic, it also does not support every use case. It has been made with the Ethereum Data Availability Sampling vision in mind. One can see that for example, we allow evaluations over particular cosets, where the order of the elements in each coset and the order of the cosets themselves are fixed. (Even though we test internally with permutations of the cosets)

## Installation

Installation of this crate can be done by adding this to your `Cargo.toml`:

```toml
[dependencies]
crate_crypto_kzg_multi_open_fk20 = "0.1.0"
```
