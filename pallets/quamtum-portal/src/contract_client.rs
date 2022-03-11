use ethereum::{LegacyTransaction, TransactionAction, TransactionSignature, TransactionV2};
use ethabi_nostd::{encoder, Token, Address};
use crate::chain_queries::{CallResponse, fetch_json_rpc, JsonRpcRequest};
use crate::chain_utils::{ChainRequestError, ChainUtils, JsonSer};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sp_core::{ecdsa, H256, U256};
use sp_std::{str};
use sp_std::prelude::*;
use rlp::{Encodable};

#[derive(Debug, Clone)]
pub struct ContractClient {
    pub http_api: &'static str,
    pub contract_address: Address,
    pub chain_id: u64,
}

impl ContractClient {
    pub fn new(
        http_api: &'static str,
        contract_address: &Address,
        chain_id: u64,
    ) -> Self {
        ContractClient {
            http_api,
            contract_address: contract_address.clone(),
            chain_id,
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
            .string("to", str::from_utf8(
                ChainUtils::address_to_hex(self.contract_address).as_slice()).unwrap())
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

    pub fn send(
        &self,
        method_signature: &[u8],
        inputs: &[Token],
        gas_limit: Option<U256>,
        gas_price: U256,
        value: U256,
        nonce: Option<U256>,
        from: Address,
        signer: fn(&H256) -> ecdsa::Signature,
    ) -> Result<H256, ChainRequestError> {
        let encoded_bytes = encoder::encode_function_u8(method_signature, inputs);
        let encoded_bytes_0x = ChainUtils::bytes_to_hex(&encoded_bytes.as_slice());
        let encoded_bytes_slice = encoded_bytes_0x.as_slice();
        let encoded_bytes_slice = ChainUtils::hex_add_0x(encoded_bytes_slice);
        let encoded = str::from_utf8(encoded_bytes_slice.as_slice()).unwrap();
        log::info!("encoded {}", encoded);
        let nonce_val = match nonce {
            None => {
                // TODO: Get nonce for "from"
                U256::zero()
            },
            Some(v) => v,
        };
        let gas_limit_val = match gas_limit {
            None => {
                // TODO: Get the gas limit
                U256::zero()
            },
            Some(v) => v
        };
        let mut tx = LegacyTransaction {
            nonce: nonce_val,
            gas_price,
            gas_limit: gas_limit_val,
            action: TransactionAction::Call(self.contract_address),
            value,
            input: encoded_bytes,
            signature: ChainUtils::empty_signature(),
        };
        let hash = tx.hash();
        let sig_bytes: ecdsa::Signature = signer(&hash);
        let sig = ChainUtils::decode_transaction_signature(
            &sig_bytes.0, self.chain_id)?;
        tx.signature = sig;

        let raw_tx = tx.rlp_bytes();
        let hex_tx = ChainUtils::bytes_to_hex(&raw_tx);
        let hex_tx_fmtd = ChainUtils::wrap_in_quotes(
            ChainUtils::hex_add_0x(hex_tx.as_slice()).as_slice());
        let req = JsonRpcRequest {
            id: 1,
            params: Vec::from([hex_tx_fmtd]),
            method: b"eth_sendRawTransaction".to_vec(),
        };
        log::info!("Have request {:?}", &req);
        let rv: Box<CallResponse> = fetch_json_rpc(self.http_api, &req)?;
        log::info!("Have response {:?}", &rv);
        Ok(H256::from_slice(rv.result.as_slice()))
    }
}