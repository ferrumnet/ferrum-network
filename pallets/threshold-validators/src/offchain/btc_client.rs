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
use bitcoin::{
	bech32::FromBase32,
	blockdata::{opcodes::all, script::Builder},
	psbt::{Prevouts, TapTree},
	util::{
		key::Secp256k1,
		sighash::{ScriptPath, SighashCache},
		taproot::{LeafVersion, TaprootBuilder},
	},
	Address, Amount, OutPoint, PublicKey, SchnorrSig, SchnorrSighashType, Script, Transaction,
	TxIn, TxOut, Txid, Witness, XOnlyPublicKey,
};
use bitcoincore_rpc::bitcoin::address::NetworkUnchecked;
use electrum_client::{Client, ElectrumApi, ListUnspentRes};
use ferrum_primitives::BTC_OFFCHAIN_SIGNER_KEY_TYPE;
use reqwest;
use serde::{Deserialize, Serialize};
use sp_core::{ed25519, sr25519, ByteArray, Pair, Public, H256};
use sp_io::crypto::{ecdsa_generate, ecdsa_sign_prehashed, sr25519_generate, sr25519_sign};
use sp_std::str::FromStr;

const MAX_PERMITTED_FEE_IN_SATS: u64 = 1000;

#[derive(Debug, Clone)]
pub struct BTCClient {
	pub http_api: Vec<u8>,
}

// taken from https://bitcoincore.org/en/doc/0.16.2/rpc/wallet/listreceivedbyaddress/
// rust types for bitcoin received by address list
#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct ListReceivedByAddressResult {
	pub involvesWatchonly: bool,
	pub address: Address,
	pub amount: u64,
	pub confirmations: u32,
	pub label: String,
	pub txids: Vec<bitcoin::Txid>,
}

impl BTCClient {
	pub fn get_incoming_transactions(
		current_pool_address: Vec<u8>,
	) -> Result<ListReceivedByAddressResult, String> {
		// Bitcoin RPC configuration
		let rpc_url = "http://rpcuser:rpcpassword@127.0.0.1:8332";

		// JSON-RPC request payload for listreceivedbyaddress
		let json_rpc_request = serde_json::json!({
			"jsonrpc": "2.0",
			"id": "1",
			"method": "listreceivedbyaddress",
			"params": [0, false, false], // minconf, include_empty, watch_only
		});

		// Make the HTTP POST request to the Bitcoin RPC endpoint
		// see : https://bitcoincore.org/en/doc/0.16.2/rpc/wallet/listreceivedbyaddress/
		let response: Result<ListReceivedByAddressResult, String> =
			reqwest::blocking::Client::new()
				.post(rpc_url)
				.json(&json_rpc_request)
				.send()
				.unwrap()
				.json()
				.unwrap();

		response
	}

	pub fn get_transaction_details(tx_id: Txid) -> Result<Transaction, String> {
		// ensure we can connect to BTC Client
		let btc_client = Client::new("ssl://electrum.blockstream.info:60002")
			.expect("Cannot establish connection to BTC Client!");

		btc_client.transaction_get(&tx_id).map_err(|_| "BTC Get TX failed!".to_string())
	}

	// Extract public key from Bitcoin script
	// TODO : Just for POC, this needs to handle all possible cases
	pub fn extract_public_key_from_script(script: &Vec<u8>) -> Option<PublicKey> {
		// Assuming the script contains a standard pay-to-public-key (P2PK) script
		if script.len() > 0 && script[0] == 0x41 {
			// Check if the script is a standard P2PK script
			if let Ok(pubkey) = PublicKey::from_slice(&script[1..]) {
				return Some(pubkey)
			}
		}

		None
	}
}
