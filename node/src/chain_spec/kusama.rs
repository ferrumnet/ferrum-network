use super::*;
use cumulus_primitives_core::ParaId;
use ferrum_runtime::{
	AccountId, AuraId, EthereumConfig, ParachainInfoConfig, RuntimeGenesisConfig,
	EXISTENTIAL_DEPOSIT,
};
use hex_literal::hex;
use sc_chain_spec::Properties;
use sc_service::ChainType;
use sp_application_crypto::ByteArray;
use std::str::FromStr;

/// Specialized `ChainSpec` for the normal parachain runtime.
pub type KusamaChainSpec = sc_service::GenericChainSpec<(), Extensions>;

/// Generate the session keys from individual elements.
///
/// The input must be a tuple of individual keys (a single arg for now since we have just one key).
pub fn ferrum_session_keys(keys: AuraId) -> ferrum_runtime::SessionKeys {
	ferrum_runtime::SessionKeys { aura: keys }
}

fn properties() -> Properties {
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("tokenSymbol".into(), "tqpFRM".into());
	properties.insert("tokenDecimals".into(), 18.into());
	properties.insert("ss58Format".into(), 42.into());
	properties
}

pub fn kusama_local_config() -> KusamaChainSpec {
	#[allow(deprecated)]
	KusamaChainSpec::builder(
		ferrum_runtime::WASM_BINARY.expect("WASM binary was not built, please build it!"),
		Extensions {
			relay_chain: "rococo-local".into(),
			// You MUST set this to the correct network!
			para_id: 1000,
		},
	)
	.with_name("Quantum Portal Network Local")
	.with_id("quantum_portal_network_local")
	.with_chain_type(ChainType::Live)
	.with_properties(properties())
	.with_genesis_config_patch(generate_genesis(
		// Sudo Key
		AccountId::from_str("8097c3C354652CB1EEed3E5B65fBa2576470678A").unwrap(),
		// Pre-funded accounts
		vec![AccountId::from_str("8097c3C354652CB1EEed3E5B65fBa2576470678A").unwrap()],
		// Initial PoA authorities
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
		1000.into(),
	))
	.build()
}

pub fn kusama_config() -> KusamaChainSpec {
	// Give your base currency a QPN name and decimal places
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("tokenSymbol".into(), "qpFRM".into());
	properties.insert("tokenDecimals".into(), 18.into());
	properties.insert("ss58Format".into(), 42.into());

	#[allow(deprecated)]
	KusamaChainSpec::builder(
		ferrum_runtime::WASM_BINARY.expect("WASM binary was not built, please build it!"),
		Extensions {
			relay_chain: "kusama".into(),
			// You MUST set this to the correct network!
			para_id: 2274,
		},
	)
	.with_name("Quantum Portal Network")
	.with_id("quantum_portal_network")
	.with_chain_type(ChainType::Live)
	.with_properties(properties)
	.with_genesis_config_patch(generate_genesis(
		// Sudo Key
		AccountId::from_str("8097c3C354652CB1EEed3E5B65fBa2576470678A").unwrap(),
		// Pre-funded accounts
		vec![AccountId::from_str("8097c3C354652CB1EEed3E5B65fBa2576470678A").unwrap()],
		// Initial PoA authorities
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
		2274.into(),
	))
	.build()
}

fn generate_genesis(
	root_key: AccountId,
	endowed_accounts: Vec<AccountId>,
	invulnerables: Vec<(AccountId, AuraId)>,
	id: ParaId,
) -> serde_json::Value {
	serde_json::json!({
		"balances": {
			"balances": endowed_accounts.iter().cloned().map(|k| (k, EXISTENTIAL_DEPOSIT * 1000)).collect::<Vec<_>>(),
		},
		"parachainInfo": {
			"parachainId": id,
		},
		"collatorSelection": {
			"invulnerables": invulnerables.iter().cloned().map(|(acc, _)| acc).collect::<Vec<_>>(),
			"candidacyBond": EXISTENTIAL_DEPOSIT * 16,
		},
		"session": {
			"keys": invulnerables
				.into_iter()
				.map(|(acc, aura)| {
					(
						acc.clone(),                 // account id
						acc,                         // validator id
						ferrum_session_keys(aura), // session keys
					)
				})
			.collect::<Vec<_>>(),
		},
		"polkadotXcm": {
			"safeXcmVersion": Some(SAFE_XCM_VERSION),
		},
		"sudo": { "key": Some(root_key) }
	})
}
