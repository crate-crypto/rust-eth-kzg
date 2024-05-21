# Changelog

## [0.2.2](https://github.com/crate-crypto/peerdas-kzg/compare/v0.2.1...v0.2.2) (2024-05-21)


### Features

* Add publishing code for other crates ([e0eadd9](https://github.com/crate-crypto/peerdas-kzg/commit/e0eadd95f05f6cc93f0b7b8efe78ea3f51ba0f11))
* **release-please:** Remove release-type and change component name ([422d7fb](https://github.com/crate-crypto/peerdas-kzg/commit/422d7fb64fb888af1d996cb4cc769a30fcffed84))


### Bug Fixes

* Add back cargo-workspace ([435d3b2](https://github.com/crate-crypto/peerdas-kzg/commit/435d3b2b0438bd317e1ef1687d8462fbc8529457))
* Add rlib to c bindings since it is being imported by nim rust crate ([b157a88](https://github.com/crate-crypto/peerdas-kzg/commit/b157a88c43f4aa523097184530e9e8efa1b379e2))
* PR requires pull-request-title-pattern ([fb3e327](https://github.com/crate-crypto/peerdas-kzg/commit/fb3e3271def98391598325ffc26ba61ef1cab82c))
* **release-please:** Remove cargo-workspace plugin ([015c005](https://github.com/crate-crypto/peerdas-kzg/commit/015c0054c079743385dd86494f0b7ee3942ba60f))
* Remove all configuration from the root package ([e3181fa](https://github.com/crate-crypto/peerdas-kzg/commit/e3181faada94e6e90a0f6f621127905eea397f24))
* Remove component from tag for all components ([1d130d5](https://github.com/crate-crypto/peerdas-kzg/commit/1d130d5c2ef3f18467f2166bcd6861451d173e4a))
* Root package must be set as simple or release-please will look for a Cargo.toml file and err ([d25f836](https://github.com/crate-crypto/peerdas-kzg/commit/d25f83661690ff949e6fecf11e054c28bf0790f2))
* Run PR title check on Pull requests only ([078cf18](https://github.com/crate-crypto/peerdas-kzg/commit/078cf18d5a7ae538ae1ec8f88abe85b024326520))
* Use "." as the root package ([11279ee](https://github.com/crate-crypto/peerdas-kzg/commit/11279eeecace869b59a16fcfa4439373c7b83644))

## [0.2.1](https://github.com/crate-crypto/peerdas-kzg/compare/v0.2.0...v0.2.1) (2024-05-21)


### Bug Fixes

* Comment in bls12_381 Cargo.toml ([abaa2cc](https://github.com/crate-crypto/peerdas-kzg/commit/abaa2ccd8aa5ede857c8a474da2a4489c943dd33))
* Explicitly pass the RELEASE-TOKEN when dispatching workflows ([e1e99b3](https://github.com/crate-crypto/peerdas-kzg/commit/e1e99b326d91d0ca2f6dd812511e14cc0f0a24f6))
* Formatting ([08aa75f](https://github.com/crate-crypto/peerdas-kzg/commit/08aa75fbebc33d75c634788ab6323695f42e3c9e))
* Move bls12_381 extra-file change into bls12_381 component ([14e9ce9](https://github.com/crate-crypto/peerdas-kzg/commit/14e9ce9a8e2d268bd5d3f6017ab09a650b9713fe))

## [0.2.0](https://github.com/crate-crypto/peerdas-kzg/compare/v0.1.0...v0.2.0) (2024-05-21)


### ⚠ BREAKING CHANGES

* rollback scope package naming for release-please
* update crate names in release-please manifest

### Miscellaneous Chores

* Rollback scope package naming for release-please ([9e7f047](https://github.com/crate-crypto/peerdas-kzg/commit/9e7f04724119ca97fd49cf992dad4b23d6da6387))
* Update crate names in release-please manifest ([c180947](https://github.com/crate-crypto/peerdas-kzg/commit/c18094731ba6e091b607faaac18b4e82f2f5b704))

## [0.1.0](https://github.com/crate-crypto/peerdas-kzg/compare/v0.0.1...v0.1.0) (2024-05-20)


### ⚠ BREAKING CHANGES

* add namespace to internal crates and kzg crate

### Miscellaneous Chores

* Add namespace to internal crates and kzg crate ([57ed80e](https://github.com/crate-crypto/peerdas-kzg/commit/57ed80e4122c56cfc1868afdd27cbb7f79bba88d))

## [0.0.1](https://github.com/crate-crypto/peerdas-kzg/compare/v0.0.1...v0.0.1) (2024-05-20)


### ⚠ BREAKING CHANGES

* remove trailing slash in package name (trigger release-please)
* Each rust crate is a separate component that is linked together with linked-version plugin
* reset global release type to rust and remove Cargo.toml from path (trigger-release-please)
* Add component name to bls12_381 package (trigger release-please)
* Add separate release-type for bls12_381 crate (trigger release-please)
* Make release type simple
* modify release-please to point to the c rust crate (trigger release-please)
* cargo format (trigger release-please)
* replace hythens in all crates with underscore
* replace bls12-381 with bls12_381.
* enforce PR title

### Miscellaneous Chores

* Add component name to bls12_381 package (trigger release-please) ([07dc500](https://github.com/crate-crypto/peerdas-kzg/commit/07dc500c22311d7b6843ec8790f98391d1097423))
* Add separate release-type for bls12_381 crate (trigger release-please) ([7d37ef1](https://github.com/crate-crypto/peerdas-kzg/commit/7d37ef16ae037d6630a0c2ed69973cd99821be9a))
* Cargo format (trigger release-please) ([3acecea](https://github.com/crate-crypto/peerdas-kzg/commit/3acecea41b5baec67376a191d1fdb91acfc9d7c4))
* Each rust crate is a separate component that is linked together with linked-version plugin ([39d7e50](https://github.com/crate-crypto/peerdas-kzg/commit/39d7e506ba2ea9e3aaea1f65f97f3518dbcbaf54))
* Enforce PR title ([c50e666](https://github.com/crate-crypto/peerdas-kzg/commit/c50e666ec8c408134b7d50d6caa6d2f616f9219f))
* Make release type simple ([73fc287](https://github.com/crate-crypto/peerdas-kzg/commit/73fc287b29c4cd17cb1a7ad5d84ee09f3b43a1eb))
* Modify release-please to point to the c rust crate (trigger release-please) ([0192964](https://github.com/crate-crypto/peerdas-kzg/commit/01929646394efd9285c268a1e0d0d98f29c83a91))
* Release 0.0.1 ([f551ec9](https://github.com/crate-crypto/peerdas-kzg/commit/f551ec9f7c045dfa06024ee223067d3cc05ec169))
* Remove trailing slash in package name (trigger release-please) ([5caa419](https://github.com/crate-crypto/peerdas-kzg/commit/5caa419d668b15a954cc183f65231ee8c5e01348))
* Replace bls12-381 with bls12_381. ([9299a37](https://github.com/crate-crypto/peerdas-kzg/commit/9299a37493317e0aabbe027de2771f11607ff418))
* Replace hythens in all crates with underscore ([66a63d8](https://github.com/crate-crypto/peerdas-kzg/commit/66a63d839ac475f79ae19c4cd340f9987f431b30))
* Reset global release type to rust and remove Cargo.toml from path (trigger-release-please) ([fd557f9](https://github.com/crate-crypto/peerdas-kzg/commit/fd557f908a7798e08034a172b4856c333c557a21))
