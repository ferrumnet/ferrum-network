use sp_core::H256;
use sp_std::prelude::*;
use crate::chain_utils::{ChainRequestError, ChainRequestResult};
use crate::quantum_portal_client::QuantumPortalClient;

pub enum  PendingTransaction {
    MineTransaction(u64, u64, H256),
    FinalizeTransaction(u64, H256),
}

pub struct QuantumPortalService {
    pub clients: Vec<QuantumPortalClient>,
}

impl QuantumPortalService {
    pub fn new(clients: Vec<QuantumPortalClient>) -> Self {
        QuantumPortalService {
            clients,
        }
    }

    pub fn process_pair(&self, chain1: u64, chain2: u64) -> ChainRequestResult<()>{
        // Processes between two chains.
        // If there is an existing pending tx, for this pair, it will wait until the pending is
        // completed or timed out.
        // Nonce management? :: V1. No special nonce management
        //                      V2. TODO: record and re-use the nonce to ensure controlled timeouts
        let live_txs = self.pending_transactions()?; // TODO: Consider having separate config per pair
        if live_txs.len() > 0 {
            log::info!("There are already {} pending transactions. Ignoring this round",
                live_txs.len());
            return Ok(());
        }
        let client1: &QuantumPortalClient = &self.clients[self.find_client_idx(chain1)];
        let client2: &QuantumPortalClient = &self.clients[self.find_client_idx(chain2)];
        if !client2.finalize(chain1)? {
            client2.mine(chain1, chain2)?;
        }
        Ok(())
    }

    fn pending_transactions(&self) -> ChainRequestResult<Vec<PendingTransaction>> {
        let stored_pending_transactions = self.stored_pending_transactions()?;
        Ok(stored_pending_transactions.into_iter().filter(
            |t| self.is_tx_pending(t).unwrap() // TODO: No unwrap here.
        ).collect())
    }

    fn stored_pending_transactions(&self) -> ChainRequestResult<Vec<PendingTransaction>> {
        Err(b"Not implemented".as_slice().into())
    }

    fn is_tx_pending(&self, t: &PendingTransaction) -> ChainRequestResult<bool> {
        // Check if the tx is still pending
        // If so, return true.
        // otherwise. Update storage and remove the tx.
        // then return false
        Ok(true)
    }

    fn find_client_idx(&self, chain_id: u64) -> usize {
        return  0; // Find the relevant chain id.
    }
}