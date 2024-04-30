use commit_key::CommitKey;
use opening_key::OpeningKey;

pub mod commit_key;
pub mod lincomb;
pub mod opening_key;
// TODO: We can replace this with a file being embedded in the future.
// This is simply the trusted setup file from the ethereum ceremony
pub mod trusted_setup;

/// This is a placeholder method for creating the commit and opening keys for the ethereum
/// ceremony. This will be replaced with a method that reads the trusted setup file at a higher
/// level.
pub fn create_eth_commit_opening_keys() -> (CommitKey, OpeningKey) {
    let (g1s, g2s) = trusted_setup::from_eth_setup();
    let generator = g1s[0];

    let ck = CommitKey::new(g1s);
    let vk = OpeningKey::new(generator, g2s);
    (ck, vk)
}

#[cfg(test)]
mod tests {
    use crate::{opening_key::OpeningKey, trusted_setup::from_eth_setup};

    use super::CommitKey;

    #[test]
    fn eth_trusted_setup_deserializes() {
        // Just test that the trusted setup can be loaded/deserialized
        let (g1s, g2s) = from_eth_setup();
        let generator = g1s[0];

        let _ck = CommitKey::new(g1s);
        let _vk = OpeningKey::new(generator, g2s);
    }
}
