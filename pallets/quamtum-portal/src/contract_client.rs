use ethabi_nostd::{encoder, Token};
use crate::chain_queries::{CallResponse, fetch_json_rpc, JsonRpcRequest};
use crate::chain_utils::{ChainRequestError, ChainUtils, JsonSer};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sp_std::{str};
use sp_std::prelude::*;

#[derive(Debug, Clone)]
pub struct ContractClient {
    http_api: &'static str,
    contract_address: &'static str,
}

impl ContractClient {
    pub fn new(
        http_api: &'static str,
        contract_address: &'static str
    ) -> Self {
        ContractClient {
            http_api,
            contract_address: contract_address.clone(),
        }
    }

    pub fn call<T>(
        &self,
        method_signature: &[u8],
        inputs: &[Token],
    ) -> Result<Box<T>, ChainRequestError>
        where T: for<'de> Deserialize<'de> {
        let encoded_bytes = encoder::encode_function_u8(method_signature, inputs);
        let encoded_bytes_0x = ChainUtils::bytes_to_hex(encoded_bytes.as_slice());
        let encoded_bytes_slice = encoded_bytes_0x.as_slice();
        let encoded_bytes_slice = ChainUtils::hex_add_0x(encoded_bytes_slice);
        let encoded = str::from_utf8(encoded_bytes_slice.as_slice()).unwrap();
        log::info!("encoded {}", encoded);
        let call_json = JsonSer::new()
            .start()
            .string("input", encoded)
            .string("to", self.contract_address)
            .end()
            .to_vec();
        log::info!("call_json is {}", str::from_utf8(&call_json).unwrap());
        let req = JsonRpcRequest {
            id: 1,
            params: Vec::from([call_json, Vec::from("\"latest\"".as_bytes())]),
            method: b"eth_call".to_vec(),
        };
        log::info!("Have request {:?}", &req);
        fetch_json_rpc(self.http_api, &req)
    }
}