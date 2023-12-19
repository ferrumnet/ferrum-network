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
use crate::offchain::{
	btc_client::BTCClient,
	chain_queries::{fetch_json_rpc, CallResponse, JsonRpcRequest},
	chain_utils::{ChainRequestError, ChainUtils, JsonSer, TransactionCreationError},
};
use bitcoin::{Amount, Txid};
use codec::Encode;
use ethabi_nostd::{encoder, Address, Token};
use ethereum::{LegacyTransaction, TransactionAction};
use ferrum_primitives::OFFCHAIN_SIGNER_KEY_TYPE;
use rlp::Encodable;
use serde::{Deserialize, Serialize};
use sp_core::{ecdsa, H160, H256, U256};
use sp_io::crypto;
use sp_std::{
	ops::{Div, Mul},
	prelude::*,
	str,
};

#[derive(Debug, Clone)]
pub struct EvmClient;

impl EvmClient {
	fn handle_new_incoming_transaction(txids: Vec<Txid>) {
		// read all intents on chain
		let rpc_endpoint = "http://127.0.0.1:8545"; // local ferrum node

		let intent_contract_address =
			ChainUtils::hex_to_address(b"c1F13fde5fFDE7B7ae6C95C9190d038A2eEb9e29");
		let client = ContractClient::new(rpc_endpoint.into(), &intent_contract_address, 26100);
		let intents = client.get_registered_intents().unwrap();

		// lets decode all btc transactionids
		let transactions: Vec<(u64, Address)> = vec![];

		for txid in txids {
			let tx = BTCClient::get_transaction_details(current_pool_address).unwrap();
			let sender = BTCClient::extract_public_key_from_script(
				&tx.input[0].script_sig.as_bytes().to_vec(),
			);
			transactions.push((tx.output[0].value, sender));
		}

		// search if any intents have sent tx
		for intent in intents {
			if transactions.contains((intent.btc_amount, intent.btc_address)) {
				// send call to execute intent
				let inputs = [Token::Uint(U256::from(intent.id))];

				let method_signature = b"executeIntent(uint256)";

				let recipient_address = client.contract.intent_contract_address;

				let res = client.contract.send(
					method_signature,
					&inputs,
					None, //Some(U256::from(1000000 as u64)), // None,
					None, //Some(U256::from(10000000000 as u64)), // None,
					U256::zero(),
					None,
					client.signer.from,
					&client.signer,
					recipient_address,
				)?;
			}
		}
	}
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct Intent {
	id: u8,
	btc_amount: U256,
	btc_address: Address,
	target_contract: Address,
	encoded_call: Box<[u8]>,
	executed: bool,
}

#[derive(Debug, Clone)]
pub struct ContractClient {
	pub http_api: Vec<u8>,
	pub intent_contract_address: Address,
	pub chain_id: u64,
}

// #[derive(Clone)]
pub struct ContractClientSignature {
	pub from: Address,
	pub _signer: ecdsa::Public,
}

impl ContractClientSignature {
	pub fn new(from: Address, signer: &[u8]) -> Self {
		ContractClientSignature { from, _signer: ecdsa::Public::try_from(signer).unwrap() }
	}

	pub fn signer(&self, hash: &H256) -> Result<ecdsa::Signature, TransactionCreationError> {
		log::info!("Signer address is : {:?}", self.from);
		// TODO : We should handle this properly, if the signing is not possible maybe propogate the
		// error upstream
		let signed: Result<ecdsa::Signature, TransactionCreationError> =
			crypto::ecdsa_sign_prehashed(OFFCHAIN_SIGNER_KEY_TYPE, &self._signer, &hash.0)
				.ok_or(TransactionCreationError::SigningFailed);

		if signed.is_ok() {
			let sig_bytes = signed.as_ref().unwrap().encode();
			log::info!(
				"Got a signature of size {}: {}",
				sig_bytes.len(),
				str::from_utf8(ChainUtils::bytes_to_hex(sig_bytes.as_slice()).as_slice()).unwrap()
			);
		}

		signed
	}

	pub fn get_signer_address(&self) -> Vec<u8> {
		log::info!("Signer address is : {:?}", self.from);
		self._signer.as_ref().to_vec()
	}
}

impl From<ecdsa::Public> for ContractClientSignature {
	fn from(signer: ecdsa::Public) -> Self {
		log::info!("PUBLIC KEY {:?}", signer);
		let addr = ChainUtils::eth_address_from_public_key(&signer.0);
		let from = H160::from_slice(addr.as_slice());

		ContractClientSignature { _signer: signer, from }
	}
}

impl ContractClient {
	pub fn new(http_api: Vec<u8>, intent_contract_address: &Address, chain_id: u64) -> Self {
		ContractClient { http_api, intent_contract_address: *intent_contract_address, chain_id }
	}

	pub fn get_registered_intents(&self) -> Result<Vec<Intent>, ChainRequestError> {
		// no cache, we fetch from the gateway contract
		let signature = b"getAllIntents()";
		let res: Box<CallResponse> =
			self.call(signature, &[], Some(self.intent_contract_address))?;
		log::info!("Intent Contract response is : {:?}", res);
		let intents = ChainUtils::decode_intents_response(res.result.as_slice());
		log::info!("Selected miner intents is : {:?}", intents);

		Ok(intents)
	}

	pub fn call<T>(
		&self,
		method_signature: &[u8],
		inputs: &[Token],
		address: Option<Address>,
	) -> Result<Box<T>, ChainRequestError>
	where
		T: for<'de> Deserialize<'de>,
	{
		log::info!("CALL : method_signature {:?}", method_signature);
		log::info!("CALL : inputs {:?}", inputs);
		let encoded_bytes = encoder::encode_function_u8(method_signature, inputs);
		let encoded_bytes_0x = ChainUtils::bytes_to_hex(encoded_bytes.as_slice());
		let encoded_bytes_slice = encoded_bytes_0x.as_slice();
		let encoded_bytes_slice = ChainUtils::hex_add_0x(encoded_bytes_slice);

		let encoded = str::from_utf8(encoded_bytes_slice.as_slice()).unwrap();
		log::info!("encoded {}", encoded);
		let contract_address = if let Some(address) = address {
			address
		} else {
			todo!(); //Remove
		};

		log::info!(
			"contract address is {}",
			str::from_utf8(ChainUtils::address_to_hex(contract_address).as_slice()).unwrap()
		);
		let call_json = JsonSer::new()
			.start()
			.string("data", encoded)
			.string(
				"to",
				str::from_utf8(ChainUtils::address_to_hex(contract_address).as_slice()).unwrap(),
			)
			.end()
			.to_vec();

		log::info!("call_json is {}", str::from_utf8(&call_json).unwrap());
		let req = JsonRpcRequest {
			id: 1,
			params: Vec::from([call_json, Vec::from("\"latest\"".as_bytes())]),
			method: b"eth_call".to_vec(),
		};
		log::info!("Have request {:?}", str::from_utf8(method_signature).unwrap());
		let http_api = str::from_utf8(&self.http_api[..]).unwrap();
		fetch_json_rpc(http_api, &req)
	}

	#[allow(clippy::too_many_arguments)]
	pub fn send(
		&self,
		method_signature: &[u8],
		inputs: &[Token],
		gas_limit: Option<U256>,
		gas_price: Option<U256>,
		value: U256,
		nonce: Option<U256>,
		from: Address,
		// encoded_bytes: Vec<u8>,
		signing: &ContractClientSignature,
		recipient_address: Address,
	) -> Result<H256, ChainRequestError> {
		let encoded_bytes = encoder::encode_function_u8(method_signature, inputs);
		let encoded_bytes_0x = ChainUtils::bytes_to_hex(encoded_bytes.as_slice());
		let encoded_bytes_slice = encoded_bytes_0x.as_slice();
		let encoded_bytes_slice = ChainUtils::hex_add_0x(encoded_bytes_slice);

		let nonce_val = match nonce {
			None => self.nonce(from)?,
			Some(v) => v,
		};
		let gas_limit_val = match gas_limit {
			None =>
				self.estimate_gas(encoded_bytes_slice.as_slice(), &value, from, recipient_address)?,
			Some(v) => v,
		};
		let gas_price_val = match gas_price {
			None => self.gas_price()?.mul(U256::from(125_u32)).div(U256::from(100_u32)),
			Some(v) => v,
		};
		let mut tx = LegacyTransaction {
			nonce: nonce_val,
			gas_price: gas_price_val,
			gas_limit: gas_limit_val,
			action: TransactionAction::Call(recipient_address),
			value,
			input: encoded_bytes,
			signature: ChainUtils::empty_signature(),
		};
		let hash = ChainUtils::tx_hash_to_sign(&tx, self.chain_id);
		let sig_bytes: ecdsa::Signature = signing.signer(&hash)?;
		let sig = ChainUtils::decode_transaction_signature(&sig_bytes.0, self.chain_id)?;
		tx.signature = sig;

		let raw_tx = tx.rlp_bytes();
		let hex_tx = ChainUtils::bytes_to_hex(&raw_tx);
		let hex_tx_fmtd =
			ChainUtils::wrap_in_quotes(ChainUtils::hex_add_0x(hex_tx.as_slice()).as_slice());
		let req = JsonRpcRequest {
			id: 1,
			params: Vec::from([hex_tx_fmtd]),
			method: b"eth_sendRawTransaction".to_vec(),
		};
		// log::info!("Have request {:?}", &req);
		let http_api = str::from_utf8(&self.http_api[..]).unwrap();
		let rv: Box<CallResponse> = fetch_json_rpc(http_api, &req)?;
		log::info!("Have response {:?}", &rv);
		Ok(H256::from_slice(ChainUtils::hex_to_bytes(rv.result.as_slice())?.as_slice()))
	}

	pub fn nonce(&self, from: Address) -> Result<U256, ChainRequestError> {
		let req = JsonRpcRequest {
			id: 1,
			params: Vec::from([
				ChainUtils::wrap_in_quotes(ChainUtils::address_to_hex(from).as_slice()),
				b"\"latest\"".to_vec(),
			]),
			method: b"eth_getTransactionCount".to_vec(),
		};
		let http_api = str::from_utf8(&self.http_api[..]).unwrap();
		let rv: Box<CallResponse> = fetch_json_rpc(http_api, &req)?;
		let nonce = ChainUtils::hex_to_u64(rv.result.as_slice())?;
		Ok(U256::from(nonce))
	}

	pub fn gas_price(&self) -> Result<U256, ChainRequestError> {
		let req = JsonRpcRequest { id: 1, params: Vec::new(), method: b"eth_gasPrice".to_vec() };
		let http_api = str::from_utf8(&self.http_api[..]).unwrap();
		let rv: Box<CallResponse> = fetch_json_rpc(http_api, &req)?;
		let gp = ChainUtils::hex_to_u256(rv.result.as_slice())?;
		Ok(gp)
	}

	pub fn estimate_gas(
		&self,
		encoded: &[u8],
		value: &U256,
		from: Address,
		recipient_address: Address,
	) -> Result<U256, ChainRequestError> {
		let call_json = JsonSer::new()
			.start()
			.string("input", str::from_utf8(encoded).unwrap())
			.string("from", str::from_utf8(ChainUtils::address_to_hex(from).as_slice()).unwrap())
			.string(
				"to",
				str::from_utf8(ChainUtils::address_to_hex(recipient_address).as_slice()).unwrap(),
			)
			.string("value", str::from_utf8(ChainUtils::u256_to_hex_0x(value).as_slice()).unwrap())
			.end()
			.to_vec();
		log::info!("estimateGas json is {}", str::from_utf8(&call_json).unwrap());
		let req = JsonRpcRequest {
			id: 1,
			params: Vec::from([call_json, Vec::from("\"latest\"".as_bytes())]),
			method: b"eth_estimateGas".to_vec(),
		};
		let http_api = str::from_utf8(&self.http_api[..]).unwrap();
		let rv: Box<CallResponse> = fetch_json_rpc(http_api, &req)?;
		let gp = ChainUtils::hex_to_u256(rv.result.as_slice())?;
		Ok(gp)
	}
}
