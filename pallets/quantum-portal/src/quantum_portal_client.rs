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
#![cfg_attr(not(feature = "std"), no_std)]
use crate::{
	chain_queries::CallResponse,
	chain_utils::{ChainRequestError, ChainRequestResult, ChainUtils, TransactionCreationError},
	contract_client::{ContractClient, ContractClientSignature},
	eip_712_utils::EIP712Utils,
	qp_types::{QpLocalBlock, QpRemoteBlock, QpTransaction},
	Config,
};
use ethabi_nostd::{decoder::decode, ParamKind, Token};
use frame_system::offchain::{
	AppCrypto, CreateSignedTransaction, SendSignedTransaction, SendUnsignedTransaction,
	SignedPayload, Signer, SigningTypes, SubmitTransaction,
};
use sp_core::{H256, U256};
use sp_std::{marker::PhantomData, prelude::*};

#[allow(dead_code)]
const DUMMY_HASH: H256 = H256::zero();
const ZERO_HASH: H256 = H256::zero();

pub struct QuantumPortalClient<T: Config> {
	pub contract: ContractClient,
	pub signer: ContractClientSignature,
	pub now: u64,
	pub block_number: u64,
	_phantom: PhantomData<T>,
}

fn local_block_tuple0() -> Vec<ParamKind> {
	vec![ParamKind::Uint(256), ParamKind::Uint(256), ParamKind::Uint(256)]
}

fn local_block_tuple() -> ParamKind {
	ParamKind::Tuple(vec![
		Box::new(ParamKind::Uint(256)),
		Box::new(ParamKind::Uint(256)),
		Box::new(ParamKind::Uint(256)),
	])
}

fn decode_remote_block_and_txs<T, F>(
	data: &[u8],
	mined_block_tuple: ParamKind,
	block_tuple_decoder: F,
) -> ChainRequestResult<(T, Vec<QpTransaction>)>
where
	F: Fn(Token) -> ChainRequestResult<T>,
{
	log::info!("decode_remote_block_and_txs {:?}", data);
	// let dec = decode(
	//     &[
	//         ParamKind::Tuple(vec![
	//             Box::new(mined_block_tuple),
	//             Box::new(ParamKind::Array(
	//                 Box::new(ParamKind::Tuple(vec![         // RemoteTransaction[]
	//                                                         Box::new(ParamKind::Uint(256)),
	// // timestamp
	// Box::new(ParamKind::Address),       // remoteContract
	// Box::new(ParamKind::Address),       // sourceMsgSender
	// Box::new(ParamKind::Address),       // sourceBeneficiary
	// Box::new(ParamKind::Address),       // token
	// Box::new(ParamKind::Uint(256)),     // amount
	// Box::new(ParamKind::Bytes),         // method
	// Box::new(ParamKind::Uint(256)),     // gas                 ]))))
	//         ])],
	//     ChainUtils::hex_to_bytes(&data)?.as_slice(),
	// ).unwrap();
	let dec = decode(
		&[
			mined_block_tuple,
			ParamKind::Array(Box::new(ParamKind::Tuple(vec![
				// RemoteTransaction[]
				Box::new(ParamKind::Uint(256)),                         // timestamp
				Box::new(ParamKind::Address),                           // remoteContract
				Box::new(ParamKind::Address),                           // sourceMsgSender
				Box::new(ParamKind::Address),                           // sourceBeneficiary
				Box::new(ParamKind::Address),                           // token
				Box::new(ParamKind::Uint(256)),                         // amount
				Box::new(ParamKind::Array(Box::new(ParamKind::Bytes))), // method
				Box::new(ParamKind::Uint(256)),                         // gas
				Box::new(ParamKind::Uint(256)),                         // fixedFee
			]))),
		],
		ChainUtils::hex_to_bytes(data)?.as_slice(),
	)
	.unwrap();
	log::info!("decoded {:?}, - {}", dec, dec.as_slice().len());
	let dec: ChainRequestResult<Vec<Token>> = match dec.as_slice() {
		[tuple, txs] => Ok(vec![tuple.clone(), txs.clone()]),
		_ => Err(b"Unexpected output. Could not decode local block at first level"
			.as_slice()
			.into()),
	};
	let dec = dec?;
	log::info!("decoded = 2 | {:?}, - {}", dec, dec.as_slice().len());
	match dec.as_slice() {
		[mined_block, remote_transactions] => {
			let mined_block = mined_block.clone();
			let remote_transactions = remote_transactions.clone();
			log::info!("PRE = Mined block is opened up");
			let block = block_tuple_decoder(mined_block)?;
			log::info!("Mined block is opened up == {:?}", remote_transactions);
			let remote_transactions = remote_transactions
				.to_array()
				.unwrap()
				.into_iter()
				.map(|t| {
					decode_remote_transaction_from_tuple(t.to_tuple().unwrap().as_slice()).unwrap()
				})
				.collect();
			Ok((block, remote_transactions))
		},
		_ => Err(b"Unexpected output. Could not decode local block".as_slice().into()),
	}
}

fn decode_remote_transaction_from_tuple(dec: &[Token]) -> ChainRequestResult<QpTransaction> {
	match dec {
		[timestamp, remote_contract, source_msg_sender, source_beneficiary, token, amount, method, gas, fixed_fee] =>
		{
			let timestamp = timestamp.clone().to_uint().unwrap().as_u64();
			let remote_contract = remote_contract.clone().to_address().unwrap();
			let source_msg_sender = source_msg_sender.clone().to_address().unwrap();
			let source_beneficiary = source_beneficiary.clone().to_address().unwrap();
			let token = token.clone().to_address().unwrap();
			let amount = amount.clone().to_uint().unwrap();
			let fixed_fee = fixed_fee.clone().to_uint().unwrap();
			let method = method.clone().to_bytes().unwrap();
			let gas = gas.clone().to_uint().unwrap().as_u64();
			Ok(QpTransaction {
				timestamp,
				remote_contract,
				source_msg_sender,
				source_beneficiary,
				token,
				amount,
				method,
				gas: gas.into(),
				fixed_fee,
			})
		},
		_ => Err(b"Unexpected output. Could not decode remote transaction".as_slice().into()),
	}
}

impl<T: Config> QuantumPortalClient<T> {
	pub fn new(
		contract: ContractClient,
		signer: ContractClientSignature,
		now: u64,
		block_number: u64,
	) -> Self {
		QuantumPortalClient { contract, signer, now, block_number, _phantom: Default::default() }
	}

	pub fn is_local_block_ready(&self, chain_id: u64) -> ChainRequestResult<bool> {
		let signature = b"isLocalBlockReady(uint64)";
		let res: Box<CallResponse> =
			self.contract.call(signature, &[Token::Uint(U256::from(chain_id))], None)?;
		let val = ChainUtils::hex_to_u256(&res.result)?;
		Ok(!val.is_zero())
	}

	pub fn last_remote_mined_block(&self, chain_id: u64) -> ChainRequestResult<QpLocalBlock> {
		let signature = b"lastRemoteMinedBlock(uint64)";
		let res: Box<CallResponse> =
			self.contract.call(signature, &[Token::Uint(U256::from(chain_id))], None)?;
		self.decode_local_block(res.result.as_slice())
	}

	pub fn last_finalized_block(&self, chain_id: u64) -> ChainRequestResult<QpLocalBlock> {
		let signature = b"getLastFinalizedBlock(uint256)";
		let state_contract_address = self.contract.get_state_contract_address()?;
		let res: Box<CallResponse> = self.contract.call(
			signature,
			&[Token::Uint(U256::from(chain_id))],
			Some(state_contract_address),
		)?;
		self.decode_local_block(res.result.as_slice())
	}

	pub fn last_local_block(&self, chain_id: u64) -> ChainRequestResult<QpLocalBlock> {
		let signature = b"getLastLocalBlock(uint256)";
		let state_contract_address = self.contract.get_state_contract_address()?;
		let res: Box<CallResponse> = self.contract.call(
			signature,
			&[Token::Uint(U256::from(chain_id))],
			Some(state_contract_address),
		)?;
		self.decode_local_block(res.result.as_slice())
	}

	pub fn local_block_by_nonce(
		&self,
		chain_id: u64,
		last_block_nonce: u64,
	) -> ChainRequestResult<(QpLocalBlock, Vec<QpTransaction>)> {
		let signature = b"localBlockByNonce(uint64,uint64)";
		let res: Box<CallResponse> = self.contract.call(
			signature,
			&[Token::Uint(U256::from(chain_id)), Token::Uint(U256::from(last_block_nonce))],
			None,
		)?;
		decode_remote_block_and_txs(res.result.as_slice(), local_block_tuple(), |block| {
			log::info!("1-DECODING BLOCK {:?}", block);
			let block = block.to_tuple();
			let block = block.unwrap();
			log::info!("2-DECODING BLOCK {:?}", block);
			Self::decode_local_block_from_tuple(block.as_slice())
		})
	}

	pub fn mined_block_by_nonce(
		&self,
		chain_id: u64,
		last_block_nonce: u64,
	) -> ChainRequestResult<(QpRemoteBlock, Vec<QpTransaction>)> {
		let signature = b"minedBlockByNonce(uint64,uint64)";
		let res: Box<CallResponse> = self.contract.call(
			signature,
			&[Token::Uint(U256::from(chain_id)), Token::Uint(U256::from(last_block_nonce))],
			None,
		)?;
		let mined_block_tuple = ParamKind::Tuple(vec![
			// MinedBlock
			Box::new(ParamKind::FixedBytes(32)), // blockHash
			Box::new(ParamKind::Address),        // miner
			Box::new(ParamKind::Uint(256)),      // invalid block
			Box::new(ParamKind::Uint(256)),      // stake
			Box::new(ParamKind::Uint(256)),      // totalValue
			Box::new(local_block_tuple()),
		]);
		// let mined_block_tuple = vec![             // MinedBlock
		//                                    ParamKind::FixedBytes(32),    // blockHash
		//                                    ParamKind::Address,           // miner
		//                                    ParamKind::Uint(256),         // stake
		//                                    ParamKind::Uint(256),         // totalValue
		//                                    local_block_tuple()
		// ];
		decode_remote_block_and_txs(res.result.as_slice(), mined_block_tuple, |block| {
			log::info!("Decoding local block, {:?}", block);
			Self::decode_mined_block_from_tuple(block.to_tuple().unwrap().as_slice())
		})
	}

	pub fn create_finalize_transaction(
		&self,
		remote_chain_id: u64,
		block_nonce: u64,
		_finalizer_hash: H256,
		_finalizers: &[Vec<u8>],
		verification_result: bool,
	) -> ChainRequestResult<H256> {
		// because of sp_std, so here are the alternatives:
		// - Manually construct the function call as [u8].
		// function finalize(
		// 	uint256 remoteChainId,
		// 	uint256 blockNonce,
		// 	bytes32 finalizersHash,
		// 	address[] memory finalizers
		// ) ...
		// The last item is a bit complicated, but for now we pass an empty array.
		// Support buytes and dynamic arrays in future
		let finalizer_list: Vec<Token> = vec![];

		let (block_details, _) = self.mined_block_by_nonce(remote_chain_id, block_nonce)?;

		let method_signature =
            b"finalizeSingleSigner(uint256,uint256,uint256[],bytes32,address[],bytes32,uint64,bytes)";

		let salt = Token::FixedBytes(block_details.block_hash.as_ref().to_vec());
		let finalizer_hash = Token::FixedBytes(block_details.block_hash.as_ref().to_vec());

		let current_timestamp = block_details.block_metadata.timestamp;
		// expirt 1hr from now
		let expiry_buffer = core::time::Duration::from_secs(3600u64);
		let expiry_time = current_timestamp.saturating_add(expiry_buffer.as_secs());
		let expiry = Token::Uint(U256::from(expiry_time));

		let multi_sig = self.generate_multi_signature(
			remote_chain_id,
			block_nonce,
			finalizer_hash.clone(),
			finalizer_list.clone(),
			salt.clone(),
			expiry.clone(),
		)?;

		let signer = Signer::<T, T::AuthorityId>::all_accounts();
		if !signer.can_sign() {
			return Err(ChainRequestError::ErrorGettingJsonRpcResponse)
		}

		let results = signer.send_signed_transaction(|_account| {
			// Received price is wrapped into a call to `submit_price` public function of this
			// pallet. This means that the transaction, when executed, will simply call that
			// function passing `price` as an argument.
			crate::Call::submit_signature {
				chain_id: remote_chain_id,
				block_number: block_nonce,
				signature: multi_sig.clone(),
			}
		});

		Ok(Default::default())
	}

	pub fn post_finalize_transaction(
		&self,
		remote_chain_id: u64,
		block_nonce: u64,
		_finalizer_hash: H256,
		_finalizers: &[Vec<u8>],
		verification_result: bool,
	) -> ChainRequestResult<H256> {
		// because of sp_std, so here are the alternatives:
		// - Manually construct the function call as [u8].
		// function finalize(
		// 	uint256 remoteChainId,
		// 	uint256 blockNonce,
		// 	bytes32 finalizersHash,
		// 	address[] memory finalizers
		// ) ...
		// The last item is a bit complicated, but for now we pass an empty array.
		// Support buytes and dynamic arrays in future
		let finalizer_list: Vec<Token> = vec![];

		let (block_details, _) = self.mined_block_by_nonce(remote_chain_id, block_nonce)?;

		let method_signature =
            b"finalizeSingleSigner(uint256,uint256,uint256[],bytes32,address[],bytes32,uint64,bytes)";

		let salt = Token::FixedBytes(block_details.block_hash.as_ref().to_vec());
		let finalizer_hash = Token::FixedBytes(block_details.block_hash.as_ref().to_vec());

		let current_timestamp = block_details.block_metadata.timestamp;
		// expirt 1hr from now
		let expiry_buffer = core::time::Duration::from_secs(3600u64);
		let expiry_time = current_timestamp.saturating_add(expiry_buffer.as_secs());
		let expiry = Token::Uint(U256::from(expiry_time));

		let multi_sigs = PendingFinalizeSignatures::<T>::get(remote_chain_id, block_nonce)
			.expect("Should contain signatures");

		// Compute multisig format
		// This computation makes it match the implementation we have in qp smart contracts repo
		// refer https://github.com/ferrumnet/quantum-portal-smart-contracts/blob/326341cdfcb55052437393228f1d58e014c90f7b/test/common/Eip712Utils.ts#L93
		let mut multi_sigs_combined: Vec<u8> = Default::default();
		for sig in multi_sigs {
			multi_sigs_combined.extend(sig);
		}
		let mut multisig_compressed: Vec<u8> = multi_sigs_combined.0[0..64].to_vec();
		multisig_compressed.extend([28u8]);
		multisig_compressed.extend([0u8; 31]);

		log::info!(
			"Extended signature of size {}: {}",
			multisig_compressed.len(),
			sp_std::str::from_utf8(
				ChainUtils::bytes_to_hex(multisig_compressed.as_slice()).as_slice()
			)
			.unwrap()
		);

		log::info!(
			"Encoded Multisig generated : {:?}",
			sp_std::str::from_utf8(
				ChainUtils::bytes_to_hex(multisig_compressed.as_slice()).as_slice()
			)
			.unwrap()
		);

		// set this block nonce as invalid if verification failed
		let invalid_block: Vec<Token> =
			if !verification_result { vec![Token::Uint(U256::from(block_nonce))] } else { vec![] };

		let inputs = [
			Token::Uint(U256::from(remote_chain_id)),
			Token::Uint(U256::from(block_nonce)),
			Token::Array(invalid_block),
			finalizer_hash,
			Token::Array(finalizer_list),
			salt,
			expiry,
			Token::Bytes(multi_sig),
		];

		let recipient_address = self.contract.get_ledger_manager_address()?;

		let res = self.contract.send(
			method_signature,
			&inputs,
			None, //Some(U256::from(1000000 as u64)), // None,
			None, //Some(U256::from(10000000000 as u64)), // None,
			U256::zero(),
			None,
			self.signer.from,
			&self.signer,
			recipient_address,
		)?;

		Ok(Default::default())
	}

	pub fn post_finalizer_transaction(
		&self,
		remote_chain_id: u64,
		block_nonce: u64,
		_finalizer_hash: H256,
		_finalizers: &[Vec<u8>],
		verification_result: bool,
	) -> ChainRequestResult<H256> {
		// because of sp_std, so here are the alternatives:
		// - Manually construct the function call as [u8].
		// function finalize(
		// 	uint256 remoteChainId,
		// 	uint256 blockNonce,
		// 	bytes32 finalizersHash,
		// 	address[] memory finalizers
		// ) ...
		// The last item is a bit complicated, but for now we pass an empty array.
		// Support buytes and dynamic arrays in future
		let finalizer_list: Vec<Token> = vec![];

		let (block_details, _) = self.mined_block_by_nonce(remote_chain_id, block_nonce)?;

		let method_signature =
            b"finalizeSingleSigner(uint256,uint256,uint256[],bytes32,address[],bytes32,uint64,bytes)";

		let salt = Token::FixedBytes(block_details.block_hash.as_ref().to_vec());
		let finalizer_hash = Token::FixedBytes(block_details.block_hash.as_ref().to_vec());

		let current_timestamp = block_details.block_metadata.timestamp;
		// expirt 1hr from now
		let expiry_buffer = core::time::Duration::from_secs(3600u64);
		let expiry_time = current_timestamp.saturating_add(expiry_buffer.as_secs());
		let expiry = Token::Uint(U256::from(expiry_time));

		let multi_sig = self.generate_multi_signature(
			remote_chain_id,
			block_nonce,
			finalizer_hash.clone(),
			finalizer_list.clone(),
			salt.clone(),
			expiry.clone(),
		)?;

		// let signer = Signer::<T, T::AuthorityId>::all_accounts();
		// if !signer.can_sign() {
		// 	return Err(
		// 		ChainRequestError::ErrorGettingJsonRpcResponse
		// 	)
		// }

		// let results = signer.send_signed_transaction(|_account| {
		// 	// Received price is wrapped into a call to `submit_price` public function of this
		// 	// pallet. This means that the transaction, when executed, will simply call that
		// 	// function passing `price` as an argument.
		// 	crate::Call::submit_signature { chain_id : remote_chain_id, block_number: block_nonce,
		// signature: multi_sig.clone() } });

		log::info!(
			"Encoded Multisig generated : {:?}",
			sp_std::str::from_utf8(ChainUtils::bytes_to_hex(multi_sig.as_slice()).as_slice())
				.unwrap()
		);

		// set this block nonce as invalid if verification failed
		let invalid_block: Vec<Token> =
			if !verification_result { vec![Token::Uint(U256::from(block_nonce))] } else { vec![] };

		let inputs = [
			Token::Uint(U256::from(remote_chain_id)),
			Token::Uint(U256::from(block_nonce)),
			Token::Array(invalid_block),
			finalizer_hash,
			Token::Array(finalizer_list),
			salt,
			expiry,
			Token::Bytes(multi_sig),
		];

		let recipient_address = self.contract.get_ledger_manager_address()?;

		let res = self.contract.send(
			method_signature,
			&inputs,
			None, //Some(U256::from(1000000 as u64)), // None,
			None, //Some(U256::from(10000000000 as u64)), // None,
			U256::zero(),
			None,
			self.signer.from,
			&self.signer,
			recipient_address,
		)?;

		Ok(Default::default())
	}

	/// Returns the multiSignature to sign finalize transactions
	/// The function will
	/// 1. Generate the domain seperator values, encoded and hashed
	/// 2. Generate the message hash from the args of the finalize call and encoded it to the
	///    signature
	/// 3. Generate the eip_712 type hash for the ValidateAuthoritySignature function
	pub fn generate_multi_signature(
		&self,
		remote_chain_id: u64,
		block_nonce: u64,
		finalizer_hash: Token,
		finalizer_list: Vec<Token>,
		salt: Token,
		expiry: Token,
	) -> Result<Vec<u8>, TransactionCreationError> {
		let (verifying_contract_address, verifying_contract_version, verifying_contract_name) =
			&self
				.contract
				.get_authority_manager_address()
				.map_err(|_| TransactionCreationError::CannotFindContractAddress)?;
		// Generate the domain seperator hash, the hash is generated from the given arguments
		let domain_seperator_hash = EIP712Utils::generate_eip_712_domain_seperator_hash(
			verifying_contract_name,     // ContractName
			verifying_contract_version,  // ContractVersion
			self.contract.chain_id,      // ChainId
			*verifying_contract_address, // VerifyingAddress
		);
		log::info!("domain_seperator_hash {:?}", domain_seperator_hash);

		// Generate the finalize method sigature to encode the finalize call
		let finalize_method_signature = b"Finalize(uint256 remoteChainId,uint256 blockNonce,bytes32 finalizersHash,address[] finalizers,bytes32 salt,uint64 expiry)";
		let finalize_method_signature_hash = ChainUtils::keccack(finalize_method_signature);
		log::info!("finalize_method_signature_hash {:?}", finalize_method_signature_hash);

		log::info!("remote_chain_id {:?}", remote_chain_id);
		log::info!("block_nonde {:?}", block_nonce);
		log::info!("finalizer_hash {:?}", finalizer_hash);
		log::info!("finalizer_list {:?}", finalizer_list);
		log::info!("salt {:?}", salt);
		log::info!("expiry {:?}", expiry);

		// encode the finalize call to the expected format
		let encoded_message_hash = EIP712Utils::get_encoded_hash(vec![
			Token::FixedBytes(Vec::from(finalize_method_signature_hash.as_bytes())), /* finalize
			                                                                          * method signature
			                                                                          * hash */
			Token::Uint(U256::from(remote_chain_id)), // remote chain id
			Token::Uint(U256::from(block_nonce)),     // block nonce
			finalizer_hash,                           // finalizers hash
			Token::Array(finalizer_list),             // finalizers
			salt.clone(),                             // salt
			expiry.clone(),                           // expiry
		]);
		log::info!("encoded_message_hash {:?}", encoded_message_hash);

		// Generate the ValidateAuthoritySignature method signature to encode the eip_args
		let method_signature = b"ValidateAuthoritySignature(uint256 action,bytes32 msgHash,bytes32 salt,uint64 expiry)";
		let method_hash = ChainUtils::keccack(method_signature);
		log::info!("method_hash {:?}", method_hash);

		// Generate the encoded eip message
		let eip_args_hash = EIP712Utils::get_encoded_hash(vec![
			Token::FixedBytes(Vec::from(method_hash.as_bytes())), // method hash
			Token::Uint(U256::from(1)),                           // action
			Token::FixedBytes(Vec::from(encoded_message_hash.as_bytes())), // msgHash
			salt,                                                 // salt
			expiry,                                               // expiry
		]);
		log::info!("eip_args_hash {:?}", eip_args_hash);

		let eip_712_hash =
			EIP712Utils::generate_eip_712_hash(&domain_seperator_hash[..], &eip_args_hash[..]);
		log::info!("EIP712 Hash {:?}", eip_712_hash);

		// Sign the eip message, we only consider a single signer here since we only expect a single
		// key in the keystore
		let multi_sig_bytes = self.signer.signer(&eip_712_hash)?;

		// Compute multisig format
		// This computation makes it match the implementation we have in qp smart contracts repo
		// refer https://github.com/ferrumnet/quantum-portal-smart-contracts/blob/326341cdfcb55052437393228f1d58e014c90f7b/test/common/Eip712Utils.ts#L93
		let mut multisig_compressed: Vec<u8> = multi_sig_bytes.0[0..64].to_vec();
		multisig_compressed.extend([28u8]);
		multisig_compressed.extend([0u8; 31]);

		log::info!(
			"Extended signature of size {}: {}",
			multisig_compressed.len(),
			sp_std::str::from_utf8(
				ChainUtils::bytes_to_hex(multisig_compressed.as_slice()).as_slice()
			)
			.unwrap()
		);

		Ok(multisig_compressed)
	}

	#[allow(clippy::ptr_arg)]
	pub fn create_mine_transaction(
		&self,
		remote_chain_id: u64,
		block_nonce: u64,
		txs: &Vec<QpTransaction>,
		source_block: QpLocalBlock,
	) -> ChainRequestResult<H256> {
		let method_signature = b"mineRemoteBlock(uint64,uint64,(uint64,address,address,address,address,uint256,bytes,uint256,uint256)[],bytes32,uint64,bytes)";

		// set timestamp 1hr from now
		let current_timestamp = source_block.timestamp;
		let expiry_buffer = core::time::Duration::from_secs(360000u64);
		let expiry_time = current_timestamp.saturating_add(expiry_buffer.as_secs());
		let expiry = Token::Uint(U256::from(expiry_time));
		let salt = Token::FixedBytes(vec![0u8, 0u8]);

		let tx_vec: Vec<Token> = txs
			.iter()
			.map(|t| {
				Token::Tuple(vec![
					Token::Uint(U256::from(t.timestamp)),
					Token::Address(t.remote_contract),
					Token::Address(t.source_msg_sender),
					Token::Address(t.source_beneficiary),
					Token::Address(t.token),
					Token::Uint(t.amount),
					Token::Array(vec![Token::Bytes(t.method.clone())]),
					Token::Uint(t.gas),
					Token::Uint(t.fixed_fee),
				])
			})
			.collect();

		let multi_sig = self.generate_miner_signature(
			remote_chain_id,
			block_nonce,
			tx_vec.clone(),
			salt.clone(),
			expiry.clone(),
		)?;

		log::info!(
			"Encoded Miner Signature generated : {:?}",
			sp_std::str::from_utf8(ChainUtils::bytes_to_hex(multi_sig.as_slice()).as_slice())
				.unwrap()
		);

		let recipient_address = self.contract.get_ledger_manager_address()?;

		let res = self.contract.send(
			method_signature,
			&[
				Token::Uint(U256::from(remote_chain_id)),
				Token::Uint(U256::from(block_nonce)),
				Token::Array(tx_vec),
				salt,
				expiry,
				Token::Bytes(multi_sig),
			],
			Some(U256::from(1000000_u32)), // None,
			None,                          // Some(U256::from(60000000000 as u64)), // None,
			U256::zero(),
			None,
			self.signer.from,
			&self.signer,
			recipient_address,
		)?;
		Ok(res)
	}

	/// Returns the Signature to sign mine transactions
	/// The function will
	/// 1. Generate the domain seperator values, encoded and hashed
	/// 2. Generate the message hash from the (chainId, nonce, txs) and encoded it to the signature
	/// 3. Generate the eip_712 type hash for the MinerSignature function
	pub fn generate_miner_signature(
		&self,
		remote_chain_id: u64,
		block_nonce: u64,
		txs: Vec<Token>,
		salt: Token,
		expiry: Token,
	) -> Result<Vec<u8>, TransactionCreationError> {
		let (verifying_contract_address, verifying_contract_version, verifying_contract_name) =
			&self
				.contract
				.get_miner_manager_address()
				.map_err(|_| TransactionCreationError::CannotFindContractAddress)?;

		// Generate the domain seperator hash, the hash is generated from the given arguments
		let domain_seperator_hash = EIP712Utils::generate_eip_712_domain_seperator_hash(
			verifying_contract_name,     // ContractName
			verifying_contract_version,  // ContractVersion
			self.contract.chain_id,      // ChainId
			*verifying_contract_address, // VerifyingAddress
		);
		log::info!("domain_seperator_hash {:?}", domain_seperator_hash);

		// Generate the finalize method sigature to encode the finalize call
		let miner_method_signature = b"MinerSignature(bytes32 msgHash,uint64 expiry,bytes32 salt)";
		let miner_method_signature_hash = ChainUtils::keccack(miner_method_signature);
		log::info!("miner_method_signature_hash {:?}", miner_method_signature_hash);

		log::info!("remote_chain_id {:?}", remote_chain_id);
		log::info!("block_nonce {:?}", block_nonce);
		log::info!("txs {:?}", txs);
		log::info!("salt {:?}", salt);
		log::info!("expiry {:?}", expiry);

		// encode the finalize call to the expected format
		let encoded_message_hash = EIP712Utils::get_encoded_hash(vec![
			Token::Uint(U256::from(remote_chain_id)), // remote chain id
			Token::Uint(U256::from(block_nonce)),     // block nonce
			Token::Array(txs),                        // transactions
		]);
		log::info!("encoded_message_hash {:?}", encoded_message_hash);

		// Generate the encoded eip message
		let eip_args_hash = EIP712Utils::get_encoded_hash(vec![
			Token::FixedBytes(Vec::from(miner_method_signature_hash.as_bytes())), // method hash
			Token::FixedBytes(Vec::from(encoded_message_hash.as_bytes())),        // msgHash
			expiry,                                                               // expiry
			salt,                                                                 // salt
		]);
		log::info!("eip_args_hash {:?}", eip_args_hash);

		let eip_712_hash =
			EIP712Utils::generate_eip_712_hash(&domain_seperator_hash[..], &eip_args_hash[..]);
		log::info!("EIP712 Hash {:?}", eip_712_hash);

		// Sign the eip message, we only consider a single signer here since we only expect a single
		// key in the keystore
		let multi_sig_bytes = self.signer.signer(&eip_712_hash)?;

		// Compute multisig format
		// This computation makes it match the implementation we have in qp smart contracts repo
		// refer https://github.com/ferrumnet/quantum-portal-smart-contracts/blob/326341cdfcb55052437393228f1d58e014c90f7b/test/common/Eip712Utils.ts#L93
		let mut multisig_compressed: Vec<u8> = multi_sig_bytes.0[0..64].to_vec();
		multisig_compressed.extend([28u8]);
		multisig_compressed.extend([0u8; 31]);

		log::info!(
			"Extended signature of size {}: {}",
			multisig_compressed.len(),
			sp_std::str::from_utf8(
				ChainUtils::bytes_to_hex(multisig_compressed.as_slice()).as_slice()
			)
			.unwrap()
		);

		Ok(multisig_compressed)
	}

	pub fn finalize(&self, chain_id: u64) -> ChainRequestResult<Option<H256>> {
		log::info!("finalize({})", chain_id);
		let block = self.last_remote_mined_block(chain_id)?;
		log::info!("finalize-last_remote_mined_block({:?})", &block);
		let last_fin = self.last_finalized_block(chain_id)?;

		log::info!("finalize-last_finalized_block({:?})", &last_fin);
		if block.nonce > last_fin.nonce {
			log::info!(
				"Preparing to finalize, verifying mined block ({}, {})",
				chain_id,
				block.nonce
			);
			let (_mined_block, mined_txs) = self.mined_block_by_nonce(chain_id, block.nonce)?;
			let (_source_block, source_txs) = self.local_block_by_nonce(chain_id, block.nonce)?;
			// verify data before finalization
			let verification_result = Self::compare_and_verify_mined_block(&source_txs, &mined_txs);

			// if we have enough signers for finalize then we post transaction onchain
			let multi_sigs = PendingFinalizeSignatures::<T>::get(chain_id, block.nonce);
			let threshold = FinalizerThreshold::<T>::get(chain_id);

			if multi_sigs.len() > threshold {
				log::info!("Calling mgr.post_transaction({}, {})", chain_id, block.nonce);
				Ok(Some(self.post_finalize_transaction(
					chain_id,
					block.nonce,
					H256::zero(),
					&[self.signer.get_signer_address()],
					verification_result,
				)?))
			} else {
				// we dont have threshold so try to sign and post
				log::info!("Calling mgr.finalize({}, {})", chain_id, block.nonce);
				Ok(Some(self.create_finalize_transaction(
					chain_id,
					block.nonce,
					H256::zero(),
					&[self.signer.get_signer_address()],
					verification_result,
				)?))
			}
		} else {
			log::info!("Nothing to finalize for ({})", chain_id);
			Ok(None)
		}
	}

	pub fn mine(&self, remote_client: &QuantumPortalClient<T>) -> ChainRequestResult<Option<H256>> {
		let local_chain = self.contract.chain_id;
		let remote_chain = remote_client.contract.chain_id;
		log::info!("mine({} => {})", remote_chain, local_chain);
		let block_ready = remote_client.is_local_block_ready(local_chain)?;
		log::info!("local block ready? {}", block_ready);
		if !block_ready {
			return Ok(None)
		}
		log::info!("Getting last local block");
		let last_block = remote_client.last_local_block(local_chain)?;
		log::info!("Last local block is {:?}", last_block);
		let last_mined_block = self.last_remote_mined_block(remote_chain)?;
		log::info!("Local block f remote (chain {}) nonce is {}. Remote mined block on local (chain {}) is {}",
			remote_chain, last_block.nonce, local_chain, last_mined_block.nonce);
		if last_mined_block.nonce >= last_block.nonce {
			log::info!("Nothing to mine!");
			return Ok(None)
		}
		log::info!("Last block is on chain1 for target {} is {}", local_chain, last_block.nonce);
		let mined_block = self.mined_block_by_nonce(remote_chain, last_block.nonce)?;
		let already_mined = !mined_block.0.block_hash.eq(&ZERO_HASH);
		if already_mined {
			return Err(ChainRequestError::RemoteBlockAlreadyMined)
		}
		log::info!("Getting source block?");
		let source_block = remote_client
			.local_block_by_nonce(local_chain, last_mined_block.nonce.saturating_add(1))?;
		let default_qp_transaction = QpTransaction::default();
		log::info!(
			"Source block is GOT\n{:?}\n{:?}",
			source_block.0,
			if !source_block.1.is_empty() {
				source_block.1.get(0).unwrap()
			} else {
				&default_qp_transaction
			}
		);
		let txs = source_block.1;

		log::info!(
			"Checking if the slot to mine block on chain is assigned to us {}:{}",
			source_block.0.nonce,
			remote_chain,
		);

		let _assigned_miner = self.contract.get_miner_for_block(
			source_block.0.hash(),                      // block hash from txs
			source_block.0.timestamp,                   // block timestamp
			sp_io::offchain::timestamp().unix_millis(), // chain timestamp
		)?;

		// if ChainUtils::address_to_hex(assigned_miner) != self.signer.get_signer_address() {
		//     log::info!(
		//         "Not our slot to mine, Assigned miner is {:?} our address is {:?}",
		//         ChainUtils::address_to_hex(assigned_miner),
		//         self.signer.get_signer_address()
		//     );

		//     return Err(ChainRequestError::SlotNotAvailable);
		// }

		log::info!("About to mine block {}:{}", remote_chain, source_block.0.nonce);
		Ok(Some(self.create_mine_transaction(
			remote_chain,
			source_block.0.nonce,
			&txs,
			source_block.0,
		)?))
	}

	fn decode_local_block(&self, data: &[u8]) -> ChainRequestResult<QpLocalBlock> {
		let dec = decode(
			// &[local_block_tuple()],
			local_block_tuple0().as_slice(),
			ChainUtils::hex_to_bytes(data)?.as_slice(),
		)
		.unwrap();
		Self::decode_local_block_from_tuple(dec.as_slice())
	}

	fn decode_local_block_from_tuple(dec: &[Token]) -> ChainRequestResult<QpLocalBlock> {
		log::info!("Decoding local block, {:?}", dec);
		match dec {
			[chain_id, nonce, timestamp] => {
				let chain_id = chain_id.clone().to_uint();
				let nonce = nonce.clone().to_uint();
				let timestamp = timestamp.clone().to_uint();
				Ok(QpLocalBlock {
					chain_id: chain_id.unwrap().as_u64(),
					nonce: nonce.unwrap().as_u64(),
					timestamp: timestamp.unwrap().as_u64(),
				})
			},
			_ => Err(b"Unexpected output. Could not decode local block".as_slice().into()),
		}
	}

	fn decode_mined_block_from_tuple(dec: &[Token]) -> ChainRequestResult<QpRemoteBlock> {
		log::info!("decode_mined_block_from_tuple {:?}", dec);
		match dec {
			[block_hash, miner, stake, total_value, block_metadata] => {
				log::info!(
					"D {:?}::{:?}:{:?}:{:?}::{:?}",
					block_hash,
					miner,
					stake,
					total_value,
					block_metadata
				);
				let block_hash = block_hash.clone();
				let miner = miner.clone();
				let stake = stake.clone();
				let total_value = total_value.clone();
				let block_metadata = block_metadata.clone();
				log::info!("Decoding block metadata");
				let block_metadata =
					Self::decode_local_block_from_tuple(&block_metadata.to_tuple().unwrap())?;
				log::info!("DecodED block metadata");
				Ok(QpRemoteBlock {
					block_hash: H256::from_slice(block_hash.to_fixed_bytes().unwrap().as_slice()),
					miner: miner.to_address().unwrap(),
					stake: stake.to_uint().unwrap(),
					total_value: total_value.to_uint().unwrap(),
					block_metadata,
				})
			},
			_ => Err(b"Unexpected output. Could not decode mined block".as_slice().into()),
		}
	}

	fn compare_and_verify_mined_block(
		source_txs: &[QpTransaction],
		mined_txs: &[QpTransaction],
	) -> bool {
		// sanity check, ensure the source and mined transactions are the same
		if source_txs.len() != mined_txs.len() {
			log::info!(
				"Transaction count mismatch, source block has {:?} txs, mined block has {:?} txs",
				source_txs.len(),
				mined_txs.len()
			);
			return false
		}

		// ensure the transaction content is the same
		for transaction in source_txs.iter() {
			if !mined_txs.contains(transaction) {
				log::info!("Transaction content mismatch between source and mined txs");
				return false
			}
		}

		true
	}
}
