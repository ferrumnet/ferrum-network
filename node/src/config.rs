use std::{fs::File, io::BufReader, path::Path};

use serde::{Deserialize, Serialize};

use pallet_quantum_portal::qp_types::{EIP712Config, QpConfig, QpNetworkItem};

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub networks: NetworkConfig,
}

#[derive(Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NetworkConfig {
    // The NetworkItem data structure
    network_vec: Vec<NetworkItem>,
    // The pair of ChainIds to mine
    pair_vec: Vec<(u64, u64)>,
    // The public key for the signer account
    #[serde(with = "serde_bytes")]
    pub signer_public_key: Vec<u8>,
    // EIP712 config
    #[serde(with = "serde_bytes")]
    pub authority_manager_contract_name: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pub authority_manager_contract_version: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pub authority_manager_contract_address: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pub miner_manager_contract_name: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pub miner_manager_contract_version: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pub miner_manager_contract_address: Vec<u8>,
    /// The role of this node
    #[serde(with = "serde_bytes")]
    pub role: Vec<u8>,
}

pub fn convert(network_config: NetworkConfig) -> QpConfig {
    let role_as_bytes: &[u8] = &network_config.role;
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
        eip_712_config: EIP712Config {
            finalizer_contract_name: network_config.authority_manager_contract_name,
            finalizer_contract_version: network_config.authority_manager_contract_version,
            finalizer_verifying_address: network_config.authority_manager_contract_address,

            miner_contract_name: network_config.miner_manager_contract_name,
            miner_contract_version: network_config.miner_manager_contract_version,
            miner_verifying_address: network_config.miner_manager_contract_address,
        },
        role: role_as_bytes.into(),
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
