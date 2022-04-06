use log::log;
use parity_scale_codec::MaxEncodedLen;
use sp_core::H256;
use sp_std::prelude::*;
use sp_std::str;
use frame_support::codec::{Encode, Decode};
use sp_runtime::offchain::storage::StorageValueRef;
use byte_slice_cast::{*};
use crate::chain_queries::{ChainQueries, TransactionStatus};
use crate::chain_utils::{ChainRequestError, ChainRequestResult, ChainUtils};
use crate::{Config, PendingTransactions};
use crate::quantum_portal_client::QuantumPortalClient;

const TIMEOUT: u64 = 3600 * 1000;

#[derive(Debug, Encode, Decode, Clone, PartialEq, MaxEncodedLen, scale_info::TypeInfo)]
pub enum  PendingTransaction {
    // MineTransaction(chain, remote_chain, timestamp, tx_id)
    MineTransaction(u64, u64, u64, H256),
    FinalizeTransaction(u64, u64, H256),
    None,
}

impl Default for PendingTransaction {
    fn default() -> Self {
        PendingTransaction::None
    }
}

pub struct QuantumPortalService<T: Config> {
    pub clients: Vec<QuantumPortalClient>,
    config: Option<T>, // To allow compilation. Not sued
}

impl <T: Config> QuantumPortalService<T> {
    pub fn new(clients: Vec<QuantumPortalClient>) -> Self {
        QuantumPortalService {
            clients,
            config: None,
        }
    }

    fn lock_is_open(&self) -> ChainRequestResult<bool> {
        // Save a None tx.
        let tx = self.stored_pending_transactions(9999)?;
        log::info!("Current pending txs {:?}", tx);
        if tx.is_empty() {
            log::info!("No lock! We can go ahead");
            return Ok(true);
        }
        log::info!("LOCKED! {:?}", tx.get(0).unwrap());
        Ok(false)
    }

    fn lock(&self) -> ChainRequestResult<()> {
        log::info!("Saving a lock!");
        self.save_tx(PendingTransaction::FinalizeTransaction(9999, 0, H256::zero()))?;
        Ok(())
    }

    fn remove_lock(&self) -> ChainRequestResult<()> {
        log::info!("Removing a lock!");
        let tx = PendingTransaction::FinalizeTransaction(9999, 0, H256::zero());
        self.remove_transaction_from_db(&tx)?;
        Ok(())
    }

    pub fn process_pair_with_lock(
        &self, remote_chain: u64, local_chain: u64) -> ChainRequestResult<()> {
        if !self.lock_is_open()? {
            log::info!("We will not proceed because we have a process lock lock. Processing {} => {}",
                remote_chain, local_chain);
            return Ok(());
        }
        self.lock()?;
        let tx = self.stored_pending_transactions(9999)?;
        log::info!("RESULTAT OF PENDING_TX {:?}", tx);
        let rv = self.process_pair(remote_chain, local_chain);
        self.remove_lock();
        rv?;
        Ok(())
    }

    pub fn process_pair(&self,
                        remote_chain: u64,
                        local_chain: u64,) -> ChainRequestResult<()>{
        // Processes between two chains.
        // If there is an existing pending tx, for this pair, it will wait until the pending is
        // completed or timed out.
        // Nonce management? :: V1. No special nonce management
        //                      V2. TODO: record and re-use the nonce to ensure controlled timeouts

        log::info!("process_pair: {} -> {}", remote_chain, local_chain);
        let live_txs = self.pending_transactions(local_chain)?; // TODO: Consider having separate config per pair
        if live_txs.len() > 0 {
            log::info!("There are already {} pending transactions. Ignoring this round",
                live_txs.len());
            return Ok(());
        }
        let local_client: &QuantumPortalClient = &self.clients[self.find_client_idx(local_chain)];
        let remote_client: &QuantumPortalClient = &self.clients[self.find_client_idx(remote_chain)];
        log::info!("Clients: {} <> {} :: {} <> {}", local_client.block_number, remote_client.block_number,
            local_client.contract.http_api,
            remote_client.contract.http_api,
        );
        let now = local_client.now;
        let fin_tx = local_client.finalize(remote_chain)?;
        if fin_tx.is_some() {
            // Save tx
            // MineTransaction(chain, remote_chain, timestamp, tx_id)
            self.save_tx(
                PendingTransaction::FinalizeTransaction(
                    local_chain, now, fin_tx.unwrap()
                ))?
        } else {
            // Save tx
            let mine_tx = local_client.mine(remote_client)?;
            if mine_tx.is_some() {
                self.save_tx(
                    PendingTransaction::MineTransaction(
                        local_chain, remote_chain, now, mine_tx.unwrap()
                    ))?
            }
        }
        self.remove_lock()?;
        Ok(())
    }

    fn storage_key(key: u64) -> Vec<u8> {
        let key = key.to_be_bytes();
        let key = key.as_slice();
        let key = ChainUtils::bytes_to_hex(key);
        let key = key.as_slice();
        let key_pre = b"quantum-portal::tx::".as_slice();
        let key = [key_pre, key].concat();
        Vec::from(key.as_slice())
    }

    fn save_tx(&self, tx: PendingTransaction) -> ChainRequestResult<()> {
        let key = Self::storage_key_for_tx(&tx);
        let key = Self::storage_key(key);
        let key = key.as_slice();
        let s = StorageValueRef::persistent(key);
        s.set(&tx);
        // PendingTransactions::<T>::insert(
        //     key,
        //     tx
        // );
        Ok(())
    }

    fn pending_transactions(&self, chain_id: u64) -> ChainRequestResult<Vec<PendingTransaction>> {
        let stored_pending_transactions = self.stored_pending_transactions(chain_id)?;
        Ok(stored_pending_transactions.into_iter().filter(
            |t| self.is_tx_pending(t).unwrap() // TODO: No unwrap here.
        ).collect())
    }

    fn stored_pending_transactions(&self, chain_id: u64) -> ChainRequestResult<Vec<PendingTransaction>> {
        let key = Self::storage_key(chain_id);
        let key = key.as_slice();
        let s = StorageValueRef::persistent(key);
        let rv = s.get().unwrap();
        Ok(match rv {
            None => {
                log::info!("stored_pending_transactions nichivo");
                Vec::new()
            },
            Some(v) => vec![v],
        })
        // let rv = PendingTransactions::<T>::try_get(chain_id);
        // Ok(match rv {
        //     Err(e) => {
        //         log::info!("Error stored_pending_transactions {:?}", e);
        //         Vec::new()
        //     },
        //     Ok(v) => vec![v],
        // })
    }

    fn remove_transaction_from_db(&self, t: &PendingTransaction) -> ChainRequestResult<()> {
        let key = Self::storage_key_for_tx(t);
        let key = Self::storage_key(key);
        let key = key.as_slice();
        let mut s = StorageValueRef::persistent(key);
        s.clear();
        Ok(())
    }

    fn is_tx_pending(&self, t: &PendingTransaction) -> ChainRequestResult<bool> {
        // Check if the tx is still pending
        // If so, return true.
        // otherwise. Update storage and remove the tx.
        // then return false
        let (chain_id1, chain_id2, timestamp, tx_id) = match t {
            PendingTransaction::MineTransaction(c1, c2, timestamp , tid) => (c1, c2, timestamp, tid),
            PendingTransaction::FinalizeTransaction(c, timestamp, tid) => (c, &(0 as u64), timestamp, tid),
            PendingTransaction::None => panic!("tx is none")
        };
        let client = &self.clients[self.find_client_idx(chain_id1.clone())];

        let status = ChainQueries::get_transaction_status(
            client.contract.http_api,
            tx_id)?;
        let res = match status {
            TransactionStatus::Confirmed => {
                // Remove
                log::info!("The transaction is confirmed! Please investigate {} - {}",
                    chain_id1, str::from_utf8(tx_id.0.as_slice()).unwrap());
                self.remove_transaction_from_db(t)?;
                false
            },
            TransactionStatus::Failed => {
                // Remove
                log::error!("The transaction is failed! Please investigate {} - {}",
                    chain_id1, str::from_utf8(tx_id.0.as_slice()).unwrap());
                self.remove_transaction_from_db(t)?;
                false
            },
            TransactionStatus::Pending => true,
            TransactionStatus::NotFound => {
                if (timestamp + TIMEOUT) > client.now {
                    log::error!("The transaction is timed out! Please investigate {} - {}",
                        chain_id1, str::from_utf8(tx_id.0.as_slice()).unwrap());
                    self.remove_transaction_from_db(t)?;
                    false
                } else {
                    true
                }
            },
        };
        Ok(res)
    }

    fn find_client_idx(&self, chain_id: u64) -> usize {
        let c = self.clients.as_slice();
        c.into_iter().position(
            |c| c.contract.chain_id == chain_id).unwrap()
    }

    fn storage_key_for_tx(tx: &PendingTransaction) -> u64 {
        match tx {
            PendingTransaction::MineTransaction(c, _, _, _) => c,
            PendingTransaction::FinalizeTransaction(c, _, _) => c,
            PendingTransaction::None => panic!("tx is none. Cannot save"),
        }.clone()
    }
}