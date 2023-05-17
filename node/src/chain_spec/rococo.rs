use super::*;
use cumulus_primitives_core::ParaId;
use ferrum_rococo_runtime::{AccountId, AuraId, EXISTENTIAL_DEPOSIT};

use sc_service::ChainType;

use std::str::FromStr;

/// Specialized `RococoChainSpec` for the normal parachain runtime.
pub type RococoChainSpec =
    sc_service::GenericChainSpec<ferrum_rococo_runtime::GenesisConfig, Extensions>;

/// Generate collator keys from seed.
///
/// This function's return type must always match the session keys of the chain in tuple format.
pub fn get_collator_keys_from_seed(s: &str) -> AuraId {
    get_from_seed::<AuraId>(s)
}

/// Generate the session keys from individual elements.
///
/// The input must be a tuple of individual keys (a single arg for now since we have just one key).
pub fn ferrum_session_keys(keys: AuraId) -> ferrum_rococo_runtime::SessionKeys {
    ferrum_rococo_runtime::SessionKeys { aura: keys }
}

pub fn rococo_local_config() -> RococoChainSpec {
    // Give your base currency a tFRM name and decimal places
    let mut properties = sc_chain_spec::Properties::new();
    properties.insert("tokenSymbol".into(), "rFRM".into());
    properties.insert("tokenDecimals".into(), 18.into());
    properties.insert("ss58Format".into(), 42.into());

    RococoChainSpec::from_genesis(
        // Name
        "Ferrum Rococo",
        // ID
        "rococo_ferrum",
        ChainType::Local,
        move || {
            rococo_genesis(
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

pub fn rococo_config() -> RococoChainSpec {
    // Give your base currency a tFRM name and decimal places
    let mut properties = sc_chain_spec::Properties::new();
    properties.insert("tokenSymbol".into(), "rFRM".into());
    properties.insert("tokenDecimals".into(), 18.into());
    properties.insert("ss58Format".into(), 42.into());

    RococoChainSpec::from_genesis(
        // Name
        "Ferrum Rococo",
        // ID
        "rococo_ferrum",
        ChainType::Live,
        move || {
            rococo_genesis(
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
                4238.into(),
            )
        },
        // Bootnodes
        Vec::new(),
        // Telemetry
        None,
        // Protocol ID
        Some("ferrum-rococo"),
        // Fork ID
        None,
        // Properties
        Some(properties),
        // Extensions
        Extensions {
            relay_chain: "rococo".into(), // You MUST set this to the correct network!
            para_id: 4238,
        },
    )
}

fn rococo_genesis(
    invulnerables: Vec<(AccountId, AuraId)>,
    endowed_accounts: Vec<AccountId>,
    root_key: AccountId,
    id: ParaId,
) -> ferrum_rococo_runtime::GenesisConfig {
    ferrum_rococo_runtime::GenesisConfig {
        system: ferrum_rococo_runtime::SystemConfig {
            code: ferrum_rococo_runtime::WASM_BINARY
                .expect("WASM binary was not build, please build it!")
                .to_vec(),
        },
        balances: ferrum_rococo_runtime::BalancesConfig {
            balances: endowed_accounts
                .iter()
                .cloned()
                .map(|k| (k, EXISTENTIAL_DEPOSIT * 1000))
                .collect(),
        },
        parachain_info: ferrum_rococo_runtime::ParachainInfoConfig { parachain_id: id },
        collator_selection: ferrum_rococo_runtime::CollatorSelectionConfig {
            invulnerables: invulnerables.iter().cloned().map(|(acc, _)| acc).collect(),
            candidacy_bond: EXISTENTIAL_DEPOSIT * 16,
            ..Default::default()
        },
        session: ferrum_rococo_runtime::SessionConfig {
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
        sudo: ferrum_rococo_runtime::SudoConfig {
            // Assign network admin rights.
            key: Some(root_key),
        },
        polkadot_xcm: ferrum_rococo_runtime::PolkadotXcmConfig {
            safe_xcm_version: Some(SAFE_XCM_VERSION),
        },
        evm: Default::default(),
        ethereum: ferrum_rococo_runtime::EthereumConfig {},
        dynamic_fee: Default::default(),
        base_fee: Default::default(),
    }
}
