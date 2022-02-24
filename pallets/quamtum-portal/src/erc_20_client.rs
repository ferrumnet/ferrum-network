#![cfg_attr(not(feature = "std"), no_std)]

use crate::chain_queries::{ChainQueries, fetch_json_rpc, JsonRpcRequest, CallResponse};
use sp_core::{ H256, U256 };
use sp_std::{str};
use ethereum::{Account, LegacyTransaction, TransactionAction, TransactionSignature, TransactionV2};
use hex_literal::hex;
use serde_json::json;
use ethabi_nostd::encoder;
use crate::chain_utils::{ChainRequestError, ToJson};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sp_core::crypto::{AccountId32};
use sp_std::prelude::*; //vec::{Vec};

struct Erc20Client {
    http_api: &'static str,
    contract_address: AccountId32,
}

// fn encode_input(
//     path: &str,
//     name_or_signature: &str,
//     values: &[Vec<u8>],
//     lenient: bool) -> anyhow::Result<Vec<u8>> {
//     let function = load_function(path, name_or_signature)?;
//
//     let params: Vec<_> =
//         function.inputs.iter().map(|param| param.kind.clone()).zip(values.iter().map(|v| v as &str)).collect();
//
//     let tokens = parse_tokens(&params, lenient)?;
//     let result = function.encode_input(&tokens)?;
//
//     Ok(result)
// }
//
impl Erc20Client {
    pub fn new(
        http_api: &'static str,
        contract_address: &AccountId32,
    ) -> Self {
        Erc20Client {
            http_api,
            contract_address: contract_address.clone(),
        }
    }

    pub fn total_supply(&self) -> Result<U256, ChainRequestError> {
        let signature = "totalSupply()";
        let encoded = encoder::encode_function(signature, &[]);
        // let addr = self.contract_address.as_slice();
        // log::info!("About to get total supply for {}", &str::from_utf8(addr).unwrap());
        let call = TransactionV2::Legacy(LegacyTransaction {
            nonce: Default::default(),
            gas_price: Default::default(),
            gas_limit: Default::default(),
            action: TransactionAction::Create,
            value: Default::default(),
            input: vec![],
            signature: TransactionSignature::new(0, H256::zero(), H256::zero()).unwrap()
        });
        let req = JsonRpcRequest {
            id: 1,
            params: Vec::from([call.to_json(), Vec::from("latest".as_bytes())]),
            method: b"eth_call".to_vec(),
        };
        log::info!("Have request {:?}", &req);
        let res: Box<CallResponse> = fetch_json_rpc(self.http_api, &req)?;
        log::info!("Result is {:?}", &res);
        Ok(U256::zero())
    }
}