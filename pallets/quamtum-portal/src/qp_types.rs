use ethereum::{Account};
use sp_core::{H256, U256};
use sp_std::{collections::vec_deque::VecDeque, prelude::*, str};
use ethabi_nostd::Address;

#[derive(Debug, Default)]
pub struct QpTransaction {
	pub timestamp: u64,
	pub remote_contract: Address,
	pub source_msg_sender: Address,
	pub source_beneficiary: Address,
	pub token: Address,
	pub amount: U256,
	pub method: Vec<u8>,
	pub gas: u64
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