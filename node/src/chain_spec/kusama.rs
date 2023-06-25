use super::*;
use cumulus_primitives_core::ParaId;
use ferrum_runtime::{AccountId, AuraId, EXISTENTIAL_DEPOSIT};
use hex_literal::hex;
use sc_service::ChainType;
use sp_application_crypto::ByteArray;
use std::str::FromStr;

/// Specialized `ChainSpec` for the normal parachain runtime.
pub type KusamaChainSpec = sc_service::GenericChainSpec<ferrum_runtime::GenesisConfig, Extensions>;

/// Generate the session keys from individual elements.
///
/// The input must be a tuple of individual keys (a single arg for now since we have just one key).
pub fn ferrum_session_keys(keys: AuraId) -> ferrum_runtime::SessionKeys {
    ferrum_runtime::SessionKeys { aura: keys }
}

pub fn kusama_local_config() -> KusamaChainSpec {
    // Give your base currency a QPN name and decimal places
    let mut properties = sc_chain_spec::Properties::new();
    properties.insert("tokenSymbol".into(), "tqpFRM".into());
    properties.insert("tokenDecimals".into(), 18.into());
    properties.insert("ss58Format".into(), 42.into());

    KusamaChainSpec::from_genesis(
        // Name
        "Quantum Portal Network Local",
        // ID
        "quantum_portal_network_local",
        ChainType::Live,
        move || {
            generate_genesis(
                vec![
                    (
                        AccountId::from_str("229FEf7f74a51590FaB754BDD927bA287b7F46cd").unwrap(),
                        AuraId::from_slice(&hex!(
                            "00d9eb842ad7b599d5eb1aefa8d2ef48e418dfc62c98f6925fb03853a82bda4a"
                        ))
                        .unwrap(),
                    ),
                    (
                        AccountId::from_str("850642c3b288d67A5dA39351569c0b0c4DE8210A").unwrap(),
                        AuraId::from_slice(&hex!(
                            "743d69b340f9fc9f019329ff1470cc6a738baf384a47af6023bed62ff8383e69"
                        ))
                        .unwrap(),
                    ),
                ],
                 // Endowed Accounts
                vec![AccountId::from_str("8097c3C354652CB1EEed3E5B65fBa2576470678A").unwrap()],
                // Sudo Key
                AccountId::from_str("8097c3C354652CB1EEed3E5B65fBa2576470678A").unwrap(),
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

pub fn kusama_config() -> KusamaChainSpec {
    // Give your base currency a QPN name and decimal places
    let mut properties = sc_chain_spec::Properties::new();
    properties.insert("tokenSymbol".into(), "qpFRM".into());
    properties.insert("tokenDecimals".into(), 18.into());
    properties.insert("ss58Format".into(), 42.into());

    KusamaChainSpec::from_genesis(
        // Name
        "Quantum Portal Network",
        // ID
        "quantum_portal_network",
        ChainType::Live,
        move || {
            generate_genesis(
                vec![
                    (
                        AccountId::from_str("229FEf7f74a51590FaB754BDD927bA287b7F46cd").unwrap(),
                        AuraId::from_slice(&hex!(
                            "00d9eb842ad7b599d5eb1aefa8d2ef48e418dfc62c98f6925fb03853a82bda4a"
                        ))
                        .unwrap(),
                    ),
                    (
                        AccountId::from_str("850642c3b288d67A5dA39351569c0b0c4DE8210A").unwrap(),
                        AuraId::from_slice(&hex!(
                            "743d69b340f9fc9f019329ff1470cc6a738baf384a47af6023bed62ff8383e69"
                        ))
                        .unwrap(),
                    ),
                ],
                // Endowed Accounts
                vec![AccountId::from_str("87C064f565414399Da9b7a94209378F33B17af94").unwrap()],
                // Sudo Key
                AccountId::from_str("6Edb3705bFFcA48af7c0aA816Ac004C2d1c48F7e").unwrap(),
                2274.into(),
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
            para_id: 2274,
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
                .map(|k| (k, EXISTENTIAL_DEPOSIT * 1000))
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
