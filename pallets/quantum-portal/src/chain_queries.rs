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

use crate::chain_utils::{ChainRequestError, ChainRequestResult, ChainUtils, JsonSer, ToJson};
use ethereum::TransactionV2;
use serde::{Deserialize, Deserializer, Serialize};
use sp_core::H256;
use sp_runtime::{
    codec::{Decode, Encode},
    offchain::{http, Duration},
};
use sp_std::{prelude::*, str};

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
    pub id: u32,
    #[serde(deserialize_with = "de_string_to_bytes")]
    pub method: Vec<u8>,
    #[serde(deserialize_with = "de_string_list_to_bytes_list")]
    pub params: Vec<Vec<u8>>,
}

#[derive(Deserialize, Encode, Decode)]
pub struct JsonRpcResponse<T> {
    pub id: u32,
    #[serde(deserialize_with = "de_string_to_bytes")]
    pub jsonrpc: Vec<u8>,
    pub response: T,
}

#[derive(Debug, Deserialize)]
pub struct CallResponse {
    #[serde(deserialize_with = "de_string_to_bytes")]
    pub result: Vec<u8>,
}

#[allow(dead_code)]
pub enum TransactionStatus {
    NotFound,
    Pending,
    Confirmed,
    Failed,
}

impl ToJson for TransactionV2 {
    type BaseType = TransactionV2;
    fn to_json(&self) -> Vec<u8> {
        let mut j = JsonSer::new();
        let j = match self {
            TransactionV2::Legacy(tx) => j
                .start()
                .string(
                    "nonce",
                    str::from_utf8(ChainUtils::u256_to_hex_0x(&tx.nonce).as_slice()).unwrap(),
                )
                .string(
                    "gas_price",
                    str::from_utf8(ChainUtils::u256_to_hex_0x(&tx.gas_price).as_slice()).unwrap(),
                )
                .string(
                    "gas_limit",
                    str::from_utf8(ChainUtils::u256_to_hex_0x(&tx.gas_limit).as_slice()).unwrap(),
                )
                // .string("action",
                // 		str::from_utf8(ChainUtils::u256_to_hex_0x(&tx.action).as_slice()).unwrap())
                .string(
                    "value",
                    str::from_utf8(ChainUtils::u256_to_hex_0x(&tx.value).as_slice()).unwrap(),
                )
                .string("input", str::from_utf8(&tx.input).unwrap())
                .val(
                    "signature",
                    str::from_utf8(
                        JsonSer::new()
                            .start()
                            .string(
                                "r",
                                str::from_utf8(
                                    ChainUtils::h256_to_hex_0x(tx.signature.r()).as_slice(),
                                )
                                .unwrap(),
                            )
                            .string(
                                "s",
                                str::from_utf8(
                                    ChainUtils::h256_to_hex_0x(tx.signature.s()).as_slice(),
                                )
                                .unwrap(),
                            )
                            .num("v", tx.signature.v())
                            .end()
                            .to_vec()
                            .as_slice(),
                    )
                    .unwrap(),
                )
                .end()
                .to_vec(),
            TransactionV2::EIP1559(_) => Vec::new(),
            TransactionV2::EIP2930(_) => Vec::new(),
        };
        j
    }
}

fn fetch_json_rpc_body(base_url: &str, req: &JsonRpcRequest) -> Result<Vec<u8>, ChainRequestError> {
    let mut params = JsonSer::new();
    req.params.iter().for_each(|p| {
        params.arr_val(str::from_utf8(p.as_slice()).unwrap());
    });
    let mut json_req = JsonSer::new();
    let json_req_s = json_req
        .start()
        .num("id", req.id as u64)
        .string("method", str::from_utf8(&req.method).unwrap())
        .string("jsonrpc", "2.0")
        .arr(
            "params",
            str::from_utf8(params.to_vec().as_slice()).unwrap(),
        )
        .end()
        .to_vec();
    let json_req_str = str::from_utf8(&json_req_s).unwrap();
    log::info!("About to submit {}", json_req_str);
    let request: http::Request<Vec<&[u8]>> =
        http::Request::post(base_url, Vec::from([json_req_s.as_slice()]));
    let timeout = sp_io::offchain::timestamp().add(Duration::from_millis(FETCH_TIMEOUT_PERIOD));

    let pending = request
        // .deadline(timeout) // Setting the timeout time
        .add_header("Content-Type", "application/json")
        .send() // Sending the request out by the host
        .map_err(|e| {
            log::info!("An ERROR HAPPNED!");
            // println!("ERRROOOORRRR {:?}", e);
            log::error!("{:?}", e);
            ChainRequestError::ErrorGettingJsonRpcResponse
        })?;

    // By default, the http request is async from the runtime perspective. So we are asking the
    //   runtime to wait here
    // The returning value here is a `Result` of `Result`, so we are unwrapping it twice by two `?`
    //   ref: https://docs.substrate.io/rustdocs/latest/sp_runtime/offchain/http/struct.PendingRequest.html#method.try_wait
    let response_a = pending.try_wait(timeout);
    // let response_0 = pending.wait();
    let response_0 = match response_a {
        Ok(r) => {
            // println!("Result got");
            // log::info!("Result got");
            Ok(r)
        }
        Err(e) => {
            // println!("ERRROOOORRRR AFDTER {:?}", e);
            log::info!("An ERROR HAPPNED!");
            log::info!("An ERROR HAPPNED UYPOOOOOOOOOOO ! {:?}", e);
            Err(ChainRequestError::ErrorGettingJsonRpcResponse)
        }
    }?;
    let response = match response_0 {
        Ok(r) => {
            // log::info!("Result got 2");
            Ok(r)
        }
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

    // log::info!("Response is ready!");
    let body = response.body().collect::<Vec<u8>>();
    log::info!(
        "Response code got : {}-{}",
        &response.code,
        str::from_utf8(body.as_slice()).unwrap()
    );

    if response.code != 200 {
        log::error!("Unexpected http request status code: {}", response.code);
        return Err(ChainRequestError::ErrorGettingJsonRpcResponse);
    }

    Ok(body)
}

pub fn fetch_json_rpc<T>(base_url: &str, req: &JsonRpcRequest) -> Result<Box<T>, ChainRequestError>
where
    T: for<'de> Deserialize<'de>,
{
    // println!("fetchin {} : {:?}", base_url, req);
    let body = fetch_json_rpc_body(base_url, req)?;
    // println!("Response body got : {}", str::from_utf8(&body).unwrap());
    // log::info!("Response body got : {}", str::from_utf8(&body).unwrap());
    let rv: serde_json::Result<T> = serde_json::from_slice(&body);
    match rv {
        Err(err) => {
            log::error!("Error while parsing json {:?}", err);
            Err(ChainRequestError::ErrorGettingJsonRpcResponse)
        }
        Ok(v) => Ok(Box::new(v)),
    }
}

#[derive(Debug, Deserialize, Encode, Decode)]
struct GetChainIdResponse {
    #[serde(deserialize_with = "de_string_to_bytes")]
    result: Vec<u8>,
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize, Encode, Decode)]
pub struct GetTransactionReceiptResponseData {
    #[serde(deserialize_with = "de_string_to_bytes")]
    blockHash: Vec<u8>,
    #[serde(deserialize_with = "de_string_to_bytes")]
    blockNumber: Vec<u8>,
    #[serde(deserialize_with = "de_string_to_bytes")]
    status: Vec<u8>,
}

#[derive(Debug, Deserialize, Encode, Decode)]
pub struct GetTransactionReceiptResponse {
    result: Option<GetTransactionReceiptResponseData>,
}

pub struct ChainQueries /* <T: Config> */ {}

impl ChainQueries {
    #[allow(dead_code)]
    pub fn chain_id(url: &str) -> Result<u32, ChainRequestError> {
        log::info!("About to get chain_id {}", url);
        let req = JsonRpcRequest {
            id: 1,
            params: Vec::new(),
            method: b"eth_chainId".to_vec(),
        };
        // log::info!("Have request {:?}", &req);
        let res: Box<GetChainIdResponse> = fetch_json_rpc(url, &req)?;
        log::info!("Result is {:?}", &res);
        let chain_id = ChainUtils::hex_to_u64(&res.result)?;
        Ok(chain_id as u32)
    }

    pub fn get_transaction_receipt(
        url: &str,
        tx_id: &H256,
    ) -> ChainRequestResult<Option<GetTransactionReceiptResponseData>> {
        log::info!("TX_ID is: {:?}", &tx_id.0);
        let tx_id = ChainUtils::h256_to_hex_0x(tx_id);
        log::info!(
            "About to get eth_getTransactionReceipt {}: {}",
            url,
            str::from_utf8(tx_id.as_slice()).unwrap()
        );

        let req = JsonRpcRequest {
            id: 1,
            params: vec![ChainUtils::wrap_in_quotes(tx_id.as_slice()).to_vec()],
            method: b"eth_getTransactionReceipt".to_vec(),
        };
        // log::info!("Have request {:?}", &req);
        let res: Box<GetTransactionReceiptResponse> = fetch_json_rpc(url, &req)?;
        log::info!("Result is {:?}", &res);
        Ok(res.result)
    }

    pub fn get_transaction_status(
        url: &str,
        tx_id: &H256,
    ) -> ChainRequestResult<TransactionStatus> {
        let rv = Self::get_transaction_receipt(url, tx_id)?;
        let res = match rv {
            None => TransactionStatus::NotFound,
            Some(tx) => {
                let status = ChainUtils::hex_to_u64(tx.status.as_slice())?;
                if status == 1 {
                    TransactionStatus::Confirmed
                } else {
                    TransactionStatus::Failed
                }
            }
        };
        Ok(res)
    }
}
