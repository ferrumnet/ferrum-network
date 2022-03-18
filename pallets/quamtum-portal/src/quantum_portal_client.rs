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
use crate::qp_types::{QpLocalBlock, QpRemoteBlock, QpTransaction};

pub struct QuantumPortalClient {
    contract: ContractClient,
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
    mined_block_tuple_decoder: F,
) -> ChainRequestResult<(T, Vec<QpTransaction>)> where
    F: Fn(Token) -> ChainRequestResult<T> {
    let dec = decode(
        &[
            ParamKind::Tuple(vec![
                Box::new(mined_block_tuple),
                Box::new(ParamKind::Array(
                    Box::new(ParamKind::Tuple(vec![         // RemoteTransaction[]
                                                            Box::new(ParamKind::Uint(256)),     // timestamp
                                                            Box::new(ParamKind::Address),       // remoteContract
                                                            Box::new(ParamKind::Address),       // sourceMsgSender
                                                            Box::new(ParamKind::Address),       // sourceBeneficiary
                                                            Box::new(ParamKind::Address),       // token
                                                            Box::new(ParamKind::Uint(256)),     // amount
                                                            Box::new(ParamKind::Bytes),         // method
                                                            Box::new(ParamKind::Uint(256)),     // gas
                    ]))))
            ])],
        ChainUtils::hex_to_bytes(&data)?.as_slice(),
    ).unwrap();
    match dec.as_slice() {
        [mined_block, remote_transactions] => {
            let mined_block = mined_block.clone();
            let remote_transactions = remote_transactions.clone();
            // let mined_block = Self::decode_mined_block_from_tuple(
            //     mined_block.to_array().unwrap().as_slice())?;
            let block = mined_block_tuple_decoder(mined_block)?;
            let remote_transactions = remote_transactions.to_array().unwrap()
                .into_iter()
                .map(|t|
                    decode_remote_transaction_from_tuple(
                        t.to_array().unwrap().as_slice()).unwrap()).collect();
            Ok((block, remote_transactions))
        },
        _ => Err(b"Unexpected output. Could not decode local block".as_slice().into())
    }
}

fn decode_remote_transaction_from_tuple(
    dec: &[Token],
) -> ChainRequestResult<QpTransaction> {
    match dec {
        [timestamp,
        remote_contract,
        source_msg_sender,
        source_beneficiary,
        token,
        amount,
        method,
        gas] => {
            let timestamp = timestamp.clone().to_uint().unwrap().as_u64();
            let remote_contract = remote_contract.clone().to_address().unwrap();
            let source_msg_sender = source_msg_sender.clone().to_address().unwrap();
            let source_beneficiary = source_beneficiary.clone().to_address().unwrap();
            let token = token.clone().to_address().unwrap();
            let amount = amount.clone().to_uint().unwrap();
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
                gas
            })
        },
        _ => Err(b"Unexpected output. Could not decode remote transaction".as_slice().into())
    }
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
        self.decode_local_block(res.result.as_slice())
    }

    pub fn last_finalized_block(
        &self,
        chain_id: u64,
    ) -> ChainRequestResult<QpLocalBlock> {
        let signature = b"lastFinalizedBlock(uint64)";
        let res: Box<CallResponse> = self.contract.call(signature,
                                                            &[
                                                                Token::Uint(U256::from(chain_id))
                                                            ])?;
        self.decode_local_block(res.result.as_slice())
    }

    pub fn last_local_block(
        &self,
        chain_id: u64,
    ) -> ChainRequestResult<QpLocalBlock> {
        let signature = b"lastLocalBlock(uint64)";
        let res: Box<CallResponse> = self.contract.call(signature,
                                                        &[
                                                            Token::Uint(U256::from(chain_id))
                                                        ])?;
        self.decode_local_block(res.result.as_slice())
    }

    pub fn local_block_by_nonce(
        &self,
        chain_id: u64,
        last_block_nonce: u64,
    ) -> ChainRequestResult<(QpLocalBlock, Vec<QpTransaction>)> {
        let signature = b"localBlockByNonce(uint64,uint64)";
        let res: Box<CallResponse> = self.contract.call(signature,
                                                        &[
                                                            Token::Uint(U256::from(chain_id)),
                                                            Token::Uint(U256::from(last_block_nonce))
                                                        ])?;
        decode_remote_block_and_txs(
            res.result.as_slice(),
            local_block_tuple(),
            |block| {
                Self::decode_local_block_from_tuple(block.to_array().unwrap().as_slice())
            },
        )
    }

    pub fn mined_block_by_nonce(
        &self,
        chain_id: u64,
        last_block_nonce: u64,
    ) -> ChainRequestResult<(QpRemoteBlock, Vec<QpTransaction>)> {
        let signature = b"minedBlockByNonce(uint64,uint64)";
        let res: Box<CallResponse> = self.contract.call(signature,
                                        &[
                                            Token::Uint(U256::from(chain_id)),
                                            Token::Uint(U256::from(last_block_nonce))
                                        ])?;
        let mined_block_tuple = ParamKind::Tuple(vec![             // MinedBlock
                                                                   Box::new(ParamKind::FixedBytes(32)),    // blockHash
                                                                   Box::new(ParamKind::Address),           // miner
                                                                   Box::new(ParamKind::Uint(256)),         // stake
                                                                   Box::new(ParamKind::Uint(256)),         // totalValue
                                                                   Box::new(local_block_tuple())
        ]);
        decode_remote_block_and_txs(
            res.result.as_slice(),
            mined_block_tuple,
            |block| {
                Self::decode_mined_block_from_tuple(block.to_array().unwrap().as_slice())
            },
        )
    }


    fn decode_local_block(
        &self,
        data: &[u8]
    ) -> ChainRequestResult<QpLocalBlock> {
        let dec = decode(
            &[local_block_tuple()],
            ChainUtils::hex_to_bytes(data)?.as_slice(),
        ).unwrap();
        Self::decode_local_block_from_tuple(dec.as_slice())
    }

    fn decode_local_block_from_tuple(
        dec: &[Token],
    ) -> ChainRequestResult<QpLocalBlock> {
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
            _ => Err(b"Unexpected output. Could not decode local block".as_slice().into())
        }
    }

    fn decode_mined_block_from_tuple(
        dec: &[Token],
    ) -> ChainRequestResult<QpRemoteBlock> {
        match dec {
            [block_hash, miner, stake, total_value, block_metadata] => {
                let block_hash = block_hash.clone();
                let miner = miner.clone();
                let stake = stake.clone();
                let total_value = total_value.clone();
                let block_metadata = block_metadata.clone();
                let block_metadata = Self::decode_local_block_from_tuple(
                    &block_metadata.to_array().unwrap())?;
                Ok(QpRemoteBlock {
                    block_hash: H256::from_slice(block_hash.to_bytes().unwrap().as_slice()),
                    miner: miner.to_address().unwrap(),
                    stake: U256::from(stake.to_bytes().unwrap().as_slice()),
                    total_value: U256::from(total_value.to_bytes().unwrap().as_slice()),
                    block_metadata,
                })
            },
            _ => Err(b"Unexpected output. Could not decode mined block".as_slice().into())
        }
    }
}
