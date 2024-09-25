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
use super::*;
use cumulus_primitives_core::ParaId;
use ferrum_testnet_runtime::{
	AccountId, AuraId, EthereumConfig, ParachainInfoConfig, RuntimeGenesisConfig,
	EXISTENTIAL_DEPOSIT,
};
use sc_service::ChainType;
use std::str::FromStr;

/// Specialized `TestnetChainSpec` for the normal parachain runtime.
pub type TestnetChainSpec = sc_service::GenericChainSpec<(), Extensions>;

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

	#[allow(deprecated)]
	TestnetChainSpec::builder(
		ferrum_runtime::WASM_BINARY.expect("WASM binary was not built, please build it!"),
		Extensions {
			relay_chain: "rococo-local".into(),
			// You MUST set this to the correct network!
			para_id: 1000,
		},
	)
	.with_name("Quantum Portal Network Testnet")
	.with_id("quantum_portal_network_testnet")
	.with_chain_type(ChainType::Live)
	.with_properties(properties)
	.with_genesis_config_patch(testnet_genesis(
		// Sudo Key
		AccountId::from_str("e04cc55ebee1cbce552f250e85c57b70b2e2625b").unwrap(),
		// Pre-funded accounts
		vec![AccountId::from_str("e04cc55ebee1cbce552f250e85c57b70b2e2625b").unwrap()],
		// Initial PoA authorities
		vec![
			(
				AccountId::from_str("e04cc55ebee1cbce552f250e85c57b70b2e2625b").unwrap(),
				get_collator_keys_from_seed("Alice"),
			),
			(
				AccountId::from_str("977D8B2C924dB8a92340e9bb58e6C0d876de9D60").unwrap(),
				get_collator_keys_from_seed("Bob"),
			),
		],
		1000.into(),
	))
	.build()
}

#[allow(dead_code)]
pub fn local_testnet_config() -> TestnetChainSpec {
	// Give your base currency a tFRM name and decimal places
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("tokenSymbol".into(), "tFRM".into());
	properties.insert("tokenDecimals".into(), 18.into());
	properties.insert("ss58Format".into(), 42.into());

	#[allow(deprecated)]
	TestnetChainSpec::builder(
		ferrum_runtime::WASM_BINARY.expect("WASM binary was not built, please build it!"),
		Extensions {
			relay_chain: "rococo-local".into(),
			// You MUST set this to the correct network!
			para_id: 1000,
		},
	)
	.with_name("Quantum Portal Network Local")
	.with_id("quantum_portal_network_local")
	.with_chain_type(ChainType::Local)
	.with_properties(properties)
	.with_genesis_config_patch(testnet_genesis(
		// Sudo Key
		AccountId::from_str("e04cc55ebee1cbce552f250e85c57b70b2e2625b").unwrap(),
		// Pre-funded accounts
		vec![AccountId::from_str("e04cc55ebee1cbce552f250e85c57b70b2e2625b").unwrap()],
		// Initial PoA authorities
		vec![
			(
				AccountId::from_str("e04cc55ebee1cbce552f250e85c57b70b2e2625b").unwrap(),
				get_collator_keys_from_seed("Alice"),
			),
			(
				AccountId::from_str("977D8B2C924dB8a92340e9bb58e6C0d876de9D60").unwrap(),
				get_collator_keys_from_seed("Bob"),
			),
		],
		1000.into(),
	))
	.build()
}

pub fn alpha_testnet_config() -> TestnetChainSpec {
	// Give your base currency a tFRM name and decimal places
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("tokenSymbol".into(), "tFRM".into());
	properties.insert("tokenDecimals".into(), 18.into());
	properties.insert("ss58Format".into(), 42.into());

	#[allow(deprecated)]
	TestnetChainSpec::builder(
		ferrum_runtime::WASM_BINARY.expect("WASM binary was not built, please build it!"),
		Extensions {
			relay_chain: "rococo-local".into(),
			// You MUST set this to the correct network!
			para_id: 1000,
		},
	)
	.with_name("Quantum Portal Network Testnet")
	.with_id("quantum_portal_network_testnet")
	.with_chain_type(ChainType::Live)
	.with_properties(properties)
	.with_genesis_config_patch(testnet_genesis(
		// Sudo Key
		AccountId::from_str("e04cc55ebee1cbce552f250e85c57b70b2e2625b").unwrap(),
		// Pre-funded accounts
		vec![AccountId::from_str("e04cc55ebee1cbce552f250e85c57b70b2e2625b").unwrap()],
		// Initial PoA authorities
		vec![
			(
				AccountId::from_str("e04cc55ebee1cbce552f250e85c57b70b2e2625b").unwrap(),
				get_collator_keys_from_seed("Alice"),
			),
			(
				AccountId::from_str("977D8B2C924dB8a92340e9bb58e6C0d876de9D60").unwrap(),
				get_collator_keys_from_seed("Bob"),
			),
		],
		1000.into(),
	))
	.build()
}

fn testnet_genesis(
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
