use serde::{Deserialize, Serialize};
use uint::construct_uint;

construct_uint! {
    // 4 * 64-bit words
    #[derive(Serialize, Deserialize)]
    pub struct U256(4);
}

// impl Serialize for U256 {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: serde::Serializer,
//     {
//         let bytes = self.to_big_endian();
//         bytes.serialize(serializer)
//     }
// }

// initial reward in bitcoin - multiply by 10^8 to get satoshis
pub const INITIAL_REWARD: u64 = 50;
pub const HALVING_INTERVAL: u64 = 210;
// in second
pub const IDEAL_BLOCK_TIME: u64 = 10;
pub const MIN_TARGET: U256 = U256([
    0xFFFF_FFFF_FFFF_FFFF,
    0xFFFF_FFFF_FFFF_FFFF,
    0xFFFF_FFFF_FFFF_FFFF,
    0x0000_FFFF_FFFF_FFFF,
]);
// in blocks
pub const DIFFICULTY_UPDATE_INTERVAL: u64 = 50;
pub mod crypto;
pub mod error;
pub mod sha256;
pub mod types;
pub mod util;

#[cfg(test)]
mod tests {
    use super::*;
    use ciborium;

    #[test]
    fn test_u256_serialize() {
        let u256 = U256::from(0x123456789ABCDEF0u128);
        println!("Serializing U256: {:?}", u256);
        println!("U256 as hex: {:x}", u256);

        let bytes = u256.to_big_endian();
        println!("Big-endian bytes: {:?}", bytes);
        println!("Big-endian bytes as hex: {}", hex::encode(&bytes));

        let mut cbor_bytes = Vec::new();
        ciborium::into_writer(&u256, &mut cbor_bytes).unwrap();

        println!("CBOR bytes: {:?}", cbor_bytes);
        println!("CBOR bytes as hex: {}", hex::encode(&cbor_bytes));
    }
}
