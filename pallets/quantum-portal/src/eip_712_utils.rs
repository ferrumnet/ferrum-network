// Copyright 2019-2023 Ferrum Inc.
// This file is part of Ferrum.

// Ferrum is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Ferrum is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Ferrum.  If not, see <http://www.gnu.org/licenses/>.
use crate::chain_utils::ChainUtils;
use ethabi_nostd::Address;
use ethabi_nostd::{encoder, Token, H256, U256}; //vec::{Vec};
use sp_std::prelude::*;

pub struct EIP712Utils;

impl EIP712Utils {
    /// Generate a EIP712Domain encoded hex for the given inputs
    pub fn generate_eip_712_domain_seperator_hash(
        contract_name: &[u8],
        contract_version: &[u8],
        chain_id: u64,
        contract_address: Address,
    ) -> H256 {
        let type_hash = ChainUtils::keccack(
            b"EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)",
        );
        let hashed_name = ChainUtils::keccack(contract_name);
        let hashed_version = ChainUtils::keccack(contract_version);

        Self::get_encoded_hash(vec![
            Token::FixedBytes(Vec::from(type_hash.as_bytes())),
            Token::FixedBytes(Vec::from(hashed_name.as_bytes())),
            Token::FixedBytes(Vec::from(hashed_version.as_bytes())),
            Token::Uint(U256::from(chain_id)),
            Token::Address(contract_address),
        ])
    }

    /// This function takes the domain_seperator_hash and eip_args_hash as input and returns the EIP712 format hash
    pub fn generate_eip_712_hash(domain_seperator_hash: &[u8], eip_args_hash: &[u8]) -> H256 {
        let prefix = (b"\x19\x01").to_vec();
        let concat = [&prefix[..], domain_seperator_hash, eip_args_hash].concat();
        ChainUtils::keccack(&concat)
    }

    /// This function takes a vector of Token inputs and returns the encoded keccak hash
    pub fn get_encoded_hash(inputs: Vec<Token>) -> H256 {
        let encoded = encoder::encode(&inputs);
        ChainUtils::keccack(&encoded)
    }
}
