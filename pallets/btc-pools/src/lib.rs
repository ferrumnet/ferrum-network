#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/reference/frame-pallets/>
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;
pub use weights::*;

//pub mod offchain;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::{dispatch::Vec, pallet_prelude::*};
    use frame_system::pallet_prelude::*;
    use scale_info::prelude::vec;

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
    pub type CurrentPoolAddress<T> = StorageValue<_, Vec<u8>>;

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

    /// Current pending transactions
    #[pallet::storage]
    #[pallet::getter(fn pending_transactions)]
    pub type PendingTransactions<T> = StorageMap<_, Blake2_128Concat, Vec<u8>, Vec<Vec<u8>>>;

    // Pallets use events to inform users when important changes are made.
    // https://docs.substrate.io/main-docs/build/events-errors/
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        WithdrawalSubmitted {
            address: Vec<u8>,
            amount: u32,
        },
        TransactionSubmitted {
            address: Vec<u8>,
            amount: u32,
            hash: Vec<u8>,
        },
        TransactionSignatureSubmitted {
            hash: Vec<u8>,
            signature: Vec<u8>,
        },
        TransactionProcessed {
            hash: Vec<u8>,
        },
    }

    // Errors inform users that something went wrong.
    #[pallet::error]
    pub enum Error<T> {
        /// Error names should be descriptive.
        NoneValue,
        /// Errors should have helpful documentation associated with them.
        StorageOverflow,
    }

    // #[pallet::hooks]
    // impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
    //     fn offchain_worker(block_number: BlockNumberFor<T>) {
    //         log::info!("OffchainWorker : Start Execution");
    //         log::info!("Reading configuration from storage");

    //         let mut lock = StorageLock::<Time>::new(OFFCHAIN_SIGNER_CONFIG_PREFIX);
    //         if let Ok(_guard) = lock.try_lock() {
    //             if let Err(e) = Self::offchain::btc_offchain_handler() {
    //                 log::warn!(
    //                     "Offchain worker failed to execute at block {:?} with error : {:?}",
    //                     now,
    //                     e,
    //                 )
    //             }
    //         }
    //     }

    //     log::info!("OffchainWorker : End Execution");
    // }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::do_something())]
        pub fn submit_withdrawal_request(
            origin: OriginFor<T>,
            address: Vec<u8>,
            amount: u32,
        ) -> DispatchResult {
            // TODO : Ensure the caller is allowed to submit withdrawals
            let who = ensure_signed(origin)?;

            // Update storage.
            <PendingWithdrawals<T>>::insert(address.clone(), amount);

            // Emit an event.
            Self::deposit_event(Event::WithdrawalSubmitted { address, amount });
            // Return a successful DispatchResultWithPostInfo
            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::do_something())]
        pub fn submit_transaction(
            origin: OriginFor<T>,
            address: Vec<u8>,
            amount: u32,
            hash: Vec<u8>,
        ) -> DispatchResult {
            // TODO : Ensure the caller is allowed to submit withdrawals
            let who = ensure_signed(origin)?;

            // Update storage.
            <PendingWithdrawals<T>>::remove(address.clone());

            // add to unsigned transaction
            let signatures: Vec<Vec<u8>> = Default::default();
            PendingTransactions::<T>::insert(hash.clone(), signatures);
            <PendingWithdrawals<T>>::remove(address.clone());

            // Emit an event.
            Self::deposit_event(Event::TransactionSubmitted {
                address,
                amount,
                hash,
            });
            // Return a successful DispatchResultWithPostInfo
            Ok(())
        }

        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::do_something())]
        pub fn submit_transaction_signature(
            origin: OriginFor<T>,
            hash: Vec<u8>,
            signature: Vec<u8>,
        ) -> DispatchResult {
            // TODO : Ensure the caller is allowed to submit withdrawals
            let who = ensure_signed(origin)?;

            // Update storage.
            PendingTransactions::<T>::try_mutate(hash.clone(), |signatures| -> DispatchResult {
                let mut default = vec![];
                let signatures = signatures.as_mut().unwrap_or(&mut default);
                signatures.push(signature.clone());

                Self::deposit_event(Event::TransactionSignatureSubmitted {
                    hash: hash.clone(),
                    signature,
                });

                // if above threshold, complete
                if signatures.len() as u32 >= CurrentPoolThreshold::<T>::get() {
                    Self::deposit_event(Event::TransactionProcessed { hash });
                }

                Ok(())
            })
        }
    }
}
