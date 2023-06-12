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
use pallet_quantum_portal::qp_types::{QpConfig, QpNetworkItem};
use serde::{Deserialize, Serialize};
use std::{fs::File, io::BufReader, path::Path};

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub networks: NetworkConfig,
}

#[derive(Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub struct NetworkItem {
    /// The rpc url for this network
    #[serde(with = "serde_bytes")]
    pub url: Vec<u8>,
    /// The gateway_contract_address contract address for this network
    #[serde(with = "serde_bytes")]
    pub gateway_contract_address: Vec<u8>,
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
                gateway_contract_address: network_item.gateway_contract_address,
                id: network_item.id,
            })
            .collect(),
        pair_vec: network_config.pair_vec,
        signer_public_key: network_config.signer_public_key,
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
