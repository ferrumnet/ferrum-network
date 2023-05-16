use super::*;
use cumulus_primitives_core::ParaId;
use ferrum_testnet_runtime::{AccountId, AuraId, EXISTENTIAL_DEPOSIT};

use sc_service::ChainType;

use std::str::FromStr;

/// Specialized `TestnetChainSpec` for the normal parachain runtime.
pub type TestnetChainSpec =
    sc_service::GenericChainSpec<ferrum_testnet_runtime::GenesisConfig, Extensions>;

/// Generate collator keys from seed.
///
/// This function's return type must always match the session keys of the chain in tuple format.
pub fn get_collator_keys_from_seed(s: &str) -> AuraId {
    get_from_seed::<AuraId>(s)
}

/// Generate the session keys from individual elements.
///
/// The input must be a tuple of individual keys (a single arg for now since we have just one key).
pub fn ferrum_session_keys(keys: AuraId) -> ferrum_testnet_runtime::SessionKeys {
    ferrum_testnet_runtime::SessionKeys { aura: keys }
}

pub fn development_config() -> TestnetChainSpec {
    // Give your base currency a tFRM name and decimal places
    let mut properties = sc_chain_spec::Properties::new();
    properties.insert("tokenSymbol".into(), "tFRM".into());
    properties.insert("tokenDecimals".into(), 18.into());
    properties.insert("ss58Format".into(), 42.into());

    TestnetChainSpec::from_genesis(
        // Name
        "Ferrum Development",
        // ID
        "dev",
        ChainType::Development,
        move || {
            testnet_genesis(
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
        Vec::new(),
        None,
        None,
        None,
        None,
        Extensions {
            relay_chain: "rococo-local".into(), // You MUST set this to the correct network!
            para_id: 1000,
        },
    )
}

pub fn local_testnet_config() -> TestnetChainSpec {
    // Give your base currency a tFRM name and decimal places
    let mut properties = sc_chain_spec::Properties::new();
    properties.insert("tokenSymbol".into(), "tFRM".into());
    properties.insert("tokenDecimals".into(), 18.into());
    properties.insert("ss58Format".into(), 42.into());

    TestnetChainSpec::from_genesis(
        // Name
        "Ferrum Testnet",
        // ID
        "testnet_local",
        ChainType::Local,
        move || {
            testnet_genesis(
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

pub fn alpha_testnet_config() -> TestnetChainSpec {
    // Give your base currency a tFRM name and decimal places
    let mut properties = sc_chain_spec::Properties::new();
    properties.insert("tokenSymbol".into(), "tFRM".into());
    properties.insert("tokenDecimals".into(), 18.into());
    properties.insert("ss58Format".into(), 42.into());

    TestnetChainSpec::from_genesis(
        // Name
        "Ferrum Testnet",
        // ID
        "testnet_alpha",
        ChainType::Live,
        move || {
            testnet_genesis(
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
        Some("ferrum-alpha-testnet"),
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

fn testnet_genesis(
    invulnerables: Vec<(AccountId, AuraId)>,
    endowed_accounts: Vec<AccountId>,
    root_key: AccountId,
    id: ParaId,
) -> ferrum_testnet_runtime::GenesisConfig {
    ferrum_testnet_runtime::GenesisConfig {
        system: ferrum_testnet_runtime::SystemConfig {
            code: ferrum_testnet_runtime::WASM_BINARY
                .expect("WASM binary was not build, please build it!")
                .to_vec(),
        },
        balances: ferrum_testnet_runtime::BalancesConfig {
            balances: endowed_accounts
                .iter()
                .cloned()
                .map(|k| (k, 1 << 80))
                .collect(),
        },
        parachain_info: ferrum_testnet_runtime::ParachainInfoConfig { parachain_id: id },
        collator_selection: ferrum_testnet_runtime::CollatorSelectionConfig {
            invulnerables: invulnerables.iter().cloned().map(|(acc, _)| acc).collect(),
            candidacy_bond: EXISTENTIAL_DEPOSIT * 16,
            ..Default::default()
        },
        session: ferrum_testnet_runtime::SessionConfig {
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
        sudo: ferrum_testnet_runtime::SudoConfig {
            // Assign network admin rights.
            key: Some(root_key),
        },
        polkadot_xcm: ferrum_testnet_runtime::PolkadotXcmConfig {
            safe_xcm_version: Some(SAFE_XCM_VERSION),
        },
        evm: Default::default(),
        ethereum: ferrum_testnet_runtime::EthereumConfig {},
        dynamic_fee: Default::default(),
        base_fee: Default::default(),
    }
}
