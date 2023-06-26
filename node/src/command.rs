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
use std::net::SocketAddr;

use crate::primitives::Block;
use codec::Encode;
use cumulus_client_cli::generate_genesis_block;
use cumulus_primitives_core::ParaId;

use log::{info, warn};
use sc_cli::{
    ChainSpec, CliConfiguration, DefaultConfigurationValues, ImportParams, KeystoreParams,
    NetworkParams, Result, RuntimeVersion, SharedParams, SubstrateCli,
};
use sc_service::config::{BasePath, PrometheusConfig};
use sp_core::hexdisplay::HexDisplay;
use sp_runtime::traits::{AccountIdConversion, Block as BlockT};
// Frontier
use crate::service::{start_kusama_node, start_rococo_node, start_testnet_node};
use log::error;
use sc_service::PartialComponents;

use crate::{
    chain_spec,
    cli::{Cli, RelayChainCli, Subcommand},
    service::{build_import_queue, ferrum_testnet, kusama, new_partial, rococo},
};

trait IdentifyChain {
    fn is_kusama(&self) -> bool;
    fn is_dev(&self) -> bool;
    fn is_rococo(&self) -> bool;
}

impl IdentifyChain for dyn sc_service::ChainSpec {
    fn is_kusama(&self) -> bool {
        self.id().starts_with("quantum")
    }
    fn is_dev(&self) -> bool {
        self.id().starts_with("dev") || self.id().starts_with("testnet")
    }
    fn is_rococo(&self) -> bool {
        self.id().starts_with("rococo")
    }
}

impl<T: sc_service::ChainSpec + 'static> IdentifyChain for T {
    fn is_kusama(&self) -> bool {
        <dyn sc_service::ChainSpec>::is_kusama(self)
    }
    fn is_dev(&self) -> bool {
        <dyn sc_service::ChainSpec>::is_dev(self)
    }
    fn is_rococo(&self) -> bool {
        <dyn sc_service::ChainSpec>::is_rococo(self)
    }
}

fn load_spec(id: &str) -> std::result::Result<Box<dyn ChainSpec>, String> {
    Ok(match id {
        // testnet
        "dev" => Box::new(chain_spec::testnet::development_config()),
        "testnet-alpha" => Box::new(chain_spec::testnet::alpha_testnet_config()),

        // rococo
        "rococo-local" => Box::new(chain_spec::rococo::rococo_local_config()),
        "rococo" => Box::new(chain_spec::rococo::rococo_config()),

        // kusama
        "qpn-local" => Box::new(chain_spec::kusama::kusama_local_config()),
        "qpn" => Box::new(chain_spec::kusama::kusama_config()),

        "" | "local" => Box::new(chain_spec::kusama::kusama_local_config()),
        path => {
            let chain_spec = chain_spec::TestnetChainSpec::from_json_file(path.into())?;
            if chain_spec.is_kusama() {
                Box::new(chain_spec::KusamaChainSpec::from_json_file(path.into())?)
            } else if chain_spec.is_rococo() {
                Box::new(chain_spec::RococoChainSpec::from_json_file(path.into())?)
            } else if chain_spec.is_dev() {
                Box::new(chain_spec)
            } else {
                Err("Unclear which chain spec to base this chain on")?
            }
        }
    })
}

impl SubstrateCli for Cli {
    fn impl_name() -> String {
        "Quantum Portal Parachain".into()
    }

    fn impl_version() -> String {
        env!("SUBSTRATE_CLI_IMPL_VERSION").into()
    }

    fn description() -> String {
        format!(
            "Quantum Portal Parachain\n\nThe command-line arguments provided first will be \
		passed to the parachain node, while the arguments provided after -- will be passed \
		to the relay chain node.\n\n\
		{} <parachain-args> -- <relay-chain-args>",
            Self::executable_name()
        )
    }

    fn author() -> String {
        env!("CARGO_PKG_AUTHORS").into()
    }

    fn support_url() -> String {
        "https://github.com/ferrumnet/ferrum-network/issues/new".into()
    }

    fn copyright_start_year() -> i32 {
        2020
    }

    fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
        load_spec(id)
    }

    fn native_runtime_version(_: &Box<dyn ChainSpec>) -> &'static RuntimeVersion {
        &ferrum_testnet_runtime::VERSION
    }
}

impl SubstrateCli for RelayChainCli {
    fn impl_name() -> String {
        "Quantum Portal Parachain".into()
    }

    fn impl_version() -> String {
        env!("SUBSTRATE_CLI_IMPL_VERSION").into()
    }

    fn description() -> String {
        format!(
            "Quantum Portal Parachain\n\nThe command-line arguments provided first will be \
		passed to the parachain node, while the arguments provided after -- will be passed \
		to the relay chain node.\n\n\
		{} <parachain-args> -- <relay-chain-args>",
            Self::executable_name()
        )
    }

    fn author() -> String {
        env!("CARGO_PKG_AUTHORS").into()
    }

    fn support_url() -> String {
        "https://github.com/ferrumnet/ferrum-network/issues/new".into()
    }

    fn copyright_start_year() -> i32 {
        2020
    }

    fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
        polkadot_cli::Cli::from_iter([RelayChainCli::executable_name()].iter()).load_spec(id)
    }

    fn native_runtime_version(chain_spec: &Box<dyn ChainSpec>) -> &'static RuntimeVersion {
        polkadot_cli::Cli::native_runtime_version(chain_spec)
    }
}

/// Parse command line arguments into service configuration.
pub fn run() -> Result<()> {
    let cli = Cli::from_args();

    match &cli.subcommand {
        Some(Subcommand::Key(cmd)) => cmd.run(&cli),
        Some(Subcommand::BuildSpec(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.sync_run(|config| cmd.run(config.chain_spec, config.network))
        }
        Some(Subcommand::CheckBlock(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            if runner.config().chain_spec.is_kusama() {
                runner.async_run(|config| {
                    let PartialComponents {
                        client,
                        task_manager,
                        import_queue,
                        ..
                    } = new_partial::<kusama::RuntimeApi, kusama::Executor, _>(
                        &config,
                        build_import_queue,
                        &cli,
                    )?;
                    Ok((cmd.run(client, import_queue), task_manager))
                })
            } else if runner.config().chain_spec.is_rococo() {
                runner.async_run(|config| {
                    let PartialComponents {
                        client,
                        task_manager,
                        import_queue,
                        ..
                    } = new_partial::<rococo::RuntimeApi, rococo::Executor, _>(
                        &config,
                        build_import_queue,
                        &cli,
                    )?;
                    Ok((cmd.run(client, import_queue), task_manager))
                })
            } else {
                runner.async_run(|config| {
                    let PartialComponents {
                        client,
                        task_manager,
                        import_queue,
                        ..
                    } = new_partial::<ferrum_testnet::RuntimeApi, ferrum_testnet::Executor, _>(
                        &config,
                        build_import_queue,
                        &cli,
                    )?;
                    Ok((cmd.run(client, import_queue), task_manager))
                })
            }
        }
        Some(Subcommand::ExportBlocks(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            if runner.config().chain_spec.is_kusama() {
                runner.async_run(|config| {
                    let PartialComponents {
                        client,
                        task_manager,
                        ..
                    } = new_partial::<kusama::RuntimeApi, kusama::Executor, _>(
                        &config,
                        build_import_queue,
                        &cli,
                    )?;
                    Ok((cmd.run(client, config.database), task_manager))
                })
            } else if runner.config().chain_spec.is_rococo() {
                runner.async_run(|config| {
                    let PartialComponents {
                        client,
                        task_manager,
                        ..
                    } = new_partial::<rococo::RuntimeApi, rococo::Executor, _>(
                        &config,
                        build_import_queue,
                        &cli,
                    )?;
                    Ok((cmd.run(client, config.database), task_manager))
                })
            } else {
                runner.async_run(|config| {
                    let PartialComponents {
                        client,
                        task_manager,
                        ..
                    } = new_partial::<ferrum_testnet::RuntimeApi, ferrum_testnet::Executor, _>(
                        &config,
                        build_import_queue,
                        &cli,
                    )?;
                    Ok((cmd.run(client, config.database), task_manager))
                })
            }
        }
        Some(Subcommand::ExportState(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            if runner.config().chain_spec.is_kusama() {
                runner.async_run(|config| {
                    let PartialComponents {
                        client,
                        task_manager,
                        ..
                    } = new_partial::<kusama::RuntimeApi, kusama::Executor, _>(
                        &config,
                        build_import_queue,
                        &cli,
                    )?;
                    Ok((cmd.run(client, config.chain_spec), task_manager))
                })
            } else if runner.config().chain_spec.is_rococo() {
                runner.async_run(|config| {
                    let PartialComponents {
                        client,
                        task_manager,
                        ..
                    } = new_partial::<rococo::RuntimeApi, rococo::Executor, _>(
                        &config,
                        build_import_queue,
                        &cli,
                    )?;
                    Ok((cmd.run(client, config.chain_spec), task_manager))
                })
            } else {
                runner.async_run(|config| {
                    let PartialComponents {
                        client,
                        task_manager,
                        ..
                    } = new_partial::<ferrum_testnet::RuntimeApi, ferrum_testnet::Executor, _>(
                        &config,
                        build_import_queue,
                        &cli,
                    )?;
                    Ok((cmd.run(client, config.chain_spec), task_manager))
                })
            }
        }
        Some(Subcommand::ImportBlocks(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            if runner.config().chain_spec.is_kusama() {
                runner.async_run(|config| {
                    let PartialComponents {
                        client,
                        task_manager,
                        import_queue,
                        ..
                    } = new_partial::<kusama::RuntimeApi, kusama::Executor, _>(
                        &config,
                        build_import_queue,
                        &cli,
                    )?;
                    Ok((cmd.run(client, import_queue), task_manager))
                })
            } else if runner.config().chain_spec.is_rococo() {
                runner.async_run(|config| {
                    let PartialComponents {
                        client,
                        task_manager,
                        import_queue,
                        ..
                    } = new_partial::<rococo::RuntimeApi, rococo::Executor, _>(
                        &config,
                        build_import_queue,
                        &cli,
                    )?;
                    Ok((cmd.run(client, import_queue), task_manager))
                })
            } else {
                runner.async_run(|config| {
                    let PartialComponents {
                        client,
                        task_manager,
                        import_queue,
                        ..
                    } = new_partial::<ferrum_testnet::RuntimeApi, ferrum_testnet::Executor, _>(
                        &config,
                        build_import_queue,
                        &cli,
                    )?;
                    Ok((cmd.run(client, import_queue), task_manager))
                })
            }
        }
        Some(Subcommand::Revert(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            if runner.config().chain_spec.is_kusama() {
                runner.async_run(|config| {
                    let PartialComponents {
                        client,
                        task_manager,
                        backend,
                        ..
                    } = new_partial::<kusama::RuntimeApi, kusama::Executor, _>(
                        &config,
                        build_import_queue,
                        &cli,
                    )?;
                    let aux_revert = Box::new(|client, _, blocks| {
                        sc_finality_grandpa::revert(client, blocks)?;
                        Ok(())
                    });
                    Ok((cmd.run(client, backend, Some(aux_revert)), task_manager))
                })
            } else if runner.config().chain_spec.is_rococo() {
                runner.async_run(|config| {
                    let PartialComponents {
                        client,
                        task_manager,
                        backend,
                        ..
                    } = new_partial::<rococo::RuntimeApi, rococo::Executor, _>(
                        &config,
                        build_import_queue,
                        &cli,
                    )?;
                    let aux_revert = Box::new(|client, _, blocks| {
                        sc_finality_grandpa::revert(client, blocks)?;
                        Ok(())
                    });
                    Ok((cmd.run(client, backend, Some(aux_revert)), task_manager))
                })
            } else {
                runner.async_run(|config| {
                    let PartialComponents {
                        client,
                        task_manager,
                        backend,
                        ..
                    } = new_partial::<ferrum_testnet::RuntimeApi, ferrum_testnet::Executor, _>(
                        &config,
                        build_import_queue,
                        &cli,
                    )?;
                    let aux_revert = Box::new(|client, _, blocks| {
                        sc_finality_grandpa::revert(client, blocks)?;
                        Ok(())
                    });
                    Ok((cmd.run(client, backend, Some(aux_revert)), task_manager))
                })
            }
        }
        Some(Subcommand::PurgeChain(cmd)) => {
            let runner = cli.create_runner(cmd)?;

            runner.sync_run(|config| {
                let polkadot_cli = RelayChainCli::new(
                    &config,
                    [RelayChainCli::executable_name()]
                        .iter()
                        .chain(cli.relay_chain_args.iter()),
                );

                let polkadot_config = SubstrateCli::create_configuration(
                    &polkadot_cli,
                    &polkadot_cli,
                    config.tokio_handle.clone(),
                )
                .map_err(|err| format!("Relay chain argument error: {err}"))?;

                cmd.run(config, polkadot_config)
            })
        }
        Some(Subcommand::ExportGenesisState(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.sync_run(|_config| {
                let spec = cli.load_spec(&cmd.shared_params.chain.clone().unwrap_or_default())?;
                let state_version = Cli::native_runtime_version(&spec).state_version();
                cmd.run::<Block>(&*spec, state_version)
            })
        }
        Some(Subcommand::ExportGenesisWasm(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.sync_run(|_config| {
                let spec = cli.load_spec(&cmd.shared_params.chain.clone().unwrap_or_default())?;
                cmd.run(&*spec)
            })
        }
        #[cfg(not(feature = "frame-benchmarking-cli"))]
        Some(Subcommand::Benchmark(_cmd)) => todo!(),
        #[cfg(feature = "frame-benchmarking-cli")]
        Some(Subcommand::Benchmark(cmd)) => {
            use crate::benchmarking::*;
            use sp_keyring::Sr25519Keyring;

            let runner = cli.create_runner(cmd)?;
            let chain_spec = &runner.config().chain_spec;

            // Switch on the concrete benchmark sub-command-
            match cmd {
                BenchmarkCmd::Pallet(cmd) => {
                    if chain_spec.is_kusama() {
                        runner.sync_run(|config| {
                            cmd.run::<ferrum_runtime::Block, kusama::Executor>(config)
                        })
                    } else if chain_spec.is_rococo() {
                        runner.sync_run(|config| {
                            cmd.run::<ferrum_rococo_runtime::Block, rococo::Executor>(config)
                        })
                    } else {
                        runner.sync_run(|config| {
                            cmd.run::<ferrum_testnet_runtime::Block, ferrum_testnet::Executor>(
                                config,
                            )
                        })
                    }
                }
                BenchmarkCmd::Block(cmd) => {
                    if chain_spec.is_kusama() {
                        runner.sync_run(|config| {
                            let params = new_partial::<kusama::RuntimeApi, kusama::Executor, _>(
                                &config,
                                build_import_queue,
                                &cli,
                            )?;
                            cmd.run(params.client)
                        })
                    } else if chain_spec.is_rococo() {
                        runner.sync_run(|config| {
                            let params = new_partial::<rococo::RuntimeApi, rococo::Executor, _>(
                                &config,
                                build_import_queue,
                                &cli,
                            )?;
                            cmd.run(params.client)
                        })
                    } else {
                        runner.sync_run(|config| {
                            let params = new_partial::<
                                ferrum_testnet::RuntimeApi,
                                ferrum_testnet::Executor,
                                _,
                            >(
                                &config, build_import_queue, &cli
                            )?;
                            cmd.run(params.client)
                        })
                    }
                }
                BenchmarkCmd::Storage(cmd) => {
                    if chain_spec.is_kusama() {
                        runner.sync_run(|config| {
                            let params = new_partial::<kusama::RuntimeApi, kusama::Executor, _>(
                                &config,
                                build_import_queue,
                                &cli,
                            )?;
                            let db = params.backend.expose_db();
                            let storage = params.backend.expose_storage();

                            cmd.run(config, params.client, db, storage)
                        })
                    } else if chain_spec.is_rococo() {
                        runner.sync_run(|config| {
                            let params = new_partial::<rococo::RuntimeApi, rococo::Executor, _>(
                                &config,
                                build_import_queue,
                                &cli,
                            )?;
                            let db = params.backend.expose_db();
                            let storage = params.backend.expose_storage();

                            cmd.run(config, params.client, db, storage)
                        })
                    } else {
                        runner.sync_run(|config| {
                            let params = new_partial::<
                                ferrum_testnet::RuntimeApi,
                                ferrum_testnet::Executor,
                                _,
                            >(
                                &config, build_import_queue, &cli
                            )?;
                            let db = params.backend.expose_db();
                            let storage = params.backend.expose_storage();

                            cmd.run(config, params.client, db, storage)
                        })
                    }
                }
                BenchmarkCmd::Overhead(cmd) => {
                    if chain_spec.is_kusama() {
                        runner.sync_run(|config| {
                            let params = new_partial::<kusama::RuntimeApi, kusama::Executor, _>(
                                &config,
                                build_import_queue,
                                &cli,
                            )?;
                            let ext_builder = RemarkBuilder::new(params.client.clone());
                            let inherent_data = para_benchmark_inherent_data()
                                .map_err(|e| format!("generating inherent data: {:?}", e))?;

                            cmd.run(
                                config,
                                params.client,
                                inherent_data,
                                Vec::new(),
                                &ext_builder,
                            )
                        })
                    } else if chain_spec.is_rococo() {
                        runner.sync_run(|config| {
                            let params = new_partial::<rococo::RuntimeApi, rococo::Executor, _>(
                                &config,
                                build_import_queue,
                                &cli,
                            )?;

                            let ext_builder = RemarkBuilder::new(params.client.clone());
                            let inherent_data = para_benchmark_inherent_data()
                                .map_err(|e| format!("generating inherent data: {:?}", e))?;

                            cmd.run(
                                config,
                                params.client,
                                inherent_data,
                                Vec::new(),
                                &ext_builder,
                            )
                        })
                    } else {
                        runner.sync_run(|config| {
                            let params = new_partial::<
                                ferrum_testnet::RuntimeApi,
                                ferrum_testnet::Executor,
                                _,
                            >(
                                &config, build_import_queue, &cli
                            )?;

                            let ext_builder = RemarkBuilder::new(params.client.clone());
                            let inherent_data = para_benchmark_inherent_data()
                                .map_err(|e| format!("generating inherent data: {:?}", e))?;

                            cmd.run(
                                config,
                                params.client,
                                inherent_data,
                                Vec::new(),
                                &ext_builder,
                            )
                        })
                    }
                }
                BenchmarkCmd::Extrinsic(cmd) => {
                    if chain_spec.is_kusama() {
                        runner.sync_run(|config| {
                            let params = new_partial::<kusama::RuntimeApi, kusama::Executor, _>(
                                &config,
                                build_import_queue,
                                &cli,
                            )?;
                            let remark_builder = RemarkBuilder::new(params.client.clone());
                            let tka_builder = TransferKeepAliveBuilder::new(
                                params.client.clone(),
                                Sr25519Keyring::Alice.to_account_id(),
                                params.client.existential_deposit(),
                            );
                            let ext_factory = ExtrinsicFactory(vec![
                                Box::new(remark_builder),
                                Box::new(tka_builder),
                            ]);
                            let inherent_data = para_benchmark_inherent_data()
                                .map_err(|e| format!("generating inherent data: {:?}", e))?;

                            cmd.run(params.client, inherent_data, Vec::new(), &ext_factory)
                        })
                    } else if chain_spec.is_rococo() {
                        runner.sync_run(|config| {
                            let params = new_partial::<rococo::RuntimeApi, rococo::Executor, _>(
                                &config,
                                build_import_queue,
                                &cli,
                            )?;
                            let remark_builder = RemarkBuilder::new(params.client.clone());
                            let tka_builder = TransferKeepAliveBuilder::new(
                                params.client.clone(),
                                Sr25519Keyring::Alice.to_account_id(),
                                params.client.existential_deposit(),
                            );
                            let ext_factory = ExtrinsicFactory(vec![
                                Box::new(remark_builder),
                                Box::new(tka_builder),
                            ]);
                            let inherent_data = para_benchmark_inherent_data()
                                .map_err(|e| format!("generating inherent data: {:?}", e))?;

                            cmd.run(params.client, inherent_data, Vec::new(), &ext_factory)
                        })
                    } else {
                        runner.sync_run(|config| {
                            let params = new_partial::<
                                ferrum_testnet::RuntimeApi,
                                ferrum_testnet::Executor,
                                _,
                            >(
                                &config, build_import_queue, &cli
                            )?;
                            let remark_builder = RemarkBuilder::new(params.client.clone());
                            let tka_builder = TransferKeepAliveBuilder::new(
                                params.client.clone(),
                                Sr25519Keyring::Alice.to_account_id(),
                                params.client.existential_deposit(),
                            );
                            let ext_factory = ExtrinsicFactory(vec![
                                Box::new(remark_builder),
                                Box::new(tka_builder),
                            ]);
                            let inherent_data = para_benchmark_inherent_data()
                                .map_err(|e| format!("generating inherent data: {:?}", e))?;

                            cmd.run(params.client, inherent_data, Vec::new(), &ext_factory)
                        })
                    }
                }
                BenchmarkCmd::Machine(cmd) => {
                    runner.sync_run(|config| cmd.run(&config, SUBSTRATE_REFERENCE_HARDWARE.clone()))
                }
            }
        }
        #[cfg(feature = "try-runtime")]
        Some(Subcommand::TryRuntime(cmd)) => {
            let runner = cli.create_runner(cmd)?;

            use sc_executor::{sp_wasm_interface::ExtendedHostFunctions, NativeExecutionDispatch};
            type HostFunctionsOf<E> = ExtendedHostFunctions<
                sp_io::SubstrateHostFunctions,
                <E as NativeExecutionDispatch>::ExtendHostFunctions,
            >;

            // grab the task manager.
            let registry = &runner
                .config()
                .prometheus_config
                .as_ref()
                .map(|cfg| &cfg.registry);
            let task_manager =
                sc_service::TaskManager::new(runner.config().tokio_handle.clone(), *registry)
                    .map_err(|e| format!("Error: {:?}", e))?;

            runner.async_run(|_| {
                Ok((
                    cmd.run::<Block, HostFunctionsOf<ParachainNativeExecutor>>(),
                    task_manager,
                ))
            })
        }
        #[cfg(not(feature = "try-runtime"))]
        Some(Subcommand::TryRuntime) => Err("Try-runtime was not enabled when building the node. \
			You can enable it with `--features try-runtime`."
            .into()),
        None => {
            let runner = cli.create_runner(&cli.run.normalize())?;
            let collator_options = cli.run.collator_options();

            runner.run_node_until_exit(|config| async move {
				let _hwbench = if !cli.no_hardware_benchmarks {
					config.database.path().map(|database_path| {
						let _ = std::fs::create_dir_all(database_path);
						sc_sysinfo::gather_hwbench(Some(database_path))
					})
				} else {
					None
				};

				let para_id = chain_spec::Extensions::try_get(&*config.chain_spec)
					.map(|e| e.para_id)
					.ok_or("Could not find parachain ID in chain-spec.")?;

				let polkadot_cli = RelayChainCli::new(
					&config,
					[RelayChainCli::executable_name()].iter().chain(cli.relay_chain_args.iter()),
				);

				let id = ParaId::from(para_id);

				let parachain_account =
					AccountIdConversion::<polkadot_primitives::v2::AccountId>::into_account_truncating(&id);

				let state_version = Cli::native_runtime_version(&config.chain_spec).state_version();
				let block: Block = generate_genesis_block(&*config.chain_spec, state_version)
					.map_err(|e| format!("{e:?}"))?;
				let genesis_state = format!("0x{:?}", HexDisplay::from(&block.header().encode()));

				let tokio_handle = config.tokio_handle.clone();
				let polkadot_config =
					SubstrateCli::create_configuration(&polkadot_cli, &polkadot_cli, tokio_handle)
						.map_err(|err| format!("Relay chain argument error: {err}"))?;

				info!("Parachain id: {:?}", id);
				info!("Parachain Account: {}", parachain_account);
				info!("Parachain genesis state: {}", genesis_state);
				info!("Is collating: {}", if config.role.is_authority() { "yes" } else { "no" });

				if !collator_options.relay_chain_rpc_urls.is_empty() && !cli.relay_chain_args.is_empty() {
					warn!("Detected relay chain node arguments together with --relay-chain-rpc-url. This command starts a minimal Polkadot node that only uses a network-related subset of all relay chain CLI options.");
				}

				if config.chain_spec.is_kusama() {
                    start_kusama_node(
                        config,
                        polkadot_config,
                        collator_options,
                        id,
                        true,
                        &cli
                    )
                        .await
                        .map(|r| r.0)
                        .map_err(Into::into)
                } else if config.chain_spec.is_rococo() {
                    start_rococo_node( config,
                        polkadot_config,
                        collator_options,
                        id,
                        true,
                        &cli
                    )
                        .await
                        .map(|r| r.0)
                        .map_err(Into::into)
                } else if config.chain_spec.is_dev() {
                    start_testnet_node( config,
                        polkadot_config,
                        collator_options,
                        id,
                        true,
                    &cli)
                        .await
                        .map(|r| r.0)
                        .map_err(Into::into)
                } else {
                    let err_msg = "Unrecognized chain spec - name should start with one of: kusama or rococo or testnet";
                    error!("{}", err_msg);
                    Err(err_msg.into())
                }
			})
        }
    }
}

impl DefaultConfigurationValues for RelayChainCli {
    fn p2p_listen_port() -> u16 {
        30334
    }

    fn rpc_ws_listen_port() -> u16 {
        9945
    }

    fn rpc_http_listen_port() -> u16 {
        9934
    }

    fn prometheus_listen_port() -> u16 {
        9616
    }
}

impl CliConfiguration<Self> for RelayChainCli {
    fn shared_params(&self) -> &SharedParams {
        self.base.base.shared_params()
    }

    fn import_params(&self) -> Option<&ImportParams> {
        self.base.base.import_params()
    }

    fn network_params(&self) -> Option<&NetworkParams> {
        self.base.base.network_params()
    }

    fn keystore_params(&self) -> Option<&KeystoreParams> {
        self.base.base.keystore_params()
    }

    fn base_path(&self) -> Result<Option<BasePath>> {
        Ok(self
            .shared_params()
            .base_path()?
            .or_else(|| self.base_path.clone().map(Into::into)))
    }

    fn rpc_http(&self, default_listen_port: u16) -> Result<Option<SocketAddr>> {
        self.base.base.rpc_http(default_listen_port)
    }

    fn rpc_ipc(&self) -> Result<Option<String>> {
        self.base.base.rpc_ipc()
    }

    fn rpc_ws(&self, default_listen_port: u16) -> Result<Option<SocketAddr>> {
        self.base.base.rpc_ws(default_listen_port)
    }

    fn prometheus_config(
        &self,
        default_listen_port: u16,
        chain_spec: &Box<dyn ChainSpec>,
    ) -> Result<Option<PrometheusConfig>> {
        self.base
            .base
            .prometheus_config(default_listen_port, chain_spec)
    }

    fn init<F>(
        &self,
        _support_url: &String,
        _impl_version: &String,
        _logger_hook: F,
        _config: &sc_service::Configuration,
    ) -> Result<()>
    where
        F: FnOnce(&mut sc_cli::LoggerBuilder, &sc_service::Configuration),
    {
        unreachable!("PolkadotCli is never initialized; qed");
    }

    fn chain_id(&self, is_dev: bool) -> Result<String> {
        let chain_id = self.base.base.chain_id(is_dev)?;

        Ok(if chain_id.is_empty() {
            self.chain_id.clone().unwrap_or_default()
        } else {
            chain_id
        })
    }

    fn role(&self, is_dev: bool) -> Result<sc_service::Role> {
        self.base.base.role(is_dev)
    }

    fn transaction_pool(&self, is_dev: bool) -> Result<sc_service::config::TransactionPoolOptions> {
        self.base.base.transaction_pool(is_dev)
    }

    fn trie_cache_maximum_size(&self) -> Result<Option<usize>> {
        self.base.base.trie_cache_maximum_size()
    }

    fn rpc_methods(&self) -> Result<sc_service::config::RpcMethods> {
        self.base.base.rpc_methods()
    }

    fn rpc_ws_max_connections(&self) -> Result<Option<usize>> {
        self.base.base.rpc_ws_max_connections()
    }

    fn rpc_cors(&self, is_dev: bool) -> Result<Option<Vec<String>>> {
        self.base.base.rpc_cors(is_dev)
    }

    fn default_heap_pages(&self) -> Result<Option<u64>> {
        self.base.base.default_heap_pages()
    }

    fn force_authoring(&self) -> Result<bool> {
        self.base.base.force_authoring()
    }

    fn disable_grandpa(&self) -> Result<bool> {
        self.base.base.disable_grandpa()
    }

    fn max_runtime_instances(&self) -> Result<Option<usize>> {
        self.base.base.max_runtime_instances()
    }

    fn announce_block(&self) -> Result<bool> {
        self.base.base.announce_block()
    }

    fn telemetry_endpoints(
        &self,
        chain_spec: &Box<dyn ChainSpec>,
    ) -> Result<Option<sc_telemetry::TelemetryEndpoints>> {
        self.base.base.telemetry_endpoints(chain_spec)
    }

    fn node_name(&self) -> Result<String> {
        self.base.base.node_name()
    }
}
