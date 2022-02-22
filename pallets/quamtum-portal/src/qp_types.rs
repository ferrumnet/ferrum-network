use ethereum::{Account};
use sp_core::{H256, U256};
use sp_std::{collections::vec_deque::VecDeque, prelude::*, str};

pub struct QpTransaction {
	pub timestamp: u64,
	pub remote_contract: Account,
	pub source_msg_sender: Account,
	pub source_beneficiary: Account,
	pub token: Account,
	pub amount: U256,
	pub method: Vec<u8>,
	pub gas: u64
}

pub struct QpLocalBlock {
	pub chain_id: u64,
	pub nonce: u64,
	pub timestamp: u64,
}

pub struct QpRemoteBlock {
	pub block_hash: H256,
	pub miner: Account,
	pub stake: U256,
	pub total_value: U256,
	pub block_metadata: QpLocalBlock,
}

pub struct QpContext {
}
