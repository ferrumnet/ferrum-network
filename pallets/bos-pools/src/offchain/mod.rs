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
use super::*;
use crate::PendingWithdrawals;
use electrum_client::{Client, ElectrumApi};
use sp_runtime::traits::Zero;
pub mod types;
use crate::offchain::btc_client::BTCClientSignature;
use sp_core::sr25519;
pub use types::*;

mod btc_client;

impl<T: Config> Pallet<T> {
	pub fn execute_btc_pools_offchain_worker(
		block_number: u64,
		btc_config: types::BTCConfig,
	) -> OffchainResult<()> {
		// first handle any pending withdrawal requests
		let pending_withdrawals = PendingWithdrawals::<T>::iter();
		let pending_withdrawals = pending_withdrawals.collect::<Vec<_>>();

		log::info!("BTC Pools : Pending withdrawals is {:?}", pending_withdrawals);

		for (recipient, amount) in pending_withdrawals {
			let result = Self::handle_withdrawal_request(recipient.clone(), amount);
			log::info!(
				"BTC Pools : Withdrawal request for recipient : {:?}, processed {:?}",
				recipient,
				result
			);
		}

		// check for any pending transactions
		let pending_transactions = PendingTransactions::<T>::iter();
		let pending_transactions = pending_transactions.collect::<Vec<_>>();

		log::info!("BTC Pools : Pending transactions is {:?}", pending_transactions);

		for (hash, details) in pending_transactions {
			let result =
				Self::handle_pending_transaction(hash.clone(), details, btc_config.clone());
			log::info!(
				"BTC Pools : Pending transaction for hash : {:?}, processed {:?}",
				hash,
				result
			);
		}

		Ok(())
	}

	pub fn handle_withdrawal_request(recipient: Vec<u8>, amount: u32) -> OffchainResult<()> {
		// TODO : We should have some queue here, now everyone tries to submit and only first one
		// works let prepare a transaction for this withdrawal request
		let current_pool_address = CurrentPoolAddress::<T>::get();

		// pick all the known validators
		let validators = RegisteredValidators::<T>::iter();
		let validators = validators.map(|x| x.1).collect::<Vec<_>>();

		if validators.is_empty() {
			panic!("No BTC validators found!");
		}

		let transaction = btc_client::BTCClient::generate_transaction_from_withdrawal_request(
			recipient,
			amount,
			validators,
			current_pool_address,
		)
		.unwrap()
		.txid()
		.as_ref()
		.to_vec();

		// push transaction to storage
		PendingTransactions::<T>::insert::<Vec<u8>, TransactionDetails>(
			transaction,
			Default::default(),
		);

		Ok(())
	}

	pub fn handle_pending_transaction(
		hash: Vec<u8>,
		details: TransactionDetails,
		btc_config: types::BTCConfig,
	) -> OffchainResult<()> {
		let current_pool_address = CurrentPoolAddress::<T>::get();

		let mut key = [0u8; 32];
		key[..32].copy_from_slice(&btc_config.signer_public_key);
		let signer_address = sr25519::Public(key);
		let signer = BTCClientSignature::from(signer_address);

		// if we have not already signed, sign the transaction
		if details.signatures.get(&signer.from.to_vec()).is_none() {
			// sign transaction using our key
			// TODO : Reconstruct the transaction and ensure that hash is valid
			let signature = signer.sign(&hash).expect("Signing Failed!!");

			// push the signature to storage
			let _ =
				PendingTransactions::<T>::try_mutate(hash.clone(), |details| -> Result<(), ()> {
					let mut default = TransactionDetails::default();
					let mut signatures = &mut details.as_mut().unwrap_or(&mut default).signatures;
					signatures.insert(signer_address.to_vec(), signature.0.to_vec());

					Self::deposit_event(Event::TransactionSignatureSubmitted {
						hash: hash.clone(),
						signature: signature.0.to_vec(),
					});

					// if above threshold, complete
					if signatures.len() as u32 >= CurrentPoolThreshold::<T>::get() {
						Self::deposit_event(Event::TransactionProcessed { hash });
					}

					Ok(())
				});

			return Ok(())
		}
		// if we have signed the transaction, if the threshold is reached, we broadcast to chain
		else {
			// TODO : We should have some queue here, now everyone tries to submit and only first
			// one works
			if details.signatures.len() as u32 >= CurrentPoolThreshold::<T>::get() {
				// threshold reached, we can broadcast
				let transaction = btc_client::BTCClient::broadcast_completed_transaction(
					hash,
					details.recipient,
					details.amount,
					details.signatures,
					current_pool_address,
				)
				.unwrap();
			}
		}

		Ok(())
	}
}
