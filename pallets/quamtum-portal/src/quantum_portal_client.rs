#![cfg_attr(not(feature = "std"), no_std)]

use crate::chain_queries::{ChainQueries, fetch_json_rpc, JsonRpcRequest, CallResponse, de_string_to_bytes};
use sp_core::{ecdsa, H160, H256, U256};
use sp_std::{str};
use ethereum::{Account, LegacyTransaction, TransactionAction, TransactionSignature, TransactionV2};
use frame_system::offchain::SigningTypes;
use hex_literal::hex;
use parity_scale_codec::Encode;
use serde_json::json;
use ethabi_nostd::{Address, encoder, ParamKind, Token};
use crate::chain_utils::{ChainRequestError, ChainRequestResult, ChainUtils, JsonSer, ToJson};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sp_core::crypto::{AccountId32};
use sp_std::prelude::*;
use ethabi_nostd::decoder::decode;
use crate::contract_client::ContractClient;
use crate::qp_types::QpLocalBlock;

pub struct QuantumPortalClient {
    contract: ContractClient,
}

impl QuantumPortalClient {
    pub fn new(contract: ContractClient) -> Self {
        QuantumPortalClient {
            contract
        }
    }

    pub fn is_local_block_ready(
        &self,
        chain_id: u64,
    ) -> ChainRequestResult<bool> {
        let signature = b"isLocalBlockReady(uint64)";
        let mut res: Box<CallResponse> = self.contract.call(signature,
                                   &[
                                       Token::Uint(U256::from(chain_id))
                                   ])?;
        let val = ChainUtils::hex_to_u256(&res.result)?;
        Ok(!val.is_zero())
    }

    pub fn last_remote_mined_block(
        &self,
        chain_id: u64,
    ) -> ChainRequestResult<QpLocalBlock> {
        let signature = b"lastRemoteMinedBlock(uint64)";
        let mut res: Box<CallResponse> = self.contract.call(signature,
                                                            &[
                                                                Token::Uint(U256::from(chain_id))
                                                            ])?;
        let dec = decode(
            &[ParamKind::Tuple(vec![
                Box::new(ParamKind::Uint(256)),
                Box::new(ParamKind::Uint(256)),
                Box::new(ParamKind::Uint(256)),
            ])],
            ChainUtils::hex_to_bytes(&res.result)?.as_slice(),
        ).unwrap();
        match dec.as_slice() {
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
            _ => Err(b"Unexpected output for last_remote_mined_block".as_slice().into())
        }
    }
}
