#![cfg_attr(not(feature = "std"), no_std)]

use log::log;
use crate::pallet::*;

use sp_runtime::{
	offchain::{
		http,
		Duration,
	},
	codec::{
		Decode, Encode
	},
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::json;
use sp_runtime::offchain::http::HttpResult;
use sp_std::{collections::vec_deque::VecDeque, prelude::*, str};
use crate::chain_utils::{ChainRequestError, ChainRequestResult};
use crate::chain_utils::ChainUtils;
use sp_core::{ H256 };
use ethereum::{TransactionV2};
use crate::qp_types::QuantumPortalTransaction;

const FETCH_TIMEOUT_PERIOD: u64 = 30000; // in milli-seconds

pub fn de_string_list_to_bytes_list<'de, D>(de: D) -> Result<Vec<Vec<u8>>, D::Error>
	where
		D: Deserializer<'de>,
{
	let s: Vec<&str> = Deserialize::deserialize(de)?;
	let list = s.iter().map(|v| v.as_bytes().to_vec()).collect();
	Ok(list)
}

pub fn de_string_to_bytes<'de, D>(de: D) -> Result<Vec<u8>, D::Error>
	where
		D: Deserializer<'de>,
{
	let s: &str = Deserialize::deserialize(de)?;
	Ok(s.as_bytes().to_vec())
}

// curl --data
// '{"method":"eth_chainId","params":[],"id":1,"jsonrpc":"2.0"}' -H "Content-Type: application/json"
// -X POST localhost:8545
#[derive(Debug, Deserialize, Serialize, Encode, Decode)]
pub struct JsonRpcRequest {
	id: u32,
	#[serde(deserialize_with = "de_string_to_bytes")]
	method: Vec<u8>,
	#[serde(deserialize_with = "de_string_list_to_bytes_list")]
	params: Vec<Vec<u8>>,
}

#[derive(Deserialize, Encode, Decode)]
pub struct  JsonRpcResponse<T> {
	id: u32,
	#[serde(deserialize_with = "de_string_to_bytes")]
	jsonrpc: Vec<u8>,
	response: T,
}

fn fetch_json_rpc_body(
	base_url: &str,
	req: &JsonRpcRequest,
) -> Result<Vec<u8>, ChainRequestError> {
	let json_req = json!({
		"id": req.id,
		"method": str::from_utf8(&req.method).unwrap(),
		"jsonrpc": "2.0"
	});
	let json_req_s = serde_json::to_vec(&json_req).unwrap();
	log::info!("About to submit {}", str::from_utf8(&json_req_s).unwrap());
	let request = http::Request::post(base_url, [&json_req_s]);
	// Keeping the offchain worker execution time reasonable, so limiting the call to be within 3s.
	let timeout = sp_io::offchain::timestamp()
		.add(Duration::from_millis(FETCH_TIMEOUT_PERIOD));

	let pending = request
		// .deadline(timeout) // Setting the timeout time
		.send() // Sending the request out by the host
		.map_err(|e| {
			log::info!("An ERROR HAPPNED!");
			log::error!("{:?}", e);
			ChainRequestError::ErrorGettingJsonRpcResponse
		})?;

	log::info!("Pendool!");
	// By default, the http request is async from the runtime perspective. So we are asking the
	//   runtime to wait here
	// The returning value here is a `Result` of `Result`, so we are unwrapping it twice by two `?`
	//   ref: https://docs.substrate.io/rustdocs/latest/sp_runtime/offchain/http/struct.PendingRequest.html#method.try_wait
	let response_a = pending.try_wait(timeout);
	let response_0 = match response_a {
		Ok(r) => {
			log::info!("Result got");
			Ok(r)
		},
		Err(e) => {
			log::info!("An ERROR HAPPNED!");
			log::info!("An ERROR HAPPNED UYPOOOOOOOOOOO ! {:?}", e);
			Err(ChainRequestError::ErrorGettingJsonRpcResponse)
		},
	}?;
	let response = match response_0 {
		Ok(r) => {
			log::info!("Result got 2");
			Ok(r)
		},
		Err(e) => {
			log::info!("An ERROR HAPPNED 2!");
			log::info!("An ERROR HAPPNED UYPOOOOOOOOOOO 2 ! {:?}", e);
			Err(ChainRequestError::ErrorGettingJsonRpcResponse)
		}
	}?;
	// let response = pending
	// 	.try_wait(timeout)
	// 	.map_err(|e| {
	// 		log::info!("An ERROR HAPPNED!");
	// 		log::info!("An ERROR HAPPNED UYPOOOOOOOOOOO ! {:?}", e);
	// 		log::error!("{:?}", e);
	// 		ChainRequestError::ErrorGettingJsonRpcResponse
	// 	})?
	// 	.map_err(|e| {
	// 		log::info!("An ERROR HAPPNED22!");
	// 		log::error!("{:?}", e);
	// 		ChainRequestError::ErrorGettingJsonRpcResponse
	// 	})?;

	log::info!("Response is ready!");
	log::info!("Response code got : {}", &response.code);

	if response.code != 200 {
		log::error!("Unexpected http request status code: {}", response.code);
		return Err(ChainRequestError::ErrorGettingJsonRpcResponse)
	}

	Ok(response.body().collect::<Vec<u8>>().clone())
}

pub fn fetch_json_rpc<T>(
	base_url: &str,
	req: &JsonRpcRequest,
) -> Result<Box<T>, ChainRequestError>
where T: for<'de> Deserialize<'de> {
	let body = fetch_json_rpc_body(base_url, req)?;
	// log::info!("Response body got : {}", str::from_utf8(&body).unwrap());
	let rv: serde_json::Result<T> = serde_json::from_slice(&body);
	match rv {
		Err(err) => {
			log::error!("Error while parsing json {:?}", err);
			Err(ChainRequestError::ErrorGettingJsonRpcResponse)
		},
		Ok(v) => Ok(Box::new(v)),
	}
}

#[derive(Debug, Deserialize, Encode, Decode)]
struct GetChainIdResponse {
	#[serde(deserialize_with = "de_string_to_bytes")]
	result: Vec<u8>,
}

pub struct ChainQueries/*<T: Config>*/ {
}

impl ChainQueries {
// impl<T: Config> ChainQueries<T> {
	pub fn chain_id(url: &str) -> Result<u32, ChainRequestError> {
		log::info!("About to get chain_id {}", url);
		let req = JsonRpcRequest {
			id: 1,
			params: Vec::new(),
			method: b"eth_chainId".to_vec(),
		};
		log::info!("Have request {:?}", &req);
		let res: Box<GetChainIdResponse> = fetch_json_rpc(url, &req)?;
		log::info!("Result is {:?}", &res);
		let chain_id = ChainUtils::hex_to_u64(&res.result)?;
		Ok(chain_id as u32)
	}
}

pub struct QuantumPortalContract;

impl QuantumPortalContract {
	pub fn create_finalize_transaction(
		chain_id: u64,
		blockNonce: u64,
		finalizer_hash: H256,
		finalizers: H256
	) -> ChainRequestResult<TransactionV2> {
		// TODO: We need to encode the method. 'ethabi' cannot be imported
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
		Err(ChainRequestError::ErrorCreatingTransaction)
	}

	pub fn create_mine_transaction(
		chain1: u64,
		block_nonce: u64,
		txs: &[QuantumPortalTransaction],
	) -> ChainRequestResult<TransactionV2> {
		Err(ChainRequestError::ErrorCreatingTransaction)
	}

	pub fn is_local_block_ready(
		chain_id: u64,
	) -> ChainRequestResult<bool> {
		Err(ChainRequestError::ErrorCreatingTransaction)
	}
}
