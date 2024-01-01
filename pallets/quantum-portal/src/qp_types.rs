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
use ethabi_nostd::{Address, Token};
use parity_scale_codec::{Decode, Encode};
use serde::{Deserialize, Serialize};
use sp_core::{H256, U256};
use sp_std::{prelude::*, str};
use scale_info::prelude::string::String;
use parity_scale_codec::alloc::string::ToString;

// Limit on how many pairs to mine,
// The current limit is 6, means mining both ways on 3 seperate chains
pub const MAX_PAIRS_TO_MINE: usize = 6;

/// EVM ChainId for mainnet
const QPN_EVM_MAINNET_CHAIN_ID: u64 = 26000;

/// EVM ChainId for testnet
const QPN_EVM_TESTNET_CHAIN_ID: u64 = 26100;

#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct QpTransaction {
	pub timestamp: u64,
	pub remote_contract: Address,
	pub source_msg_sender: Address,
	pub source_beneficiary: Address,
	pub token: Address,
	pub amount: U256,
	pub method: Vec<u8>,
	pub gas: U256,
	pub fixed_fee: U256,
}

#[derive(Debug)]
pub struct QpLocalBlock {
	pub chain_id: u64,
	pub nonce: u64,
	pub timestamp: u64,
}

impl QpLocalBlock {
	// generate a hash from the data in the local block
	pub fn hash(&self) -> H256 {
		let data_to_hash: Vec<Token> = vec![
			Token::Uint(U256::from(self.chain_id)),
			Token::Uint(U256::from(self.chain_id)),
			Token::Uint(U256::from(self.chain_id)),
		];

		crate::chain_utils::ChainUtils::keccack(&ethabi_nostd::encode(&data_to_hash))
	}
}

pub struct QpRemoteBlock {
	pub block_hash: H256,
	pub miner: Address,
	pub stake: U256,
	pub total_value: U256,
	pub block_metadata: QpLocalBlock,
}

#[derive(
	Clone,
	Eq,
	PartialEq,
	Decode,
	Encode,
	Debug,
	Serialize,
	Deserialize,
	scale_info::TypeInfo,
	Default,
)]
pub struct QpConfig {
	pub network_vec: Vec<QpNetworkItem>,
	pub pair_vec: Vec<(u64, u64)>,
	pub signer_public_key: Vec<u8>,
	pub role: Role,
}

impl QpConfig {
	pub fn validate(&self) -> Result<(), String> {
		// ensure the source or destinagion is QPN
		for (src, dest) in &self.pair_vec {
			if src != &QPN_EVM_MAINNET_CHAIN_ID && dest != &QPN_EVM_MAINNET_CHAIN_ID {
				if src != &QPN_EVM_TESTNET_CHAIN_ID && dest != &QPN_EVM_TESTNET_CHAIN_ID {
					return Err("Invalid Config : One of the pairs need to be QPN EVM!".to_string())
				}
			}
		}

		Ok(())
	}
}

#[derive(
	Clone, Eq, PartialEq, Decode, Encode, Debug, Serialize, Deserialize, scale_info::TypeInfo,
)]
pub struct QpNetworkItem {
	// #[serde(with = "serde_bytes")]
	pub url: Vec<u8>,
	// #[serde(with = "serde_bytes")]
	pub gateway_contract_address: Vec<u8>,
	pub id: u64,
}

#[allow(non_camel_case_types)]
#[derive(
	Clone,
	Eq,
	PartialEq,
	Decode,
	Encode,
	Debug,
	Serialize,
	Deserialize,
	scale_info::TypeInfo,
	Default,
)]
pub enum Role {
	#[default]
	None,
	QP_MINER,
	QP_FINALIZER,
}

impl From<&[u8]> for Role {
	fn from(v: &[u8]) -> Self {
		match v {
			b"QP_MINER" => Self::QP_MINER,
			b"QP_FINALIZER" => Self::QP_FINALIZER,
			_ => Self::None,
		}
	}
}
