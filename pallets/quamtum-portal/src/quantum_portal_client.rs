#![cfg_attr(not(feature = "std"), no_std)]

use crate::chain_queries::{ChainQueries, fetch_json_rpc, JsonRpcRequest, CallResponse, de_string_to_bytes};
use sp_core::{ecdsa, H160, H256, U256};
use sp_std::{str};
use ethereum::{Account, LegacyTransaction, TransactionAction, TransactionSignature, TransactionV2};
use hex_literal::hex;
use parity_scale_codec::Encode;
use serde_json::json;
use ethabi_nostd::{Address, encoder, ParamKind, Token};
use crate::chain_utils::{ChainRequestError, ChainRequestResult, ChainUtils, JsonSer, ToJson};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sp_core::crypto::{AccountId32};
use sp_std::prelude::*;
use ethabi_nostd::decoder::decode;
use ethabi_nostd::Token::{Tuple, Uint};
use crate::contract_client::{ContractClient, ContractClientSignature};
use crate::qp_types::{QpLocalBlock, QpRemoteBlock, QpTransaction};

const DUMMY_HASH: H256 = H256::zero();
const ZERO_HASH: H256 = H256::zero();

pub struct QuantumPortalClient {
    pub contract: ContractClient,
    pub signer: ContractClientSignature,
    pub now: u64,
    pub block_number: u64,
}

fn local_block_tuple0() -> Vec<ParamKind> {
    vec![
        ParamKind::Uint(256),
        ParamKind::Uint(256),
        ParamKind::Uint(256),
    ]
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
) -> ChainRequestResult<(T, Vec<QpTransaction>)> where
    F: Fn(Token) -> ChainRequestResult<T> {
    log::info!("decode_remote_block_and_txs {:?}", data);
    // let dec = decode(
    //     &[
    //         ParamKind::Tuple(vec![
    //             Box::new(mined_block_tuple),
    //             Box::new(ParamKind::Array(
    //                 Box::new(ParamKind::Tuple(vec![         // RemoteTransaction[]
    //                                                         Box::new(ParamKind::Uint(256)),     // timestamp
    //                                                         Box::new(ParamKind::Address),       // remoteContract
    //                                                         Box::new(ParamKind::Address),       // sourceMsgSender
    //                                                         Box::new(ParamKind::Address),       // sourceBeneficiary
    //                                                         Box::new(ParamKind::Address),       // token
    //                                                         Box::new(ParamKind::Uint(256)),     // amount
    //                                                         Box::new(ParamKind::Bytes),         // method
    //                                                         Box::new(ParamKind::Uint(256)),     // gas
    //                 ]))))
    //         ])],
    //     ChainUtils::hex_to_bytes(&data)?.as_slice(),
    // ).unwrap();
    let dec = decode(
            &[
                mined_block_tuple,
                ParamKind::Array(
                    Box::new(ParamKind::Tuple(vec![         // RemoteTransaction[]
                        Box::new(ParamKind::Uint(256)),     // timestamp
                        Box::new(ParamKind::Address),       // remoteContract
                        Box::new(ParamKind::Address),       // sourceMsgSender
                        Box::new(ParamKind::Address),       // sourceBeneficiary
                        Box::new(ParamKind::Address),       // token
                        Box::new(ParamKind::Uint(256)),     // amount
                        Box::new(ParamKind::Bytes),         // method
                        Box::new(ParamKind::Uint(256)),     // gas
                    ])))
            ],
        ChainUtils::hex_to_bytes(&data)?.as_slice(),
    ).unwrap();
    log::info!("decoded {:?}, - {}", dec, dec.as_slice().len());
    let dec: ChainRequestResult<Vec<Token>> = match dec.as_slice() {
        [tuple, txs] => Ok(vec![
            tuple.clone(),
            txs.clone(),
        ]),
        _ => Err(b"Unexpected output. Could not decode local block at first level".as_slice().into())
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
            let remote_transactions = remote_transactions.to_array().unwrap()
                .into_iter()
                .map(|t|
                    decode_remote_transaction_from_tuple(
                        t.to_tuple().unwrap().as_slice()).unwrap()).collect();
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
    pub fn new(
        contract: ContractClient,
        signer: ContractClientSignature,
        now: u64,
        block_number: u64,
    ) -> Self {
        QuantumPortalClient {
            contract,
            signer,
            now,
            block_number,
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
        let signature = b"lastFinalizedBlock(uint256)";
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
        let signature = b"lastLocalBlock(uint256)";
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
                log::info!("1-DECODING BLOCK {:?}", block);
                let block = block.to_tuple();
                let block = block.unwrap();
                log::info!("2-DECODING BLOCK {:?}", block);
                Self::decode_local_block_from_tuple(block.as_slice())
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
        // let mined_block_tuple = vec![             // MinedBlock
        //                                    ParamKind::FixedBytes(32),    // blockHash
        //                                    ParamKind::Address,           // miner
        //                                    ParamKind::Uint(256),         // stake
        //                                    ParamKind::Uint(256),         // totalValue
        //                                    local_block_tuple()
        // ];
        decode_remote_block_and_txs(
            res.result.as_slice(),
            mined_block_tuple,
            |block| {
                log::info!("Decoding local block, {:?}", block);
                Self::decode_mined_block_from_tuple(block.to_tuple().unwrap().as_slice())
            },
        )
    }

    pub fn create_finalize_transaction(
        &self,
        remote_chain_id: u64,
        block_nonce: u64,
        finalizer_hash: H256,
        finalizers: &[H256],
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
        let finalizer_list = finalizers.into_iter().map(
            |f| Token::FixedBytes(Vec::from(f.0.as_slice()))
        ).collect();
        let signature = b"finalize(uint256,uint256,bytes32,address[])";
        let res = self.contract.send(
            signature,
                        &[
                            Token::Uint(U256::from(remote_chain_id)),
                            Token::Uint(U256::from(block_nonce)),
                            Token::FixedBytes(Vec::from(finalizer_hash.as_bytes())),
                            Token::Array(finalizer_list),
                        ],
            None,
            None,
            U256::zero(),
            None,
            self.signer.from,
            self.signer.signer,
        )?;
        Ok(res)
    }

    pub fn create_mine_transaction(
        &self,
        remote_chain_id: u64,
        block_nonce: u64,
        txs: &Vec<QpTransaction>,
    ) -> ChainRequestResult<H256>{
        let signature = b"mineRemoteBlock(uint64,uint64,(uint64,address,address,address,address,uint256,bytes,uint256)[])";
        let tx_vec = txs
            .into_iter()
            .map(|t| Token::Tuple(
                vec![
                    Token::Uint(U256::from(t.timestamp)),
                    Token::Address(t.remote_contract),
                    Token::Address(t.source_msg_sender),
                    Token::Address(t.source_beneficiary),
                    Token::Address(t.token),
                    Token::Uint(t.amount),
                    Token::Bytes(t.method.clone()),
                    Token::Uint(U256::from(t.gas)),
                ]
            )).collect();
        let res = self.contract.send(
            signature,
            &[
                Token::Uint(U256::from(remote_chain_id)),
                Token::Uint(U256::from(block_nonce)),
                Token::Array(tx_vec),
            ],
            None,
            None,
            U256::zero(),
            None,
            self.signer.from,
            self.signer.signer,
        )?;
        Ok(res)
    }

    pub fn finalize(
        &self,
        chain_id: u64,) -> ChainRequestResult<Option<H256>>{
        log::info!("finalize({})", chain_id);
        let block = self.last_remote_mined_block(chain_id)?;
        log::info!("finalize-last_remote_mined_block({:?})", &block);
        let last_fin = self.last_finalized_block(chain_id)?;
        log::info!("finalize-last_finalized_block({:?})", &last_fin);
        if block.nonce > last_fin.nonce {
            log::info!("Calling mgr.finalize({}, {})", chain_id, last_fin.nonce);
            Ok(Some(self.create_finalize_transaction(
                chain_id,
                block.nonce,
                H256::zero(),
                &[],)?))
        } else {
            log::info!("Nothing to finalize for ({})", chain_id);
            Ok(None)
        }
    }


    pub fn mine(
        &self,
        remote_client: &QuantumPortalClient,
        ) -> ChainRequestResult<Option<H256>> {
        let local_chain = self.contract.chain_id;
        let remote_chain = remote_client.contract.chain_id;
        log::info!("mine({} => {})", remote_chain, local_chain);
        let block_ready = remote_client.is_local_block_ready(local_chain)?;
        log::info!("local block ready? {}", block_ready);
        if !block_ready { return  Ok(None); }
        log::info!("Getting last local block");
        let last_block = remote_client.last_local_block(local_chain)?;
        log::info!("Last local block is {:?}", last_block);
        let last_mined_block = self.last_remote_mined_block(remote_chain)?;
        log::info!("Local block f remote (chain {}) nonce is {}. Remote mined block on local (chain {}) is {}",
			remote_chain, last_block.nonce, local_chain, last_mined_block.nonce);
        if last_mined_block.nonce >= last_block.nonce {
            log::info!("Nothing to mine!");
            return Ok(None);
        }
        log::info!("Last block is on chain1 for target {} is {}", local_chain, last_block.nonce);
        let mined_block = self.mined_block_by_nonce(remote_chain, last_block.nonce)?;
        let already_mined = !mined_block.0.block_hash.eq(&ZERO_HASH);
        if already_mined {
            return Err(ChainRequestError::RemoteBlockAlreadyMined);
        }
        let source_block = remote_client.local_block_by_nonce(local_chain, last_block.nonce)?;
        let txs = source_block.1;
        log::info!("About to mine block {}:{}", remote_chain, source_block.0.nonce);
        Ok(Some(
            self.create_mine_transaction(remote_chain, source_block.0.nonce, &txs)?
        ))
    }

    fn decode_local_block(
        &self,
        data: &[u8]
    ) -> ChainRequestResult<QpLocalBlock> {
        let dec = decode(
            // &[local_block_tuple()],
            local_block_tuple0().as_slice(),
            ChainUtils::hex_to_bytes(data)?.as_slice(),
        ).unwrap();
        Self::decode_local_block_from_tuple(dec.as_slice())
    }

    fn decode_local_block_from_tuple(
        dec: &[Token],
    ) -> ChainRequestResult<QpLocalBlock> {
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
            _ => Err(b"Unexpected output. Could not decode local block".as_slice().into())
        }
    }

    fn decode_mined_block_from_tuple(
        dec: &[Token],
    ) -> ChainRequestResult<QpRemoteBlock> {
        log::info!("decode_mined_block_from_tuple {:?}", dec);
        match dec {
            [block_hash, miner, stake, total_value, block_metadata] => {
                log::info!("D {:?}::{:?}:{:?}:{:?}::{:?}", block_hash, miner, stake, total_value, block_metadata);
                let block_hash = block_hash.clone();
                let miner = miner.clone();
                let stake = stake.clone();
                let total_value = total_value.clone();
                let block_metadata = block_metadata.clone();
                log::info!("Decoding block metadata");
                let block_metadata = Self::decode_local_block_from_tuple(
                    &block_metadata.to_tuple().unwrap())?;
                log::info!("DecodED block metadata");
                Ok(QpRemoteBlock {
                    block_hash: H256::from_slice(block_hash.to_fixed_bytes().unwrap().as_slice()),
                    miner: miner.to_address().unwrap(),
                    stake: U256::from(stake.to_uint().unwrap()),
                    total_value: U256::from(total_value.to_uint().unwrap()),
                    block_metadata,
                })
            },
            _ => Err(b"Unexpected output. Could not decode mined block".as_slice().into())
        }
    }
}