# Node

## Overview

This directory contains the bindings for the node npm project. NAPI-RS is being used to build the rust project
and generate the relevant node bindings.

## Building

To build the project:

```
yarn build
```

## Testing

Tests are written in typescript, ie there are no Rust native tests in this directory. Hence to test:

```
yarn test
```

## Publishing

Some notes are added on how we are deploying since its noteworthy:

- NAPI-RS will build platform specific binary and publish npm packages for those platform specific
packages. See `npm` for the platforms that are supported.

- Users need to only import the wrapper package `package.json` and it will choose the relevant platform specific
package depending on the users platform.
