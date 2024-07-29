# Rust Eth KZG

## Overview

### What

A cryptographic library that is compatible with the KZG commitment scheme used in the Ethereum blockchain for PeerDAS.

### Why

The cryptography implemented in this library is the prerequisite needed for Ethereum's version of Data Availability Sampling(DAS). The library has been implemented in a modular way, so one can also use the underlying polynomial commitment scheme, for a different purpose.

## Building the source

This library is written in Rust and offers bindings to C, C#, node.js, golang, Java and Nim. These bindings can be found in the `bindings` folder. The bindings expose an API that is compatible with the API needed for Ethereum.

If you only intend to modify the cryptography, then a Rust compiler will be needed. For the bindings, one should check the respective language's README file to find out additional requirements.

### Building everything

To build everything including the artifacts needed for the bindings, you can run:

```
./scripts/compile.sh
```

To only build the native Rust code, you can run:

```
cargo build
```

## Benchmarks

Benchmarks can be run by calling:

```
cargo bench
```

> Note: This will benchmark the underlying Rust library. It will not account for (if any) discrepancies due to
calling the library via a particular language.
An example of this is the CGO overhead when calling a foreign language from Golang; in our case, this overhead is negligible compared to the actual computations being performed.

## License

Licensed and distributed under either of

MIT license: LICENSE-MIT or <http://opensource.org/licenses/MIT>

or

Apache License, Version 2.0, (LICENSE-APACHEv2 or <http://www.apache.org/licenses/LICENSE-2.0>)
at your option. These files may not be copied, modified, or distributed except according to those terms.
