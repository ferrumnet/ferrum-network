use log::log;
use parity_scale_codec::MaxEncodedLen;
use sp_core::H256;
use sp_std::prelude::*;
use sp_std::str;
use frame_support::codec::{Encode, Decode};
use crate::chain_queries::{ChainQueries, TransactionStatus};
use crate::chain_utils::{ChainRequestError, ChainRequestResult};
use crate::{Config, PendingTransactions};
use crate::quantum_portal_client::QuantumPortalClient;

const TIMEOUT: u64 = 3600 * 1000;

#[derive(Encode, Decode, Clone, PartialEq, MaxEncodedLen, scale_info::TypeInfo)]
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

    pub fn process_pair(&self, chain1: u64, chain2: u64) -> ChainRequestResult<()>{
        // Processes between two chains.
        // If there is an existing pending tx, for this pair, it will wait until the pending is
        // completed or timed out.
        // Nonce management? :: V1. No special nonce management
        //                      V2. TODO: record and re-use the nonce to ensure controlled timeouts
        let live_txs = self.pending_transactions(chain1)?; // TODO: Consider having separate config per pair
        if live_txs.len() > 0 {
            log::info!("There are already {} pending transactions. Ignoring this round",
                live_txs.len());
            return Ok(());
        }
        let client1: &QuantumPortalClient = &self.clients[self.find_client_idx(chain1)];
        let client2: &QuantumPortalClient = &self.clients[self.find_client_idx(chain2)];
        let now = client1.now;
        let fin_tx = client2.finalize(chain1)?;
        if fin_tx.is_some() {
            // Save tx
            // MineTransaction(chain, remote_chain, timestamp, tx_id)
            self.save_tx(
                PendingTransaction::FinalizeTransaction(
                    chain1, now, fin_tx.unwrap()
                ))?
        } else {
            // Save tx
            let mine_tx = client2.mine(chain1, chain2)?;
            if mine_tx.is_some() {
                self.save_tx(
                    PendingTransaction::MineTransaction(
                        chain2, chain1, now, mine_tx.unwrap()
                    ))?
            }
        }
        Ok(())
    }

    fn save_tx(&self, tx: PendingTransaction) -> ChainRequestResult<()> {
        let key = Self::storage_key(&tx);
        PendingTransactions::<T>::insert(
            key,
            tx
        );
        Ok(())
    }

    fn pending_transactions(&self, chain_id: u64) -> ChainRequestResult<Vec<PendingTransaction>> {
        let stored_pending_transactions = self.stored_pending_transactions(chain_id)?;
        Ok(stored_pending_transactions.into_iter().filter(
            |t| self.is_tx_pending(t).unwrap() // TODO: No unwrap here.
        ).collect())
    }

    fn stored_pending_transactions(&self, chain_id: u64) -> ChainRequestResult<Vec<PendingTransaction>> {
        let rv = PendingTransactions::<T>::try_get(chain_id);
        Ok(match rv {
            Err(e) => {
                log::info!("Error stored_pending_transactions {:?}", e);
                Vec::new()
            },
            Ok(v) => vec![v],
        })
    }

    fn remove_transaction_from_db(&self, t: &PendingTransaction) -> ChainRequestResult<()> {
        let key = Self::storage_key(t);
        PendingTransactions::<T>::remove(key);
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

    fn storage_key(tx: &PendingTransaction) -> u64 {
        match tx {
            PendingTransaction::MineTransaction(c, _, _, _) => c,
            PendingTransaction::FinalizeTransaction(c, _, _) => c,
            PendingTransaction::None => panic!("tx is none. Cannot save"),
        }.clone()
    }
}