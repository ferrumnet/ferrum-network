#![cfg_attr(not(feature = "std"), no_std)]

use ethereum::{Account};
use sp_core::{U256};
use sp_std::{collections::vec_deque::VecDeque, prelude::*, str};

pub struct QuantumPortalTransaction {
	timestamp: u64,
	remote_contract: Account,
	source_msg_sender: Account,
	source_beneficiary: Account,
	token: Account,
	amount: U256,
	method: Vec<u8>,
	gas: u64
}
