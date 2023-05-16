use cumulus_primitives_core::ParaId;
use ferrum_runtime::{AccountId, AuraId, EXISTENTIAL_DEPOSIT};
use sc_chain_spec::{ChainSpecExtension, ChainSpecGroup};
use sc_service::ChainType;
use serde::{Deserialize, Serialize};
use sp_core::{Pair, Public};
use std::str::FromStr;
use super::*;

/// Specialized `ChainSpec` for the normal parachain runtime.
pub type KusamaChainSpec = sc_service::GenericChainSpec<ferrum_runtime::GenesisConfig, Extensions>;

/// Generate the session keys from individual elements.
///
/// The input must be a tuple of individual keys (a single arg for now since we have just one key).
pub fn ferrum_session_keys(keys: AuraId) -> ferrum_runtime::SessionKeys {
    ferrum_runtime::SessionKeys { aura: keys }
}

pub fn kusama_local_config() -> ChainSpec {
    // Give your base currency a QPN name and decimal places
    let mut properties = sc_chain_spec::Properties::new();
    properties.insert("tokenSymbol".into(), "tQPN".into());
    properties.insert("tokenDecimals".into(), 18.into());
    properties.insert("ss58Format".into(), 42.into());

    ChainSpec::from_genesis(
        // Name
        "Quantum Portal Network Local",
        // ID
        "quantum_portal_network_local",
        ChainType::Local,
        move || {
            testnet_genesis(
                // TODO : Configure initial accounts
                vec![
                    (
                        AccountId::from_str("e04cc55ebee1cbce552f250e85c57b70b2e2625b").unwrap(),
                        get_collator_keys_from_seed("Alice"),
                    ),
                    (
                        AccountId::from_str("0x25451A4de12dcCc2D166922fA938E900fCc4ED24").unwrap(),
                        get_collator_keys_from_seed("Bob"),
                    ),
                ],
                // Endowed Accounts
                vec![AccountId::from_str("e04cc55ebee1cbce552f250e85c57b70b2e2625b").unwrap()],
                // Sudo Key
                AccountId::from_str("e04cc55ebee1cbce552f250e85c57b70b2e2625b").unwrap(),
                1000.into(),
            )
        },
        // Bootnodes
        Vec::new(),
        // Telemetry
        None,
        // Protocol ID
        Some("template-local"),
        // Fork ID
        None,
        // Properties
        Some(properties),
        // Extensions
        Extensions {
            relay_chain: "rococo-local".into(), // You MUST set this to the correct network!
            para_id: 1000,
        },
    )
}

pub fn kusama_config() -> ChainSpec {
    // Give your base currency a QPN name and decimal places
    let mut properties = sc_chain_spec::Properties::new();
    properties.insert("tokenSymbol".into(), "QPN".into());
    properties.insert("tokenDecimals".into(), 18.into());
    properties.insert("ss58Format".into(), 42.into());

    ChainSpec::from_genesis(
        // Name
        "Quantum Portal Network",
        // ID
        "quantum_portal_network",
        ChainType::Local,
        move || {
            generate_genesis(
                // initial collators.
                vec![
                    (
                        AccountId::from_str("e04cc55ebee1cbce552f250e85c57b70b2e2625b").unwrap(),
                        get_collator_keys_from_seed("Alice"),
                    ),
                    (
                        AccountId::from_str("0x25451A4de12dcCc2D166922fA938E900fCc4ED24").unwrap(),
                        get_collator_keys_from_seed("Bob"),
                    ),
                ],
                // Endowed Accounts
                vec![AccountId::from_str("e04cc55ebee1cbce552f250e85c57b70b2e2625b").unwrap()],
                // Sudo Key
                AccountId::from_str("e04cc55ebee1cbce552f250e85c57b70b2e2625b").unwrap(),
                1000.into(),
            )
        },
        // Bootnodes
        Vec::new(),
        // Telemetry
        None,
        // Protocol ID
        Some("qpn-parachain"),
        // Fork ID
        None,
        // Properties
        Some(properties),
        // Extensions
        Extensions {
            relay_chain: "kusama".into(),
            para_id: 1000, // TODO : Set this after we reserve slot
        },
    )
}

fn generate_genesis(
    invulnerables: Vec<(AccountId, AuraId)>,
    endowed_accounts: Vec<AccountId>,
    root_key: AccountId,
    id: ParaId,
) -> ferrum_runtime::GenesisConfig {
    ferrum_runtime::GenesisConfig {
        system: ferrum_runtime::SystemConfig {
            code: ferrum_runtime::WASM_BINARY
                .expect("WASM binary was not build, please build it!")
                .to_vec(),
        },
        balances: ferrum_runtime::BalancesConfig {
            balances: endowed_accounts
                .iter()
                .cloned()
                .map(|k| (k, 1000)) // TODO : Use UNITS
                .collect(),
        },
        parachain_info: ferrum_runtime::ParachainInfoConfig { parachain_id: id },
        collator_selection: ferrum_runtime::CollatorSelectionConfig {
            invulnerables: invulnerables.iter().cloned().map(|(acc, _)| acc).collect(),
            candidacy_bond: EXISTENTIAL_DEPOSIT * 16,
            ..Default::default()
        },
        session: ferrum_runtime::SessionConfig {
            keys: invulnerables
                .into_iter()
                .map(|(acc, aura)| {
                    (
                        acc,                       // account id
                        acc,                       // validator id
                        ferrum_session_keys(aura), // session keys
                    )
                })
                .collect(),
        },
        // no need to pass anything to aura, in fact it will panic if we do. Session will take care
        // of this.
        aura: Default::default(),
        aura_ext: Default::default(),
        parachain_system: Default::default(),
        sudo: ferrum_runtime::SudoConfig {
            // Assign network admin rights.
            key: Some(root_key),
        },
        polkadot_xcm: ferrum_runtime::PolkadotXcmConfig {
            safe_xcm_version: Some(SAFE_XCM_VERSION),
        },
        evm: Default::default(),
        ethereum: ferrum_runtime::EthereumConfig {},
        dynamic_fee: Default::default(),
        base_fee: Default::default(),
    }
}
