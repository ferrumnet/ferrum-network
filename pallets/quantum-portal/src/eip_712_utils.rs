use crate::chain_utils::ChainUtils;
use ethabi_nostd::{encoder, Address, Token, H256, U256}; //vec::{Vec};
use libsecp256k1;
use numtoa::NumToA;
use parity_scale_codec::Encode;
use sp_core::ecdsa;
use sp_io::crypto;
use sp_std::{prelude::*, str};
use tiny_keccak::{Hasher, Keccak};

pub struct EIP712Utils;

impl EIP712Utils {
    /// Generate a EIP712Domain encoded hex for the given inputs
    pub fn generate_eip_712_domain_seperator_hash(
        contract_name: &[u8],
        contract_version: &[u8],
        chain_id: u64,
        contract_address: &[u8],
    ) -> H256 {
        let type_hash = ChainUtils::keccack(
            b"EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)",
        );
        let hashed_name = ChainUtils::keccack(contract_name);
        let hashed_version = ChainUtils::keccack(contract_version);

        let encoded_domain_seperator = Self::get_encoded_hash(vec![
            Token::FixedBytes(Vec::from(type_hash.as_bytes())),
            Token::FixedBytes(Vec::from(hashed_name.as_bytes())),
            Token::FixedBytes(Vec::from(hashed_version.as_bytes())),
            Token::Uint(U256::from(chain_id)),
            Token::Address(ChainUtils::hex_to_address(contract_address)),
        ]);

        encoded_domain_seperator
    }

    /// This function takes the domain_seperator_hash and eip_args_hash as input and returns the EIP712 format hash
    pub fn generate_eip_712_hash(domain_seperator_hash: &[u8], eip_args_hash: &[u8]) -> H256 {
        let prefix = (b"\x19\x01").to_vec();
        let concat = [&prefix[..], &domain_seperator_hash[..], &eip_args_hash[..]].concat();
        ChainUtils::keccack(&concat)
    }

    /// This function takes a vector of Token inputs and returns the encoded keccak hash
    pub fn get_encoded_hash(inputs: Vec<Token>) -> H256 {
        let encoded = encoder::encode(&inputs);
        ChainUtils::keccack(&encoded)
    }
}
