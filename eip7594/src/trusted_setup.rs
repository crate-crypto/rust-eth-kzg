use bls12_381::{G1Point, G2Point};
use kzg_multi_open::{commit_key::CommitKey, verification_key::VerificationKey};
use serde::Deserialize;

use crate::constants::{FIELD_ELEMENTS_PER_BLOB, FIELD_ELEMENTS_PER_CELL};

const TRUSTED_SETUP_JSON: &str = include_str!("../data/trusted_setup_4096.json");

#[derive(Deserialize, Debug, PartialEq, Eq)]
pub struct TrustedSetup {
    /// G1 Monomial represents a list of uncompressed
    /// hex encoded group elements in the G1 group on the bls12-381 curve.
    ///
    /// Ethereum has multiple trusted setups, however the one being
    /// used currently contains 4096 G1 elements.
    pub g1_monomial: Vec<String>,
    /// G1 Lagrange represents a list of uncompressed
    /// hex encoded group elements in the G1 group on the bls12-381 curve.
    ///
    /// These are related to `G1 Monomial` in that they are what one
    /// would get if we did an inverse FFT on the `G1 monomial` elements.
    ///
    /// The length of this vector is equal to the length of G1_Monomial.
    pub g1_lagrange: Vec<String>,
    /// G2 Monomial represents a list of uncompressed hex encoded
    /// group elements in the G2 group on the bls12-381 curve.
    ///
    /// The length of this vector is 65.
    pub g2_monomial: Vec<String>,
}

impl Default for TrustedSetup {
    fn default() -> Self {
        Self::from_embed()
    }
}

/// An enum used to specify whether to check that the points are in the correct subgroup
#[derive(Debug, Copy, Clone)]
enum SubgroupCheck {
    Check,
    NoCheck,
}

impl From<&TrustedSetup> for CommitKey {
    fn from(setup: &TrustedSetup) -> Self {
        setup.to_commit_key(SubgroupCheck::NoCheck)
    }
}

impl From<&TrustedSetup> for VerificationKey {
    fn from(setup: &TrustedSetup) -> Self {
        setup.to_verification_key(SubgroupCheck::NoCheck)
    }
}

impl TrustedSetup {
    /// Parse a Json string in the format specified by the ethereum trusted setup.
    ///
    /// The file that is being used on mainnet is located here: https://github.com/ethereum/consensus-specs/blob/389b2ddfb954731da7ccf4c0ef89fab2d4575b99/presets/mainnet/trusted_setups/trusted_setup_4096.json
    ///
    // The format that the file follows that this function also accepts, looks like the following:
    /*
    {
      "g1_monomial": [
        "0x97f1d3a73197d7942695638c4fa9ac0fc3688c4f9774b905a14e3a3f171bac586c55e83ff97a1aeffb3af00adb22c6bb",
        ...
      ],
      "g1_lagrange": [
        "0xa0413c0dcafec6dbc9f47d66785cf1e8c981044f7d13cfe3e4fcbb71b5408dfde6312493cb3c1d30516cb3ca88c03654",
        "0x8b997fb25730d661918371bb41f2a6e899cac23f04fc5365800b75433c0a953250e15e7a98fb5ca5cc56a8cd34c20c57",
        ...
      ],
      "g2_monomial": [
        "0x93e02b6052719f607dacd3a088274f65596bd0d09920b61ab5da61bbdc7f5049334cf11213945d57e5ac7d055d042b7e024aa2b2f08f0a91260805272dc51051c6e47ad4fa403b02b4510b647ae3d1770bac0326a805bbefd48056c8c121bdb8",
        ...
      ]
    }
    */
    pub fn from_json(json: &str) -> Self {
        let trusted_setup = Self::from_json_unchecked(json);
        trusted_setup.validate_trusted_setup();
        trusted_setup
    }
    /// Parse a Json string in the format specified by the ethereum trusted setup.
    ///
    /// This method does not check that the points are in the correct subgroup.
    pub fn from_json_unchecked(json: &str) -> Self {
        // Note: it is fine to panic here since this method is called on startup
        // and we want to fail fast if the trusted setup is malformed.
        serde_json::from_str(json)
            .expect("could not parse json string into a TrustedSetup structure")
    }

    /// This validates that the points in the trusted setup are in the correct subgroup.
    ///
    /// Panics if any of the points are not in the correct subgroup
    fn validate_trusted_setup(&self) {
        self.to_commit_key(SubgroupCheck::Check);
        self.to_verification_key(SubgroupCheck::Check);
    }

    fn to_commit_key(&self, subgroup_check: SubgroupCheck) -> CommitKey {
        let points = deserialize_g1_points(&self.g1_monomial, subgroup_check);
        CommitKey::new(points)
    }

    fn to_verification_key(&self, subgroup_check: SubgroupCheck) -> VerificationKey {
        let g2_points = deserialize_g2_points(&self.g2_monomial, subgroup_check);
        let num_g2_points = g2_points.len();
        // The setup needs as many g1 elements for the verification key as g2 elements, in order
        // to commit to the remainder/interpolation polynomial.
        let g1_points = deserialize_g1_points(&self.g1_monomial[..num_g2_points], subgroup_check);

        VerificationKey::new(
            g1_points,
            g2_points,
            FIELD_ELEMENTS_PER_CELL,
            FIELD_ELEMENTS_PER_BLOB,
        )
    }

    /// Loads the official trusted setup file being used on mainnet from the embedded data folder.
    fn from_embed() -> Self {
        Self::from_json_unchecked(TRUSTED_SETUP_JSON)
    }
}

/// Deserialize G1 points from hex strings without checking that the element
/// is in the correct subgroup.
fn deserialize_g1_points<T: AsRef<str>>(
    g1_points_hex_str: &[T],
    check: SubgroupCheck,
) -> Vec<G1Point> {
    let mut g1_points = Vec::new();
    for g1_hex_str in g1_points_hex_str {
        let g1_hex_str = g1_hex_str.as_ref();

        let g1_hex_str_without_0x = g1_hex_str
            .strip_prefix("0x")
            .expect("expected hex points to be prefixed with `0x`");
        let g1_point_bytes: [u8; 48] = hex::decode(g1_hex_str_without_0x)
            .expect("trusted setup has malformed g1 points")
            .try_into()
            .expect("expected 48 bytes for G1 point");

        let point = match check {
            SubgroupCheck::Check => {
                G1Point::from_compressed(&g1_point_bytes).expect("invalid g1 point")
            }
            SubgroupCheck::NoCheck => {
                G1Point::from_compressed_unchecked(&g1_point_bytes).expect("invalid g1 point")
            }
        };

        g1_points.push(point)
    }

    g1_points
}

/// Deserialize G2 points from hex strings without checking that the element
/// is in the correct subgroup.
fn deserialize_g2_points<T: AsRef<str>>(
    g2_points_hex_str: &[T],
    subgroup_check: SubgroupCheck,
) -> Vec<G2Point> {
    let mut g2_points = Vec::new();
    for g2_hex_str in g2_points_hex_str {
        let g2_hex_str = g2_hex_str.as_ref();
        let g2_hex_str_without_0x = g2_hex_str
            .strip_prefix("0x")
            .expect("expected hex points to be prefixed with `0x`");
        let g2_point_bytes: [u8; 96] = hex::decode(g2_hex_str_without_0x)
            .expect("trusted setup has malformed g2 points")
            .try_into()
            .expect("expected 96 bytes for G2 point");

        let point = match subgroup_check {
            SubgroupCheck::Check => G2Point::from_compressed(&g2_point_bytes).unwrap(),
            SubgroupCheck::NoCheck => G2Point::from_compressed_unchecked(&g2_point_bytes).unwrap(),
        };
        g2_points.push(point)
    }

    g2_points
}

#[test]
fn test_embedded_setup_has_points_in_correct_subgroup() {
    let setup = TrustedSetup::default();
    setup.validate_trusted_setup();
}
