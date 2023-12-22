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
#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/reference/frame-pallets/>
pub use pallet::*;

use codec::{Decode, Encode};
use ferrum_primitives::{OFFCHAIN_SIGNER_CONFIG_KEY, OFFCHAIN_SIGNER_CONFIG_PREFIX};
use serde::{Deserialize, Serialize};
use sp_runtime::offchain::{
	storage::StorageValueRef,
	storage_lock::{StorageLock, Time},
};
use sp_std::collections::btree_map::BTreeMap;
pub use weights::*;
pub mod offchain;
use offchain::types::BTCConfig;

#[derive(
	Clone,
	Eq,
	PartialEq,
	Decode,
	Encode,
	Debug,
	Serialize,
	Deserialize,
	scale_info::TypeInfo,
	Default,
)]
pub struct TransactionDetails {
	pub signatures: SignatureMap,
	pub recipient: Vec<u8>,
	pub amount: u32,
}

pub type SignatureMap = BTreeMap<Vec<u8>, Vec<u8>>;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use scale_info::prelude::{vec, vec::Vec};

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Type representing the weight of this pallet
		type WeightInfo: WeightInfo;
	}

	// The pallet's runtime storage items.
	// https://docs.substrate.io/main-docs/build/runtime-storage/
	#[pallet::storage]
	#[pallet::getter(fn current_pool_address)]
	pub type CurrentPoolAddress<T> = StorageValue<_, Vec<u8>, ValueQuery>;

	#[pallet::type_value]
	pub fn DefaultThreshold<T: Config>() -> u32 {
		2u32
	}

	#[pallet::storage]
	#[pallet::getter(fn current_pool_threshold)]
	pub type CurrentPoolThreshold<T> = StorageValue<_, u32, ValueQuery, DefaultThreshold<T>>;

	/// Current pending withdrawals
	#[pallet::storage]
	#[pallet::getter(fn pending_withdrawals)]
	pub type PendingWithdrawals<T> = StorageMap<_, Blake2_128Concat, Vec<u8>, u32>;

	// Registered BOS validators
	#[pallet::storage]
	#[pallet::getter(fn registered_validators)]
	pub type RegisteredValidators<T> =
		StorageMap<_, Blake2_128Concat, <T as frame_system::Config>::AccountId, Vec<u8>>;

	/// Current quorom
	#[pallet::storage]
	#[pallet::getter(fn current_quorom)]
	pub type CurrentQuorom<T> = StorageValue<_, Blake2_128Concat, Vec<Vec<u8>>>;

	/// Current signing queue
	// TODO : make a actual queue, we should be able to sign in parallel
	#[pallet::storage]
	#[pallet::getter(fn signing_queue)]
	pub type SigningQueue<T> = StorageValue<_, Blake2_128Concat, Vec<u8>>;

	/// Current signatures for data in signing queue
	#[pallet::storage]
	#[pallet::getter(fn signing_queue)]
	pub type Signatures<T> = StorageValue<_, Blake2_128Concat, Vec<Vec<u8>>>;

	/// Current pub key
	#[pallet::storage]
	#[pallet::getter(fn current_pub_key)]
	pub type CurrentPubKey<T> = StorageValue<_, Blake2_128Concat, Vec<u8>>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		WithdrawalSubmitted { address: Vec<u8>, amount: u32 },
		TransactionSubmitted { address: Vec<u8>, amount: u32, hash: Vec<u8> },
		TransactionSignatureSubmitted { hash: Vec<u8>, signature: Vec<u8> },
		TransactionProcessed { hash: Vec<u8> },
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Error names should be descriptive.
		NoneValue,
		/// Errors should have helpful documentation associated with them.
		StorageOverflow,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn offchain_worker(block_number: BlockNumberFor<T>) {
			log::info!("TresholdValidator OffchainWorker : Start Execution");
			log::info!("Reading configuration from storage");

			let mut lock = StorageLock::<Time>::new(OFFCHAIN_SIGNER_CONFIG_PREFIX);
			if let Ok(_guard) = lock.try_lock() {
				let network_config = StorageValueRef::persistent(OFFCHAIN_SIGNER_CONFIG_KEY);

				let decoded_config = network_config.get::<BTCConfig>();
				log::info!("TresholdValidator : Decoded config is {:?}", decoded_config);

				if let Err(_e) = decoded_config {
					log::info!("Error reading configuration, exiting offchain worker");
					return
				}

				if let Ok(None) = decoded_config {
					log::info!("Configuration not found, exiting offchain worker");
					return
				}

				if let Ok(Some(config)) = decoded_config {
					let now = block_number.try_into().map_or(0_u64, |f| f);
					log::info!("Current block: {:?}", block_number);
					if let Err(e) = Self::execute_threshold_offchain_worker(now, config) {
						log::warn!(
                            "TresholdValidator : Offchain worker failed to execute at block {:?} with error : {:?}",
                            now,
                            e,
                        )
					}
				}
			}

			log::info!("TresholdValidator : OffchainWorker : End Execution");
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::do_something())]
		pub fn register_validator(origin: OriginFor<T>, pub_key: Vec<u8>) -> DispatchResult {
			// TODO : Ensure the caller is actually allowed to be a validator
			// We need to make sure that no-one is skipping the EVM precompile
			// Solution : initial whitelist of those allowed to calls
			// Needs to have a list of addresses that can whitelisted, can be updated by sudo
			// Solution : Extrinsic should only be called by runtime proxy
			let who = ensure_signed(origin)?;
			RegisteredValidators::<T>::insert(who, pub_key);

			Ok(())
		}

		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::do_something())]
		pub fn generate_new_key(origin: OriginFor<T>) -> DispatchResult {
			// TODO : Remove after testing
			let who = ensure_signed(origin)?;
			Self::generate_new_key();
			Ok(())
		}

		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::do_something())]
		pub fn add_new_data_to_sign(origin: OriginFor<T>, data: Vec<u8>) -> DispatchResult {
			// TODO : Remove after testing
			let who = ensure_signed(origin)?;
			SigningQueue::<T>::set(data);
			Ok(())
		}
	}
}
