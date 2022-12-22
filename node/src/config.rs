use std::{fs::File, io::BufReader, path::Path};

use serde::Deserialize;

use pallet_quantum_portal::qp_types::{QpConfig, QpNetworkItem};

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub chain_spec: ChinSpecConfig,
    pub networks: NetworkConfig,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ChinSpecConfig {
    pub initial_authourity_seed_list: Vec<String>,
    pub root_seed: String,
    pub endowed_accounts_seed_list: Vec<String>,
    pub address_list: Vec<String>,
}

#[derive(Clone, Eq, PartialEq, Debug, Deserialize)]
pub struct NetworkItem {
    /// The rpc url for this network
    #[serde(with = "serde_bytes")]
    pub url: Vec<u8>,
    /// The ledger_manager contract address for this network
    #[serde(with = "serde_bytes")]
    pub ledger_manager: Vec<u8>,
    /// The authority_mananger contract address for this network
    #[serde(with = "serde_bytes")]
    pub authority_manager: Vec<u8>,
    // The public key for the signer account
    #[serde(with = "serde_bytes")]
    pub signer_public_key: Vec<u8>,
    /// The ChainId for this network
    pub id: u64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct NetworkConfig {
    network_vec: Vec<NetworkItem>,
    pair_vec: Vec<(u64, u64)>,
}

pub fn convert(network_config: NetworkConfig) -> QpConfig {
    QpConfig {
        network_vec: network_config
            .network_vec
            .into_iter()
            .map(|network_item| QpNetworkItem {
                url: network_item.url,
                ledger_manager: network_item.ledger_manager,
                authority_manager: network_item.authority_manager,
                signer_public_key: network_item.signer_public_key,
                id: network_item.id,
            })
            .collect(),
        pair_vec: network_config.pair_vec,
    }
}

pub fn read_config_from_file<P: AsRef<Path>>(path: P) -> Result<Config, String> {
    match File::open(path) {
        Ok(file) => {
            let reader = BufReader::new(file);

            match serde_json::from_reader(reader) {
                Ok(config) => return Ok(config),
                Err(err) => return Err(err.to_string()),
            }
        }
        Err(err) => Err(err.to_string()),
    }
}
