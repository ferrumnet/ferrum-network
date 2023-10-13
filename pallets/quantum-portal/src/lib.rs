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

pub use pallet::*;
mod chain_queries;
mod chain_utils;
mod contract_client;
mod eip_712_utils;
pub mod qp_types;
mod quantum_portal_client;
pub mod quantum_portal_service;

#[frame_support::pallet]
pub mod pallet {
    // Re-import necessary modules for pallet.
    use crate::{
        chain_utils::{ChainRequestError, ChainUtils},
        contract_client::{ContractClient, ContractClientSignature},
        qp_types,
        qp_types::{QpConfig, QpNetworkItem, Role},
        quantum_portal_client::QuantumPortalClient,
        quantum_portal_service::QuantumPortalService,
    };
    // Re-import necessary items from core and other external crates.
    use crate::qp_types::MAX_PAIRS_TO_MINE;
    use core::convert::TryInto;
    use ferrum_primitives::{OFFCHAIN_SIGNER_CONFIG_KEY, OFFCHAIN_SIGNER_CONFIG_PREFIX};
    use frame_support::pallet_prelude::*;
    use frame_support::traits::UnixTime;
    use frame_system::pallet_prelude::*;
    use sp_runtime::offchain::storage::StorageValueRef;
    use sp_runtime::offchain::storage_lock::StorageLock;
    use sp_runtime::offchain::storage_lock::Time;
    use sp_std::{prelude::*, str};

    #[pallet::config]
    pub trait Config:
        frame_system::offchain::CreateSignedTransaction<Call<Self>> + frame_system::Config
    {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        type RuntimeCall: From<frame_system::Call<Self>>;
        type Timestamp: UnixTime;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    #[pallet::event]
    pub enum Event<T: Config> {}

    #[pallet::error]
    pub enum Error<T> {}

    pub enum OffchainErr {
        RPCError(ChainRequestError),
        FailedSigning,
    }

    impl sp_std::fmt::Debug for OffchainErr {
        fn fmt(&self, fmt: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
            match *self {
                OffchainErr::FailedSigning => write!(fmt, "Unable to sign transaction"),
                OffchainErr::RPCError(ref error) => write!(fmt, "RPC error : {error:?}"),
            }
        }
    }

    pub type OffchainResult<A> = Result<A, OffchainErr>;

    impl<T: Config> Pallet<T> {
        pub fn configure_network(
            block_number: u64,
            network_item: QpNetworkItem,
            signer_public_key: Vec<u8>,
        ) -> QuantumPortalClient<T> {
            let rpc_endpoint = network_item.url;
            let id = network_item.id;

            let signer = ChainUtils::hex_to_ecdsa_pub_key(&signer_public_key[..]);
            let gateway_contract =
                ChainUtils::hex_to_address(&network_item.gateway_contract_address[..]);
            let client = ContractClient::new(rpc_endpoint, &gateway_contract, id);
            QuantumPortalClient::new(
                client,
                ContractClientSignature::from(signer),
                sp_io::offchain::timestamp().unix_millis(),
                block_number,
            )
        }

        pub fn test_qp(
            block_number: u64,
            qp_config_item: qp_types::QpConfig,
        ) -> OffchainResult<()> {
            let client_vec: Vec<_> = qp_config_item
                .network_vec
                .into_iter()
                .map(|item| {
                    Self::configure_network(
                        block_number,
                        item,
                        qp_config_item.signer_public_key.clone(),
                    )
                })
                .collect();

            let svc = QuantumPortalService::<T>::new(client_vec);
            let _res: Vec<_> = qp_config_item
                .pair_vec
                .into_iter()
                .map(|(remote_chain, local_chain)| {
                    let proces_pair_res = svc.process_pair_with_lock(
                        remote_chain,
                        local_chain,
                        qp_config_item.role.clone(),
                    );
                    if let Err(e) = proces_pair_res {
                        log::warn!("Error : {:?}", e,)
                    }
                })
                .collect();
            Ok(())
        }
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn offchain_worker(block_number: BlockNumberFor<T>) {
            log::info!("OffchainWorker : Start Execution");
            log::info!("Reading configuration from storage");

            let mut lock = StorageLock::<Time>::new(OFFCHAIN_SIGNER_CONFIG_PREFIX);
            if let Ok(_guard) = lock.try_lock() {
                let network_config = StorageValueRef::persistent(OFFCHAIN_SIGNER_CONFIG_KEY);

                let decoded_config = network_config.get::<QpConfig>();
                log::info!("Decoded config is {:?}", decoded_config);

                if let Err(_e) = decoded_config {
                    log::info!("Error reading configuration, exiting offchain worker");
                    return;
                }

                if let Ok(None) = decoded_config {
                    log::info!("Configuration not found, exiting offchain worker");
                    return;
                }

                if let Ok(Some(config)) = decoded_config {
                    let expected_role = config.role.clone();

                    if expected_role == Role::None {
                        log::info!("Not a miner or finalizer, exiting offchain worker");
                        return;
                    }

                    // ensure pairs configured are within limit
                    if config.pair_vec.len() > MAX_PAIRS_TO_MINE {
                        log::info!("Too many pairs configured, this may lead to performance issues, maximum allowed is {:?}, Exiting", MAX_PAIRS_TO_MINE);
                        return;
                    }

                    let now = block_number.try_into().map_or(0_u64, |f| f);
                    log::info!("Current block: {:?}", block_number);
                    if let Err(e) = Self::test_qp(now, config) {
                        log::warn!(
                            "Offchain worker failed to execute at block {:?} with error : {:?}",
                            now,
                            e,
                        )
                    }
                }
            }

            log::info!("OffchainWorker : End Execution");
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {}
}
