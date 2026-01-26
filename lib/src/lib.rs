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
