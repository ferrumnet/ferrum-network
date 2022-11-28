use ethabi_nostd::Address;
use parity_scale_codec::{Decode, Encode};
use serde::{Deserialize, Serialize};
use sp_core::{H256, U256};
use sp_std::{prelude::*, str};

#[derive(Debug, Default)]
pub struct QpTransaction {
	pub timestamp: u64,
	pub remote_contract: Address,
	pub source_msg_sender: Address,
	pub source_beneficiary: Address,
	pub token: Address,
	pub amount: U256,
	pub method: Vec<u8>,
	pub gas: u64,
}

#[derive(Debug)]
pub struct QpLocalBlock {
	pub chain_id: u64,
	pub nonce: u64,
	pub timestamp: u64,
}

pub struct QpRemoteBlock {
	pub block_hash: H256,
	pub miner: Address,
	pub stake: U256,
	pub total_value: U256,
	pub block_metadata: QpLocalBlock,
}

#[derive(
	Clone, Eq, PartialEq, Decode, Encode, Debug, Serialize, Deserialize, scale_info::TypeInfo,
)]
pub struct QpConfig {
	pub network_vec: Vec<QpNetworkItem>,
	pub pair_vec: Vec<(u64, u64)>,
}

#[derive(
	Clone, Eq, PartialEq, Decode, Encode, Debug, Serialize, Deserialize, scale_info::TypeInfo,
)]
pub struct QpNetworkItem {
	// #[serde(with = "serde_bytes")]
	pub url: Vec<u8>,
	// #[serde(with = "serde_bytes")]
	pub ledger_manager: Vec<u8>,
	pub id: u64,
}

impl Default for QpConfig {
	fn default() -> QpConfig {
		QpConfig { network_vec: vec![], pair_vec: vec![] }
	}
}
