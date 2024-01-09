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
//! Parachain Service and ServiceFactory implementation.
#![allow(clippy::type_complexity)]
use crate::cli::Cli;
use cumulus_client_cli::CollatorOptions;
use cumulus_client_consensus_aura::{AuraConsensus, BuildAuraConsensusParams, SlotProportion};
use cumulus_client_consensus_common::{ParachainBlockImport, ParachainConsensus};
use cumulus_client_network::BlockAnnounceValidator;
use cumulus_client_service::{
	prepare_node_config, start_collator, start_full_node, CollatorSybilResistance,
	StartCollatorParams, StartFullNodeParams,
};
use futures::FutureExt;
use sc_network::config::FullNetworkConfiguration;
use sc_network_sync::SyncingService;
use sc_transaction_pool_api::OffchainTransactionPoolFactory;

use sp_application_crypto::sp_core::offchain::{OffchainStorage, STORAGE_PREFIX};
// Local Runtime Types
use codec::Encode;
use futures::StreamExt;
// Substrate
use crate::{
	config::read_config_from_file,
	primitives::{AccountId, Balance, Block, Hash},
};
use cumulus_primitives_core::{relay_chain::Nonce, ParaId};
use cumulus_relay_chain_inprocess_interface::build_inprocess_relay_chain;
use cumulus_relay_chain_interface::{RelayChainError, RelayChainInterface, RelayChainResult};
use cumulus_relay_chain_minimal_node::build_minimal_relay_chain_node_with_rpc;
use fc_consensus::FrontierBlockImport;
use fc_rpc_core::types::{FeeHistoryCache, FilterPool};
use ferrum_primitives::OFFCHAIN_SIGNER_CONFIG_KEY;
use polkadot_service::CollatorPair;

use sc_client_api::{Backend, BlockchainEvents, ExecutorProvider};
use sc_consensus::ImportQueue;
use sc_executor::NativeElseWasmExecutor;
use sc_network::{NetworkBlock, NetworkService};
use sc_service::{Configuration, PartialComponents, TFullBackend, TFullClient, TaskManager};
use sc_telemetry::{Telemetry, TelemetryHandle, TelemetryWorker, TelemetryWorkerHandle};
use sp_api::ConstructRuntimeApi;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_keystore::KeystorePtr;
use sp_runtime::traits::BlakeTwo256;
use std::{collections::BTreeMap, sync::Arc, time::Duration};
use substrate_prometheus_endpoint::Registry;

/// Ferrum kusama runtime executor.
pub mod kusama {
	pub use ferrum_runtime::RuntimeApi;

	/// Kusama runtime executor.
	pub struct Executor;
	impl sc_executor::NativeExecutionDispatch for Executor {
		fn dispatch(method: &str, data: &[u8]) -> Option<Vec<u8>> {
			ferrum_runtime::api::dispatch(method, data)
		}

		fn native_version() -> sc_executor::NativeVersion {
			ferrum_runtime::native_version()
		}

		type ExtendHostFunctions = ();
	}
}

/// Ferrum testnet runtime executor.
pub mod ferrum_testnet {
	pub use ferrum_testnet_runtime::RuntimeApi;

	/// Testnet runtime executor.
	pub struct Executor;
	impl sc_executor::NativeExecutionDispatch for Executor {
		fn dispatch(method: &str, data: &[u8]) -> Option<Vec<u8>> {
			ferrum_testnet_runtime::api::dispatch(method, data)
		}

		fn native_version() -> sc_executor::NativeVersion {
			ferrum_testnet_runtime::native_version()
		}

		type ExtendHostFunctions = ();
	}
}

type MaybeSelectChain = Option<sc_consensus::LongestChain<TFullBackend<Block>, Block>>;

/// Starts a `ServiceBuilder` for a full service.
///
/// Use this macro if you don't actually need the full service, but just the builder in order to
/// be able to perform chain operations.
pub fn new_partial<RuntimeApi, Executor, BIQ>(
	config: &Configuration,
	build_import_queue: BIQ,
	cli: &Cli,
) -> Result<
	PartialComponents<
		TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>,
		TFullBackend<Block>,
		MaybeSelectChain,
		sc_consensus::DefaultImportQueue<Block>,
		sc_transaction_pool::FullPool<
			Block,
			TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>,
		>,
		(
			ParachainBlockImport<
				Block,
				FrontierBlockImport<
					Block,
					Arc<TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>,
					TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>,
				>,
				TFullBackend<Block>,
			>,
			Option<Telemetry>,
			Option<TelemetryWorkerHandle>,
			Arc<fc_db::Backend<Block>>,
		),
	>,
	sc_service::Error,
>
where
	RuntimeApi: ConstructRuntimeApi<Block, TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>
		+ Send
		+ Sync
		+ 'static,
	RuntimeApi::RuntimeApi: sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
		+ sp_api::Metadata<Block>
		+ sp_session::SessionKeys<Block>
		+ sp_api::ApiExt<Block>
		+ sp_offchain::OffchainWorkerApi<Block>
		+ sp_block_builder::BlockBuilder<Block>
		+ fp_rpc::EthereumRuntimeRPCApi<Block>,
	sc_client_api::StateBackendFor<TFullBackend<Block>, Block>: sp_api::StateBackend<BlakeTwo256>,
	Executor: sc_executor::NativeExecutionDispatch + 'static,
	BIQ: FnOnce(
		Arc<TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>,
		ParachainBlockImport<
			Block,
			FrontierBlockImport<
				Block,
				Arc<TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>,
				TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>,
			>,
			TFullBackend<Block>,
		>,
		&Configuration,
		Option<TelemetryHandle>,
		&TaskManager,
	) -> Result<sc_consensus::DefaultImportQueue<Block>, sc_service::Error>,
{
	let telemetry = config
		.telemetry_endpoints
		.clone()
		.filter(|x| !x.is_empty())
		.map(|endpoints| -> Result<_, sc_telemetry::Error> {
			let worker = TelemetryWorker::new(16)?;
			let telemetry = worker.handle().new_telemetry(endpoints);
			Ok((worker, telemetry))
		})
		.transpose()?;

	let executor = sc_executor::NativeElseWasmExecutor::<Executor>::new(
		config.wasm_method,
		config.default_heap_pages,
		config.max_runtime_instances,
		config.runtime_cache_size,
	);

	let (client, backend, keystore_container, task_manager) =
		sc_service::new_full_parts::<Block, RuntimeApi, _>(
			config,
			telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
			executor,
		)?;
	let client = Arc::new(client);

	let telemetry_worker_handle = telemetry.as_ref().map(|(worker, _)| worker.handle());

	let telemetry = telemetry.map(|(worker, telemetry)| {
		task_manager.spawn_handle().spawn("telemetry", None, worker.run());
		telemetry
	});

	let transaction_pool = sc_transaction_pool::BasicPool::new_full(
		config.transaction_pool.clone(),
		config.role.is_authority().into(),
		config.prometheus_registry(),
		task_manager.spawn_essential_handle(),
		client.clone(),
	);

	let keystore = keystore_container.keystore();

	let frontier_backend = crate::rpc::open_frontier_backend(client.clone(), config)?;
	let frontier_block_import = FrontierBlockImport::new(client.clone(), client.clone());

	let block_import: ParachainBlockImport<_, _, _> =
		ParachainBlockImport::new(frontier_block_import, backend.clone());

	let import_queue = build_import_queue(
		client.clone(),
		block_import.clone(),
		config,
		telemetry.as_ref().map(|telemetry| telemetry.handle()),
		&task_manager,
	)?;

	let params = PartialComponents {
		backend: backend.clone(),
		client,
		import_queue,
		keystore_container,
		task_manager,
		transaction_pool,
		select_chain: Some(sc_consensus::LongestChain::new(backend.clone())),
		other: (block_import, telemetry, telemetry_worker_handle, frontier_backend),
	};

	Ok(params)
}

async fn build_relay_chain_interface(
	polkadot_config: Configuration,
	parachain_config: &Configuration,
	telemetry_worker_handle: Option<TelemetryWorkerHandle>,
	task_manager: &mut TaskManager,
	collator_options: CollatorOptions,
	hwbench: Option<sc_sysinfo::HwBench>,
) -> RelayChainResult<(Arc<(dyn RelayChainInterface + 'static)>, Option<CollatorPair>)> {
	if let cumulus_client_cli::RelayChainMode::ExternalRpc(rpc_target_urls) =
		collator_options.relay_chain_mode
	{
		build_minimal_relay_chain_node_with_rpc(polkadot_config, task_manager, rpc_target_urls)
			.await
	} else {
		build_inprocess_relay_chain(
			polkadot_config,
			parachain_config,
			telemetry_worker_handle,
			task_manager,
			None,
		)
	}
}

/// Start a node with the given parachain `Configuration` and relay chain `Configuration`.
///
/// This is the actual implementation that is abstract over the executor and the runtime api.
#[sc_tracing::logging::prefix_logs_with("Parachain")]
async fn start_node_impl<RuntimeApi, Executor, BIQ, BIC>(
	parachain_config: Configuration,
	polkadot_config: Configuration,
	collator_options: CollatorOptions,
	id: ParaId,
	enable_evm_rpc: bool,
	build_import_queue: BIQ,
	build_consensus: BIC,
	hwbench: Option<sc_sysinfo::HwBench>,
	cli: &Cli,
) -> sc_service::error::Result<(
	TaskManager,
	Arc<TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>,
)>
where
	RuntimeApi: ConstructRuntimeApi<Block, TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>
		+ Send
		+ Sync
		+ 'static,
	RuntimeApi::RuntimeApi: sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
		+ sp_api::Metadata<Block>
		+ sp_session::SessionKeys<Block>
		+ sp_api::ApiExt<Block>
		+ sp_offchain::OffchainWorkerApi<Block>
		+ sp_block_builder::BlockBuilder<Block>
		+ substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Nonce>
		+ pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>
		+ fp_rpc::EthereumRuntimeRPCApi<Block>
		+ fp_rpc::ConvertTransactionRuntimeApi<Block>
		+ cumulus_primitives_core::CollectCollationInfo<Block>,
	sc_client_api::StateBackendFor<TFullBackend<Block>, Block>: sp_api::StateBackend<BlakeTwo256>,
	Executor: sc_executor::NativeExecutionDispatch + 'static,
	BIQ: FnOnce(
		Arc<TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>,
		ParachainBlockImport<
			Block,
			FrontierBlockImport<
				Block,
				Arc<TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>,
				TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>,
			>,
			TFullBackend<Block>,
		>,
		&Configuration,
		Option<TelemetryHandle>,
		&TaskManager,
	) -> Result<sc_consensus::DefaultImportQueue<Block>, sc_service::Error>,
	BIC: FnOnce(
		Arc<TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>,
		Arc<sc_client_db::Backend<Block>>,
		ParachainBlockImport<
			Block,
			FrontierBlockImport<
				Block,
				Arc<TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>,
				TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>,
			>,
			TFullBackend<Block>,
		>,
		Option<&Registry>,
		Option<TelemetryHandle>,
		&TaskManager,
		Arc<dyn RelayChainInterface>,
		Arc<
			sc_transaction_pool::FullPool<
				Block,
				TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>,
			>,
		>,
		Arc<SyncingService<Block>>,
		KeystorePtr,
		bool,
	) -> Result<Box<dyn ParachainConsensus<Block>>, sc_service::Error>,
{
	let parachain_config = prepare_node_config(parachain_config);

	let params =
		new_partial::<RuntimeApi, Executor, BIQ>(&parachain_config, build_import_queue, cli)?;
	let (block_import, mut telemetry, telemetry_worker_handle, frontier_backend) = params.other;

	let client = params.client.clone();
	let backend = params.backend.clone();

	let mut task_manager = params.task_manager;

	let (relay_chain_interface, collator_key) = build_relay_chain_interface(
		polkadot_config,
		&parachain_config,
		telemetry_worker_handle,
		&mut task_manager,
		collator_options.clone(),
		hwbench.clone(),
	)
	.await
	.map_err(|e| sc_service::Error::Application(Box::new(e) as Box<_>))?;

	let force_authoring = parachain_config.force_authoring;
	let is_authority = parachain_config.role.is_authority();
	let prometheus_registry = parachain_config.prometheus_registry().cloned();
	let transaction_pool = params.transaction_pool.clone();
	let import_queue_service = params.import_queue.service();
	let net_config = FullNetworkConfiguration::new(&parachain_config.network);

	let (network, system_rpc_tx, tx_handler_controller, start_network, sync_service) =
		cumulus_client_service::build_network(cumulus_client_service::BuildNetworkParams {
			parachain_config: &parachain_config,
			client: client.clone(),
			transaction_pool: transaction_pool.clone(),
			spawn_handle: task_manager.spawn_handle(),
			import_queue: params.import_queue,
			para_id: id,
			relay_chain_interface: relay_chain_interface.clone(),
			net_config,
			sybil_resistance_level: CollatorSybilResistance::Resistant,
		})
		.await?;

	let filter_pool: FilterPool = Arc::new(std::sync::Mutex::new(BTreeMap::new()));
	let fee_history_cache: FeeHistoryCache = Arc::new(std::sync::Mutex::new(BTreeMap::new()));
	let overrides = crate::rpc::overrides_handle(client.clone());

	if parachain_config.offchain_worker.enabled {
		task_manager.spawn_handle().spawn(
			"offchain-workers-runner",
			"offchain-worker",
			sc_offchain::OffchainWorkers::new(sc_offchain::OffchainWorkerOptions {
				runtime_api_provider: client.clone(),
				is_validator: parachain_config.role.is_authority(),
				keystore: Some(params.keystore_container.keystore()),
				offchain_db: backend.offchain_storage(),
				transaction_pool: Some(OffchainTransactionPoolFactory::new(
					transaction_pool.clone(),
				)),
				network_provider: network.clone(),
				enable_http_requests: true,
				custom_extensions: |_| vec![],
			})
			.run(client.clone(), task_manager.spawn_handle())
			.boxed(),
		);
	}

	if parachain_config.offchain_worker.enabled {
		// only load the config if the config file path is provided from cli
		if let Some(local_path_buf) = cli.config.config_file_path.clone() {
			let mut offchain_storage = backend.offchain_storage().unwrap();

			// read the config file
			let config = read_config_from_file(local_path_buf)
				.expect("Failed to read chainspec config file");

			// Load the configs for the offchain worker to function properly, we read from the file
			// and write to the offchain storage
			offchain_storage.set(
				STORAGE_PREFIX,
				OFFCHAIN_SIGNER_CONFIG_KEY,
				&crate::config::convert(config.networks).encode(),
			);

			println!("QP Configs loaded to offchain storage");
		}

		// just a sanity check to make sure the keystore is populated correctly
		let ecdsa_keys: Vec<sp_core::ecdsa::Public> = sp_keystore::Keystore::ecdsa_public_keys(
			&*keystore,
			ferrum_primitives::OFFCHAIN_SIGNER_KEY_TYPE,
		);
		println!("ECDSA KEYS in keystore {ecdsa_keys:?}");
	}

	// Frontier offchain DB task. Essential.
	// Maps emulated ethereum data to substrate native data.
	spawn_frontier_tasks(
		&task_manager,
		client.clone(),
		backend,
		frontier_backend,
		filter_pool,
		overrides,
		fee_history_cache,
		fee_history_cache_limit,
		sync_service.clone(),
		pubsub_notification_sinks,
	)
	.await;

	// Frontier `EthFilterApi` maintenance. Manages the pool of user-created Filters.
	// Each filter is allowed to stay in the pool for 100 blocks.
	const FILTER_RETAIN_THRESHOLD: u64 = 100;
	task_manager.spawn_essential_handle().spawn(
		"frontier-filter-pool",
		Some("frontier"),
		fc_rpc::EthTask::filter_pool_task(
			client.clone(),
			filter_pool.clone(),
			FILTER_RETAIN_THRESHOLD,
		),
	);

	const FEE_HISTORY_LIMIT: u64 = 2048;
	task_manager.spawn_essential_handle().spawn(
		"frontier-fee-history",
		Some("frontier"),
		fc_rpc::EthTask::fee_history_task(
			client.clone(),
			overrides.clone(),
			fee_history_cache.clone(),
			FEE_HISTORY_LIMIT,
		),
	);

	let block_data_cache = Arc::new(fc_rpc::EthBlockDataCacheTask::new(
		task_manager.spawn_handle(),
		overrides.clone(),
		50,
		50,
		prometheus_registry.clone(),
	));

	let ethapi_cmd = rpc_config.ethapi.clone();
	let tracing_requesters =
		if ethapi_cmd.contains(&EthApi::Debug) || ethapi_cmd.contains(&EthApi::Trace) {
			crate::rpc::tracing::spawn_tracing_tasks(
				&task_manager,
				client.clone(),
				backend.clone(),
				frontier_backend.clone(),
				overrides.clone(),
				&rpc_config,
				prometheus_registry.clone(),
			)
		} else {
			crate::rpc::tracing::RpcRequesters { debug: None, trace: None }
		};

	let rpc_builder = {
		let client = client.clone();
		let network = network.clone();
		let transaction_pool = transaction_pool.clone();

		Box::new(move |deny_unsafe, subscription| {
			let deps = crate::rpc::FullDeps {
				client: client.clone(),
				pool: transaction_pool.clone(),
				graph: transaction_pool.pool().clone(),
				network: network.clone(),
				is_authority,
				deny_unsafe,
				frontier_backend: frontier_backend.clone(),
				filter_pool: filter_pool.clone(),
				fee_history_limit: FEE_HISTORY_LIMIT,
				fee_history_cache: fee_history_cache.clone(),
				block_data_cache: block_data_cache.clone(),
				overrides: overrides.clone(),
				enable_evm_rpc,
				tracing_config: None,
			};

			crate::rpc::create_full(deps, subscription).map_err(Into::into)
		})
	};

	// Spawn basic services.
	sc_service::spawn_tasks(sc_service::SpawnTasksParams {
		rpc_builder: Box::new(rpc_builder),
		client: client.clone(),
		transaction_pool: transaction_pool.clone(),
		task_manager: &mut task_manager,
		config: parachain_config,
		keystore: params.keystore_container.keystore(),
		backend: backend.clone(),
		network: network.clone(),
		sync_service: sync_service.clone(),
		system_rpc_tx,
		tx_handler_controller,
		telemetry: telemetry.as_mut(),
	})?;

	let announce_block = {
		let sync_service = sync_service.clone();
		Arc::new(move |hash, data| sync_service.announce_block(hash, data))
	};

	let relay_chain_slot_duration = Duration::from_secs(6);

	let overseer_handle = relay_chain_interface
		.overseer_handle()
		.map_err(|e| sc_service::Error::Application(Box::new(e)))?;

	if is_authority {
		let parachain_consensus = build_consensus(
			client.clone(),
			backend,
			block_import,
			prometheus_registry.as_ref(),
			telemetry.as_ref().map(|t| t.handle()),
			&task_manager,
			relay_chain_interface.clone(),
			transaction_pool,
			sync_service.clone(),
			params.keystore_container.keystore(),
			force_authoring,
		)?;

		let spawner = task_manager.spawn_handle();

		let params = StartCollatorParams {
			para_id: id,
			block_status: client.clone(),
			announce_block,
			client: client.clone(),
			task_manager: &mut task_manager,
			relay_chain_interface,
			spawner,
			parachain_consensus,
			import_queue: import_queue_service,
			recovery_handle: Box::new(overseer_handle),
			collator_key: collator_key
				.ok_or(sc_service::error::Error::Other("Collator Key is None".to_string()))?,
			relay_chain_slot_duration,
			sync_service,
		};

		start_collator(params).await?;
	} else {
		let params = StartFullNodeParams {
			client: client.clone(),
			announce_block,
			task_manager: &mut task_manager,
			para_id: id,
			relay_chain_interface,
			relay_chain_slot_duration,
			import_queue: import_queue_service,
			recovery_handle: Box::new(overseer_handle),
			sync_service,
		};

		#[allow(deprecated)]
		start_full_node(params)?;
	}

	start_network.start_network();

	Ok((task_manager, client))
}

/// Build the import queue.
pub fn build_import_queue<RuntimeApi, Executor>(
	client: Arc<TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>,
	block_import: ParachainBlockImport<
		Block,
		FrontierBlockImport<
			Block,
			Arc<TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>,
			TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>,
		>,
		TFullBackend<Block>,
	>,
	config: &Configuration,
	telemetry: Option<TelemetryHandle>,
	task_manager: &TaskManager,
) -> Result<sc_consensus::DefaultImportQueue<Block>, sc_service::Error>
where
	RuntimeApi: ConstructRuntimeApi<Block, TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>
		+ Send
		+ Sync
		+ 'static,
	RuntimeApi::RuntimeApi: sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
		+ sp_api::Metadata<Block>
		+ sp_session::SessionKeys<Block>
		+ sp_api::ApiExt<Block>
		+ sp_offchain::OffchainWorkerApi<Block>
		+ sp_block_builder::BlockBuilder<Block>
		+ fp_rpc::EthereumRuntimeRPCApi<Block>
		+ sp_consensus_aura::AuraApi<Block, AuraId>,
	sc_client_api::StateBackendFor<TFullBackend<Block>, Block>: sp_api::StateBackend<BlakeTwo256>,
	Executor: sc_executor::NativeExecutionDispatch + 'static,
{
	let slot_duration = cumulus_client_consensus_aura::slot_duration(&*client)?;

	cumulus_client_consensus_aura::import_queue::<
		sp_consensus_aura::sr25519::AuthorityPair,
		_,
		_,
		_,
		_,
		_,
	>(cumulus_client_consensus_aura::ImportQueueParams {
		block_import,
		client,
		create_inherent_data_providers: move |_, _| async move {
			let timestamp = sp_timestamp::InherentDataProvider::from_system_time();

			let slot =
				sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
					*timestamp,
					slot_duration,
				);

			Ok((slot, timestamp))
		},
		registry: config.prometheus_registry(),
		spawner: &task_manager.spawn_essential_handle(),
		telemetry,
	})
	.map_err(Into::into)
}

/// Start a parachain node for Ferrum Testnet
pub async fn start_testnet_node(
	parachain_config: Configuration,
	polkadot_config: Configuration,
	collator_options: CollatorOptions,
	id: ParaId,
	enable_evm_rpc: bool,
	hwbench: Option<sc_sysinfo::HwBench>,
	cli: &Cli,
) -> sc_service::error::Result<(
	TaskManager,
	Arc<
		TFullClient<
			Block,
			ferrum_testnet::RuntimeApi,
			NativeElseWasmExecutor<ferrum_testnet::Executor>,
		>,
	>,
)> {
	start_node_impl::<ferrum_testnet::RuntimeApi, ferrum_testnet::Executor, _, _>(
        parachain_config,
        polkadot_config,
        collator_options,
        id,
        enable_evm_rpc,
        |client,
         block_import,
         config,
         telemetry,
         task_manager| {
            let slot_duration = cumulus_client_consensus_aura::slot_duration(&*client)?;

            cumulus_client_consensus_aura::import_queue::<
                sp_consensus_aura::sr25519::AuthorityPair,
                _,
                _,
                _,
                _,
                _,
            >(cumulus_client_consensus_aura::ImportQueueParams {
                block_import,
                client,
                create_inherent_data_providers: move |_, _| async move {
                    let timestamp = sp_timestamp::InherentDataProvider::from_system_time();

                    let slot =
                        sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
                            *timestamp,
                            slot_duration,
                        );

                    Ok((slot, timestamp))
                },
                registry: config.prometheus_registry(),
                spawner: &task_manager.spawn_essential_handle(),
                telemetry,
            })
            .map_err(Into::into)
        },
        |client,
        backend,
         block_import,
         prometheus_registry,
         telemetry,
         task_manager,
         relay_chain_interface,
         transaction_pool,
         sync_oracle,
         keystore,
         force_authoring| {
            let spawn_handle = task_manager.spawn_handle();

            let slot_duration =
                cumulus_client_consensus_aura::slot_duration(&*client).unwrap();

            let proposer_factory =
                sc_basic_authorship::ProposerFactory::with_proof_recording(
                    spawn_handle,
                    client.clone(),
                    transaction_pool,
                    prometheus_registry,
                    telemetry.clone(),
                );

            let relay_chain_for_aura = relay_chain_interface.clone();

            Ok(AuraConsensus::build::<
                sp_consensus_aura::sr25519::AuthorityPair,
                _,
                _,
                _,
                _,
                _,
                _,
            >(BuildAuraConsensusParams {
                proposer_factory,
                create_inherent_data_providers:
                    move |_, (relay_parent, validation_data)| {
                        let relay_chain_for_aura = relay_chain_for_aura.clone();
                        async move {
                            let parachain_inherent =
                                cumulus_primitives_parachain_inherent::ParachainInherentData::create_at(
                                    relay_parent,
                                    &relay_chain_for_aura,
                                    &validation_data,
                                    id,
                                ).await;
                            let timestamp = sp_timestamp::InherentDataProvider::from_system_time();
                            let slot =
                                sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
                                    *timestamp,
                                    slot_duration,
                                );

                            let parachain_inherent = parachain_inherent.ok_or_else(|| {
                                Box::<dyn std::error::Error + Send + Sync>::from(
                                    "Failed to create parachain inherent",
                                )
                            })?;
                            Ok((slot, timestamp, parachain_inherent))
                        }
                    },
                block_import,
                para_client: client,
                backoff_authoring_blocks: Option::<()>::None,
                sync_oracle,
                keystore,
                force_authoring,
                slot_duration,
                // We got around 500ms for proposing
                block_proposal_slot_portion: SlotProportion::new(1f32 / 24f32),
                // And a maximum of 750ms if slots are skipped
                max_block_proposal_slot_portion: Some(SlotProportion::new(1f32 / 16f32)),
                telemetry,
            })
        )
    },
    hwbench,
    cli
).await
}

/// Start a parachain node for QPN
pub async fn start_kusama_node(
	parachain_config: Configuration,
	polkadot_config: Configuration,
	collator_options: CollatorOptions,
	id: ParaId,
	enable_evm_rpc: bool,
	cli: &Cli,
	hwbench: Option<sc_sysinfo::HwBench>,
) -> sc_service::error::Result<(
	TaskManager,
	Arc<TFullClient<Block, kusama::RuntimeApi, NativeElseWasmExecutor<kusama::Executor>>>,
)> {
	start_node_impl::<kusama::RuntimeApi, kusama::Executor, _, _>(
        parachain_config,
        polkadot_config,
        collator_options,
        id,
        enable_evm_rpc,
        |client,
         block_import,
         config,
         telemetry,
         task_manager| {
            let slot_duration = cumulus_client_consensus_aura::slot_duration(&*client)?;

            cumulus_client_consensus_aura::import_queue::<
                sp_consensus_aura::sr25519::AuthorityPair,
                _,
                _,
                _,
                _,
                _,
            >(cumulus_client_consensus_aura::ImportQueueParams {
                block_import,
                client,
                create_inherent_data_providers: move |_, _| async move {
                    let timestamp = sp_timestamp::InherentDataProvider::from_system_time();

                    let slot =
                        sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
                            *timestamp,
                            slot_duration,
                        );

                    Ok((slot, timestamp))
                },
                registry: config.prometheus_registry(),
                spawner: &task_manager.spawn_essential_handle(),
                telemetry,
            })
            .map_err(Into::into)
        },
        |client,
        backend,
         block_import,
         prometheus_registry,
         telemetry,
         task_manager,
         relay_chain_interface,
         transaction_pool,
         sync_oracle,
         keystore,
         force_authoring| {
            let spawn_handle = task_manager.spawn_handle();

            let slot_duration =
                cumulus_client_consensus_aura::slot_duration(&*client).unwrap();

            let proposer_factory =
                sc_basic_authorship::ProposerFactory::with_proof_recording(
                    spawn_handle,
                    client.clone(),
                    transaction_pool,
                    prometheus_registry,
                    telemetry.clone(),
                );

            let relay_chain_for_aura = relay_chain_interface.clone();

            Ok(AuraConsensus::build::<
                sp_consensus_aura::sr25519::AuthorityPair,
                _,
                _,
                _,
                _,
                _,
                _,
            >(BuildAuraConsensusParams {
                proposer_factory,
                create_inherent_data_providers:
                    move |_, (relay_parent, validation_data)| {
                        let relay_chain_for_aura = relay_chain_for_aura.clone();
                        async move {
                            let parachain_inherent =
                                cumulus_primitives_parachain_inherent::ParachainInherentData::create_at(
                                    relay_parent,
                                    &relay_chain_for_aura,
                                    &validation_data,
                                    id,
                                ).await;
                            let timestamp = sp_timestamp::InherentDataProvider::from_system_time();
                            let slot =
                                sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
                                    *timestamp,
                                    slot_duration,
                                );

                            let parachain_inherent = parachain_inherent.ok_or_else(|| {
                                Box::<dyn std::error::Error + Send + Sync>::from(
                                    "Failed to create parachain inherent",
                                )
                            })?;
                            Ok((slot, timestamp, parachain_inherent))
                        }
                    },
                block_import,
                para_client: client,
                backoff_authoring_blocks: Option::<()>::None,
                sync_oracle,
                keystore,
                force_authoring,
                slot_duration,
                // We got around 500ms for proposing
                block_proposal_slot_portion: SlotProportion::new(1f32 / 24f32),
                // And a maximum of 750ms if slots are skipped
                max_block_proposal_slot_portion: Some(SlotProportion::new(1f32 / 16f32)),
                telemetry,
            })
        )
    },
    hwbench,
    cli
).await
}
