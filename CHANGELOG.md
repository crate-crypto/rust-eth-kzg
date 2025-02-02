# Changelog

## [0.5.3](https://github.com/crate-crypto/rust-eth-kzg/compare/v0.5.2...v0.5.3) (2025-02-02)


### Features

* Add batch normalization tests ([4a1920a](https://github.com/crate-crypto/rust-eth-kzg/commit/4a1920ac3b65e9cd1478d6695b1d7520bf0946bd))


### Bug Fixes

* Trigger build ([4330783](https://github.com/crate-crypto/rust-eth-kzg/commit/433078342e68db5bf113966dc57d944038707ad9))

## [0.5.2](https://github.com/crate-crypto/rust-eth-kzg/compare/v0.5.1...v0.5.2) (2024-09-24)


### Features

* Allow batch_inverse to take a scratchpad as input ([#268](https://github.com/crate-crypto/rust-eth-kzg/issues/268)) ([b47387b](https://github.com/crate-crypto/rust-eth-kzg/commit/b47387b45a851ddc75d9e5de8dbdd3cd95df8b8d))
* Pin Rust CI to 1.80.0 ([#270](https://github.com/crate-crypto/rust-eth-kzg/issues/270)) ([8a76c5c](https://github.com/crate-crypto/rust-eth-kzg/commit/8a76c5c3495db61734d852e0ac09d77045c467cd))
* Replace blst msm method with a Rust native method ([#273](https://github.com/crate-crypto/rust-eth-kzg/issues/273)) ([b4ef4af](https://github.com/crate-crypto/rust-eth-kzg/commit/b4ef4afa16ebba2b1e071eda1c6f70542d7b7989))

## [0.5.1](https://github.com/crate-crypto/rust-eth-kzg/compare/v0.5.0...v0.5.1) (2024-08-27)


### Bug Fixes

* Publish maybe_rayon when releasing crates ([#244](https://github.com/crate-crypto/rust-eth-kzg/issues/244)) ([ad9f273](https://github.com/crate-crypto/rust-eth-kzg/commit/ad9f273a26d32b29b5b6969632e8aaa99f0549b7))

## [0.5.0](https://github.com/crate-crypto/rust-eth-kzg/compare/v0.4.1...v0.5.0) (2024-08-27)


### ⚠ BREAKING CHANGES

* **java:** Rename `recoverCellsAndProof` -> `recoverCellsAndKZGProofs`  ([#232](https://github.com/crate-crypto/rust-eth-kzg/issues/232))
* Add a ThreadCount enum for specifying the number of threads to use ([#225](https://github.com/crate-crypto/rust-eth-kzg/issues/225))
* Allow downstream users to avoid the precomputations that are needed for the FixedBaseMSM algorithm in FK20 ([#224](https://github.com/crate-crypto/rust-eth-kzg/issues/224))
* **rust:** Rename `recover_cells_and_proofs` -> `recover_cells_and_kzg_proofs` ([#219](https://github.com/crate-crypto/rust-eth-kzg/issues/219))
* Move `recover_evaluations_in_domain_order` into cosets.rs ([#175](https://github.com/crate-crypto/rust-eth-kzg/issues/175))

### Features

* Add a ThreadCount enum for specifying the number of threads to use ([#225](https://github.com/crate-crypto/rust-eth-kzg/issues/225)) ([488ae20](https://github.com/crate-crypto/rust-eth-kzg/commit/488ae2008c1171506bd7ded8f50070960877d72d))
* Allow bindings to specify the number of threads and whether precomputation is wanted ([#228](https://github.com/crate-crypto/rust-eth-kzg/issues/228)) ([ee42bf8](https://github.com/crate-crypto/rust-eth-kzg/commit/ee42bf8da72e6378f53176bd54d33ec9c9963d5a))
* Allow downstream users to avoid the precomputations that are needed for the FixedBaseMSM algorithm in FK20 ([#224](https://github.com/crate-crypto/rust-eth-kzg/issues/224)) ([5367e79](https://github.com/crate-crypto/rust-eth-kzg/commit/5367e791be156bbb0cb57cf6ce204008c17957fd))


### Bug Fixes

* Namespace C methods ([#148](https://github.com/crate-crypto/rust-eth-kzg/issues/148)) ([e175ccd](https://github.com/crate-crypto/rust-eth-kzg/commit/e175ccd26f0276cbd9b5a7c0b8626cd0328bc654))
* Trigger maybe_rayon release-please config ([cef8ed3](https://github.com/crate-crypto/rust-eth-kzg/commit/cef8ed310317d9305148913aa674e03d8d0593b8))


### Miscellaneous Chores

* **java:** Rename `recoverCellsAndProof` -&gt; `recoverCellsAndKZGProofs`  ([#232](https://github.com/crate-crypto/rust-eth-kzg/issues/232)) ([7157789](https://github.com/crate-crypto/rust-eth-kzg/commit/7157789f08f636f18f9d02226344fb6c08bb4628))
* Move `recover_evaluations_in_domain_order` into cosets.rs ([#175](https://github.com/crate-crypto/rust-eth-kzg/issues/175)) ([5e6ff4f](https://github.com/crate-crypto/rust-eth-kzg/commit/5e6ff4f8a1f49508c3c272b8182bc4b9cefae234))
* **rust:** Rename `recover_cells_and_proofs` -&gt; `recover_cells_and_kzg_proofs` ([#219](https://github.com/crate-crypto/rust-eth-kzg/issues/219)) ([cc06254](https://github.com/crate-crypto/rust-eth-kzg/commit/cc0625459ab6028a4bbc718f8ab86cfe11f12992))

## [0.4.1](https://github.com/crate-crypto/rust-eth-kzg/compare/v0.4.0...v0.4.1) (2024-08-13)


### Bug Fixes

* Put workflow dispatch and push events in a separate group ([#139](https://github.com/crate-crypto/rust-eth-kzg/issues/139)) ([f5f9909](https://github.com/crate-crypto/rust-eth-kzg/commit/f5f9909fc8a0dc20dbeb3c1bb23732bdaa45366f))

## [0.4.0](https://github.com/crate-crypto/rust-eth-kzg/compare/v0.3.0...v0.4.0) (2024-08-05)


### ⚠ BREAKING CHANGES

* rename java project's usage of `peerdas-kzg` -> `eth-kzg` ([#104](https://github.com/crate-crypto/rust-eth-kzg/issues/104))
* update package name for node bindings ([#99](https://github.com/crate-crypto/rust-eth-kzg/issues/99))
* Rename `rust` packages to rust-eth-kzg ([#89](https://github.com/crate-crypto/rust-eth-kzg/issues/89))
* refactor eip7594 API ([#91](https://github.com/crate-crypto/rust-eth-kzg/issues/91))
* unify the error type in eip7594 package ([#90](https://github.com/crate-crypto/rust-eth-kzg/issues/90))
* Move all prover and verifier methods to PeerDAS context object ([#53](https://github.com/crate-crypto/rust-eth-kzg/issues/53))
* Remove recoverAllCells and computeCells ([#46](https://github.com/crate-crypto/rust-eth-kzg/issues/46))

### Features

* FK20 now only computes proofs ([#52](https://github.com/crate-crypto/rust-eth-kzg/issues/52)) ([e66472e](https://github.com/crate-crypto/rust-eth-kzg/commit/e66472ebe585b9fc19b3df041ceface3d433fb87))
* Rename `rust` packages to rust-eth-kzg ([#89](https://github.com/crate-crypto/rust-eth-kzg/issues/89)) ([8d09ad7](https://github.com/crate-crypto/rust-eth-kzg/commit/8d09ad73147fb12300bb53a1d69e9538d58ba5cd))
* VerifyCellKZGProofBatch now takes duplicated commitments ([#113](https://github.com/crate-crypto/rust-eth-kzg/issues/113)) ([5023fc2](https://github.com/crate-crypto/rust-eth-kzg/commit/5023fc2afdc252a573bf49d42307c8afac898833))


### Bug Fixes

* Add build.gradle to release-please config ([da9479b](https://github.com/crate-crypto/rust-eth-kzg/commit/da9479bd483980de56add7304b843d88273bc3e6))
* Add node package.json version to release-please ([ac33e76](https://github.com/crate-crypto/rust-eth-kzg/commit/ac33e76657d26c48e359a6516b4b3f6767099f92))
* BYTES_PER_CELL constant ([3e8455d](https://github.com/crate-crypto/rust-eth-kzg/commit/3e8455d7046309e474f85a69d3b78e41dec89c7b))
* Do not use the deduplicated commitments ([fb6df2e](https://github.com/crate-crypto/rust-eth-kzg/commit/fb6df2eefc29a3f1041aaaa3205c01de1218c02f))
* Erasure codes ([#87](https://github.com/crate-crypto/rust-eth-kzg/issues/87)) ([3279585](https://github.com/crate-crypto/rust-eth-kzg/commit/3279585c49df36c645649c156319113f5b933e0a))
* Interpret call to size_of method as bytes and not as num_elements ([#136](https://github.com/crate-crypto/rust-eth-kzg/issues/136)) ([d4dde8c](https://github.com/crate-crypto/rust-eth-kzg/commit/d4dde8c2a9d9718c3f9285a093968a83c7593f28))
* Node CI workflow runs on master ([5702205](https://github.com/crate-crypto/rust-eth-kzg/commit/5702205bc0a1a709483dd6abacc356ccc7dcdf94))
* Pack readme in csharp project ([5ce0470](https://github.com/crate-crypto/rust-eth-kzg/commit/5ce0470b002dbec37d8f3205f56f4a7aed40da55))
* Package name when publishing ([#137](https://github.com/crate-crypto/rust-eth-kzg/issues/137)) ([c1d4dc4](https://github.com/crate-crypto/rust-eth-kzg/commit/c1d4dc4c1503cb19e4eb6d2b2370a6925dca88ed))
* Recovery is done with respects to the cells not the blob ([0594fee](https://github.com/crate-crypto/rust-eth-kzg/commit/0594feee102aade0d9e224e199a6fc9c620fddb8))
* Small nits ([#82](https://github.com/crate-crypto/rust-eth-kzg/issues/82)) ([d4ef145](https://github.com/crate-crypto/rust-eth-kzg/commit/d4ef145c5cd57b5fa54bb0871493ab8dc18cb038))
* Update csbindgen to 1.19.3 ([#127](https://github.com/crate-crypto/rust-eth-kzg/issues/127)) ([f557acf](https://github.com/crate-crypto/rust-eth-kzg/commit/f557acf3a58f9d2a2a47f1ace8efb8192aec5d52))


### Miscellaneous Chores

* Move all prover and verifier methods to PeerDAS context object ([#53](https://github.com/crate-crypto/rust-eth-kzg/issues/53)) ([0e70f01](https://github.com/crate-crypto/rust-eth-kzg/commit/0e70f0186c30d950319caa043d4f038eb1f5929f))
* Refactor eip7594 API ([#91](https://github.com/crate-crypto/rust-eth-kzg/issues/91)) ([59cf8f3](https://github.com/crate-crypto/rust-eth-kzg/commit/59cf8f3377764b19c66d4b7aefee7e637561b17f))
* Remove recoverAllCells and computeCells ([#46](https://github.com/crate-crypto/rust-eth-kzg/issues/46)) ([f398eec](https://github.com/crate-crypto/rust-eth-kzg/commit/f398eec7f8c1743fa4a967ad6091e70094954d1c))
* Rename java project's usage of `peerdas-kzg` -&gt; `eth-kzg` ([#104](https://github.com/crate-crypto/rust-eth-kzg/issues/104)) ([e9df67c](https://github.com/crate-crypto/rust-eth-kzg/commit/e9df67cf6c7bbb78d94792eb29fc294bf26c71f0))
* Unify the error type in eip7594 package ([#90](https://github.com/crate-crypto/rust-eth-kzg/issues/90)) ([b7891c2](https://github.com/crate-crypto/rust-eth-kzg/commit/b7891c29ab032ba586e4cd8716b36dd248a2ac47))
* Update package name for node bindings ([#99](https://github.com/crate-crypto/rust-eth-kzg/issues/99)) ([bbcd97a](https://github.com/crate-crypto/rust-eth-kzg/commit/bbcd97a03b3657a93495ebc6e74beb2228716cf2))

## [0.3.0](https://github.com/crate-crypto/peerdas-kzg/compare/v0.2.6...v0.3.0) (2024-05-21)


### Bug Fixes

* Change bootstrap commit ([3f59841](https://github.com/crate-crypto/peerdas-kzg/commit/3f598415a8d27cc73fa3d12c9d15b2e1a1afdd25))
* Change file name to releases to not conflict with the yml file ([4f549e9](https://github.com/crate-crypto/peerdas-kzg/commit/4f549e94c27d58c1c28f47aeef1ab3e1e54f81e8))
* Remove changelog ([9bb717a](https://github.com/crate-crypto/peerdas-kzg/commit/9bb717ae4b05227544b968c92b1092fee63dd72b))
* Remove file structure and add prefix to workflows ([0e5f747](https://github.com/crate-crypto/peerdas-kzg/commit/0e5f747f8b4137dd7b47c2525ee6eb97bebdb23c))
* Remove run prefix ([13d0a6c](https://github.com/crate-crypto/peerdas-kzg/commit/13d0a6c9d412f3848a4d6fdd843b1030eed82f78))


### Miscellaneous Chores

* Release 0.0.1 ([d81e9b1](https://github.com/crate-crypto/peerdas-kzg/commit/d81e9b1e8dcdc7a9f1909db9ee48ed212ee65229))
* Release 0.3.0 ([a2fcb9a](https://github.com/crate-crypto/peerdas-kzg/commit/a2fcb9afd65dc81b90c50a4062bc2023a53e6b56))
