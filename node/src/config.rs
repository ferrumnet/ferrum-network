use std::{fs::File, io::BufReader, path::Path};

use serde::Deserialize;

use pallet_quantum_portal::qp_types::{EIP712Config, QpConfig, QpNetworkItem};

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub chain_spec: ChinSpecConfig,
    pub networks: NetworkConfig,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ChinSpecConfig {
    /// Secret seed for initial authorities [aura, grandpa]
    pub initial_authourity_seed_list: Vec<String>,
    /// AccountId of the Sudo authority
    pub root_seed: String,
    /// List of AccountId to populate balances in genesis block
    pub endowed_accounts_seed_list: Vec<String>,
    /// List of AccountIds for EVM configuration
    pub address_list: Vec<String>,
    /// Secret seed for offchain signer key
    pub offchain_signer_secret_seed: String,
}

#[derive(Clone, Eq, PartialEq, Debug, Deserialize)]
pub struct NetworkItem {
    /// The rpc url for this network
    #[serde(with = "serde_bytes")]
    pub url: Vec<u8>,
    /// The ledger_manager contract address for this network
    #[serde(with = "serde_bytes")]
    pub ledger_manager: Vec<u8>,
    /// The ChainId for this network
    pub id: u64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct NetworkConfig {
    // The NetworkItem data structure
    network_vec: Vec<NetworkItem>,
    // The pair of ChainIds to mine
    pair_vec: Vec<(u64, u64)>,
    // The public key for the signer account
    #[serde(with = "serde_bytes")]
    pub signer_public_key: Vec<u8>,
    // EIP712 config
    pub eip_712_config: EIP712Config,
}

pub fn convert(network_config: NetworkConfig) -> QpConfig {
    QpConfig {
        network_vec: network_config
            .network_vec
            .into_iter()
            .map(|network_item| QpNetworkItem {
                url: network_item.url,
                ledger_manager: network_item.ledger_manager,
                id: network_item.id,
            })
            .collect(),
        pair_vec: network_config.pair_vec,
        signer_public_key: network_config.signer_public_key,
        eip_712_config: network_config.eip_712_config,
    }
}

pub fn read_config_from_file<P: AsRef<Path>>(path: P) -> Result<Config, String> {
    match File::open(path) {
        Ok(file) => {
            let reader = BufReader::new(file);

            match serde_json::from_reader(reader) {
                Ok(config) => Ok(config),
                Err(err) => Err(err.to_string()),
            }
        }
        Err(err) => Err(err.to_string()),
    }
}
