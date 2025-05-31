use bls12_381::{G1Point, G2Point};
use kzg_multi_open::{commit_key::CommitKey, verification_key::VerificationKey};
use serde::Deserialize;
use serialization::trusted_setup::{deserialize_g1_points, deserialize_g2_points, SubgroupCheck};

use crate::constants::{FIELD_ELEMENTS_PER_BLOB, FIELD_ELEMENTS_PER_CELL};

const TRUSTED_SETUP_JSON: &str = include_str!("../data/trusted_setup_4096.json");

/// Represents an Ethereum trusted setup used for KZG commitments on the BLS12-381 curve.
#[derive(Deserialize, Debug, PartialEq, Eq)]
pub struct TrustedSetup {
    /// G1 Monomial represents a list of group elements in the G1 group on the bls12-381 curve.
    pub g1_monomial: Vec<G1Point>,
    /// G2 Monomial represents a list of group elements in the G2 group on the bls12-381 curve.
    pub g2_monomial: Vec<G2Point>,
}

/// Represents a serialized Ethereum trusted setup used for KZG commitments on the BLS12-381 curve.
///
/// This struct holds hex-encoded group elements in G1 and G2, provided in monomial and lagrange bases.
/// These elements are used to construct commitment and verification keys for polynomial commitment schemes.
///
/// The setup is typically loaded from a JSON file matching the format used in Ethereum consensus specifications.
#[derive(Deserialize, Debug, PartialEq, Eq)]
struct TrustedSetupJSON {
    /// G1 Monomial represents a list of uncompressed
    /// hex encoded group elements in the G1 group on the bls12-381 curve.
    ///
    /// Ethereum has multiple trusted setups, however the one being
    /// used currently contains 4096 G1 elements.
    pub g1_monomial: Vec<String>,
    /// G2 Monomial represents a list of uncompressed hex encoded
    /// group elements in the G2 group on the bls12-381 curve.
    ///
    /// The length of this vector is 65.
    pub g2_monomial: Vec<String>,
}

impl TrustedSetupJSON {
    /// Converts the `TrustedSetupJSON` into a `TrustedSetup` with subgroup checks.
    ///
    /// Panics if any of the points are not in the correct subgroup
    fn to_trusted_setup(&self) -> TrustedSetup {
        let g1_monomial = deserialize_g1_points(&self.g1_monomial, SubgroupCheck::Check);
        let g2_monomial = deserialize_g2_points(&self.g2_monomial, SubgroupCheck::Check);
        TrustedSetup {
            g1_monomial,
            g2_monomial,
        }
    }

    /// Converts the `TrustedSetupJSON` into a `TrustedSetup` without doing subgroup checks.
    ///
    /// Panics if:
    ///     - The hex string does not start with 0x
    ///     - The hex string does not represent a valid point in the G1/G2 group
    fn to_trusted_setup_unchecked(&self) -> TrustedSetup {
        let g1_monomial = deserialize_g1_points(&self.g1_monomial, SubgroupCheck::NoCheck);
        let g2_monomial = deserialize_g2_points(&self.g2_monomial, SubgroupCheck::NoCheck);
        TrustedSetup {
            g1_monomial,
            g2_monomial,
        }
    }

    /// Parse a JSON string in the format specified by the ethereum trusted setup.
    ///
    /// This method does not check that the points are in the correct subgroup.
    fn from_json_unchecked(json: &str) -> Self {
        // Note: it is fine to panic here since this method is called on startup
        // and we want to fail fast if the trusted setup is malformed.
        serde_json::from_str(json)
            .expect("could not parse json string into a TrustedSetup structure")
    }

    /// Loads the official trusted setup file being used on mainnet from the embedded data folder.
    fn from_embed() -> Self {
        Self::from_json_unchecked(TRUSTED_SETUP_JSON)
    }
}

impl Default for TrustedSetup {
    fn default() -> Self {
        let trusted_setup_json = TrustedSetupJSON::from_embed();
        // We have a test that checks the embedded trusted setup is well-formed.
        trusted_setup_json.to_trusted_setup_unchecked()
    }
}

impl From<&TrustedSetup> for CommitKey {
    fn from(setup: &TrustedSetup) -> Self {
        Self::new(setup.g1_monomial.clone())
    }
}

impl From<&TrustedSetup> for VerificationKey {
    fn from(setup: &TrustedSetup) -> Self {
        let g2_points = setup.g2_monomial.clone();
        let num_g2_points = g2_points.len();
        // The setup needs as many g1 elements for the verification key as g2 elements, in order
        // to commit to the remainder/interpolation polynomial.
        let g1_points = setup.g1_monomial[..num_g2_points].to_vec();

        Self::new(
            g1_points,
            g2_points,
            FIELD_ELEMENTS_PER_CELL,
            FIELD_ELEMENTS_PER_BLOB,
        )
    }
}

impl From<&TrustedSetup> for kzg_single_open::verifier::VerificationKey {
    fn from(setup: &TrustedSetup) -> Self {
        Self {
            gen_g1: setup.g1_monomial[0],
            gen_g2: setup.g2_monomial[0],
            tau_g2: setup.g2_monomial[1],
        }
    }
}

impl From<&TrustedSetup> for kzg_single_open::prover::CommitKey {
    fn from(setup: &TrustedSetup) -> Self {
        Self {
            g1s: setup.g1_monomial.clone(),
        }
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
    /// Note: That we do not need the g1_lagrange points so they are skipped.
    pub fn from_json(json: &str) -> Self {
        let trusted_setup_json = TrustedSetupJSON::from_json_unchecked(json);
        trusted_setup_json.to_trusted_setup()
    }

    /// Parse a Json string in the format specified by the ethereum trusted setup.
    ///
    /// This method does not check that the points are in the correct subgroup.
    pub fn from_json_unchecked(json: &str) -> Self {
        let trusted_setup = TrustedSetupJSON::from_json_unchecked(json);
        trusted_setup.to_trusted_setup_unchecked()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedded_setup_has_points_in_correct_subgroup() {
        let setup = TrustedSetupJSON::from_embed();
        setup.to_trusted_setup();
    }
}
