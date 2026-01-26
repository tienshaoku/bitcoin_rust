use crate::U256;
use serde::{Deserialize, Serialize};
use sha256::digest;
use std::fmt;

#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq, Eq, Hash)]
pub struct Hash(U256);

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:x}", self.0)
    }
}

impl Hash {
    pub fn hash<T: serde::Serialize>(data: &T) -> Self {
        let mut serialized: Vec<u8> = vec![];
        if let Err(e) = ciborium::into_writer(data, &mut serialized) {
            panic!("Failed to serialize data: {:?}. This should not happen", e);
        }
        let hash = digest(&serialized);
        let hash_bytes = hex::decode(hash).unwrap();
        let hash_array: [u8; 32] = hash_bytes.as_slice().try_into().unwrap();
        let u64_array: [u64; 4] = [
            u64::from_be_bytes(hash_array[0..8].try_into().unwrap()),
            u64::from_be_bytes(hash_array[8..16].try_into().unwrap()),
            u64::from_be_bytes(hash_array[16..24].try_into().unwrap()),
            u64::from_be_bytes(hash_array[24..32].try_into().unwrap()),
        ];
        Hash(U256(u64_array))
    }

    pub fn matches_target(&self, target: U256) -> bool {
        self.0 <= target
    }

    pub fn as_bytes(&self) -> [u8; 32] {
        let bytes = self.0.to_little_endian();
        bytes.as_slice().try_into().unwrap()
    }

    pub fn zero() -> Self {
        Hash(U256::zero())
    }
}
