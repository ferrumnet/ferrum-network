use cumulus_primitives_core::ParaId;
use ferrum_runtime::{AccountId, AuraId, EXISTENTIAL_DEPOSIT};
use sc_chain_spec::{ChainSpecExtension, ChainSpecGroup};
use sc_service::ChainType;
use serde::{Deserialize, Serialize};
use sp_core::{Pair, Public};
use std::str::FromStr;

/// Specialized `ChainSpec` for the normal parachain runtime.
pub type ChainSpec = sc_service::GenericChainSpec<ferrum_runtime::GenesisConfig, Extensions>;

/// The default XCM version to set in genesis config.
const SAFE_XCM_VERSION: u32 = xcm::prelude::XCM_VERSION;

/// The extensions for the [`ChainSpec`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ChainSpecGroup, ChainSpecExtension)]
#[serde(deny_unknown_fields)]
pub struct Extensions {
    /// The relay chain of the Parachain.
    pub relay_chain: String,
    /// The id of the Parachain.
    pub para_id: u32,
}

impl Extensions {
    /// Try to get the extension from the given `ChainSpec`.
    pub fn try_get(chain_spec: &dyn sc_service::ChainSpec) -> Option<&Self> {
        sc_chain_spec::get_extension(chain_spec.extensions())
    }
}

/// Generate a crypto pair from seed.
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
    TPublic::Pair::from_string(&format!("//{seed}"), None)
        .expect("static values are valid; qed")
        .public()
}

/// Generate collator keys from seed.
///
/// This function's return type must always match the session keys of the chain in tuple format.
pub fn get_collator_keys_from_seed(s: &str) -> AuraId {
    get_from_seed::<AuraId>(s)
}

// /// Helper function to generate an account ID from seed
// pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
// where
//     AccountPublic: From<<TPublic::Pair as Pair>::Public>,
// {
//     AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
// }

/// Generate the session keys from individual elements.
///
/// The input must be a tuple of individual keys (a single arg for now since we have just one key).
pub fn ferrum_session_keys(keys: AuraId) -> ferrum_runtime::SessionKeys {
    ferrum_runtime::SessionKeys { aura: keys }
}

pub fn development_config() -> ChainSpec {
    // Give your base currency a tFRM name and decimal places
    let mut properties = sc_chain_spec::Properties::new();
    properties.insert("tokenSymbol".into(), "tFRM".into());
    properties.insert("tokenDecimals".into(), 18.into());
    properties.insert("ss58Format".into(), 42.into());

    ChainSpec::from_genesis(
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

pub fn local_testnet_config() -> ChainSpec {
    // Give your base currency a tFRM name and decimal places
    let mut properties = sc_chain_spec::Properties::new();
    properties.insert("tokenSymbol".into(), "tFRM".into());
    properties.insert("tokenDecimals".into(), 18.into());
    properties.insert("ss58Format".into(), 42.into());

    ChainSpec::from_genesis(
        // Name
        "Ferrum Testnet",
        // ID
        "ferrum_testnet",
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

pub fn rococo_config() -> ChainSpec {
    // Give your base currency a tFRM name and decimal places
    let mut properties = sc_chain_spec::Properties::new();
    properties.insert("tokenSymbol".into(), "tFRM".into());
    properties.insert("tokenDecimals".into(), 18.into());
    properties.insert("ss58Format".into(), 42.into());

    ChainSpec::from_genesis(
        // Name
        "Ferrum Rococo",
        // ID
        "ferrum_rococo",
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

fn testnet_genesis(
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
                .map(|k| (k, 1 << 80))
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
