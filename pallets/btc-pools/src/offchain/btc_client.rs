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
use crate::SignatureMap;
use bitcoin::{
	bech32::FromBase32,
	blockdata::{opcodes::all, script::Builder},
	psbt::{Prevouts, TapTree},
	util::{
		key::Secp256k1,
		sighash::{ScriptPath, SighashCache},
		taproot::{LeafVersion, TaprootBuilder},
	},
	Address, OutPoint, SchnorrSig, SchnorrSighashType, Script, Transaction, TxIn, TxOut, Txid,
	Witness, XOnlyPublicKey,
};
use electrum_client::{Client, ElectrumApi, ListUnspentRes};
use ferrum_primitives::BTC_OFFCHAIN_SIGNER_KEY_TYPE;
use reqwest;
use sp_core::{ed25519, sr25519, ByteArray, Pair, Public, H256};
use sp_io::crypto::{ecdsa_generate, ecdsa_sign_prehashed, sr25519_generate, sr25519_sign};
use sp_std::str::FromStr;

const MAX_PERMITTED_FEE_IN_SATS: u64 = 1000;

#[derive(Debug, Clone)]
pub struct BTCClient {
	pub http_api: Vec<u8>,
}

impl BTCClient {
	pub fn generate_transaction_from_withdrawal_request(
		recipient: Vec<u8>,
		amount: u32,
		validators: Vec<Vec<u8>>,
		current_pool_address: Vec<u8>,
	) -> Result<Transaction, String> {
		let secp = Secp256k1::new();

		// ensure we can connect to BTC Client
		let btc_client = Client::new("ssl://electrum.blockstream.info:60002")
			.expect("Cannot establish connection to BTC Client!");

		let taproot_script = Self::generate_taproot_script(validators);

		let builder = TaprootBuilder::with_huffman_tree(vec![(1, taproot_script.clone())]).unwrap();

		let tap_tree = TapTree::from_builder(builder).unwrap();
		let pool_pub_key = XOnlyPublicKey::from_slice(&current_pool_address).unwrap();
		let tap_info = tap_tree.into_builder().finalize(&secp, pool_pub_key).unwrap();
		let merkle_root = tap_info.merkle_root();

		let address = Address::p2tr(
			&secp,
			tap_info.internal_key(),
			tap_info.merkle_root(),
			bitcoin::Network::Testnet,
		);

		log::info!(
			"BTC Pools : Taproot calculated address {:?} | Actual pool address {:?}",
			address,
			current_pool_address
		);

		let utxos = Self::fetch_utxos(address);

		let tx_ins = Self::filter_needed_utxos(amount.into(), utxos);

		let recipient_address = String::from_utf8(recipient).expect("Found invalid UTF-8");
		let current_pool_address =
			String::from_utf8(current_pool_address).expect("Found invalid UTF-8");

		let mut tx = Transaction {
			version: 2,
			lock_time: bitcoin::PackedLockTime(0),
			input: tx_ins
				.0
				.iter()
				.map(|tx| TxIn {
					previous_output: tx.previous_output.clone(),
					script_sig: Script::new(),
					sequence: bitcoin::Sequence(0xFFFFFFFF),
					witness: Witness::default(),
				})
				.collect::<Vec<_>>(),
			output: vec![
				TxOut {
					value: amount.into(),
					script_pubkey: Address::from_str(&recipient_address).unwrap().script_pubkey(),
				},
				TxOut {
					value: tx_ins.1 - amount as u64,
					script_pubkey: Address::from_str(&current_pool_address)
						.unwrap()
						.script_pubkey(),
				},
			],
		};

		let base_fees = Self::get_network_recommended_fee().unwrap();
		let total_fees = base_fees * tx.get_size() as u64;

		log::info!("BTC Pools : Calculated Fees {:?}", total_fees);

		Ok(tx)
	}

	pub fn generate_taproot_script(validators: Vec<Vec<u8>>) -> Script {
		// we follow a simple approach here, the first validator is necessary, and the rest follow a
		// threshold model
		let mut wallet_script = Builder::new();

		// convert all validators pub key to XOnlyPubKey format
		let mut x_pub_keys = validators
			.iter()
			.map(|x| XOnlyPublicKey::from_slice(x).unwrap())
			.collect::<Vec<_>>();
		wallet_script.clone().push_x_only_key(&x_pub_keys.first().unwrap());

		// calculate the threshold value
		// since one key is required, the threshold would be the remaining keys - 1
		let threshold = x_pub_keys.len() - 2;

		// add keys with OP_CHECKSIG for all keys except last
		for key in &mut x_pub_keys[1..threshold] {
			wallet_script.clone().push_x_only_key(&key);
			wallet_script.clone().push_opcode(all::OP_CHECKSIG);
		}

		// add the last key and threshold
		wallet_script.clone().push_x_only_key(&x_pub_keys.last().unwrap());
		wallet_script.clone().push_opcode(all::OP_CHECKSIGADD);
		wallet_script.clone().push_int(threshold as i64);
		wallet_script.clone().push_opcode(all::OP_GREATERTHANOREQUAL);

		wallet_script.into_script()
	}

	pub fn fetch_utxos(address: Address) -> Vec<ListUnspentRes> {
		let client = Client::new("ssl://electrum.blockstream.info:60002").unwrap();
		let vec_tx_in = client.script_list_unspent(&address.script_pubkey()).unwrap();

		println!("Found UTXOS {:?}", vec_tx_in);

		vec_tx_in
	}

	pub fn filter_needed_utxos(
		amount: u64,
		mut available: Vec<ListUnspentRes>,
	) -> (Vec<TxIn>, u64) {
		let mut needed_utxos = vec![];
		let mut total_amount = 0;

		// sort by the oldest, we want to use the oldest first
		available.sort_by(|a, b| b.height.cmp(&a.height));

		for utxo in available {
			total_amount += utxo.value;

			needed_utxos.push(utxo);

			if total_amount >= amount {
				break
			}
		}

		let tx_ins = needed_utxos
			.iter()
			.map(|l| {
				return TxIn {
					previous_output: OutPoint::new(l.tx_hash, l.tx_pos.try_into().unwrap()),
					script_sig: Script::new(),
					sequence: bitcoin::Sequence(0xFFFFFFFF),
					witness: Witness::default(),
				}
			})
			.collect::<Vec<TxIn>>();

		(tx_ins, total_amount)
	}

	pub fn broadcast_completed_transaction(
		transaction: Vec<u8>,
		recipient: Vec<u8>,
		amount: u32,
		signatures: SignatureMap,
		current_pool_address: Vec<u8>,
	) -> Result<Txid, String> {
		let secp = Secp256k1::new();

		// ensure we can connect to BTC Client
		let btc_client = Client::new("ssl://electrum.blockstream.info:60002")
			.expect("Cannot establish connection to BTC Client!");

		let validators = signatures.clone().into_iter().map(|x| x.0).collect::<Vec<_>>();

		let taproot_script = Self::generate_taproot_script(validators);

		let builder = TaprootBuilder::with_huffman_tree(vec![(1, taproot_script.clone())]).unwrap();

		let tap_tree = TapTree::from_builder(builder).unwrap();
		let pool_pub_key = XOnlyPublicKey::from_slice(&current_pool_address.clone()).unwrap();
		let tap_info = tap_tree.into_builder().finalize(&secp, pool_pub_key).unwrap();
		let merkle_root = tap_info.merkle_root();

		let address = Address::p2tr(
			&secp,
			tap_info.internal_key(),
			tap_info.merkle_root(),
			bitcoin::Network::Testnet,
		);

		log::info!(
			"BTC Pools : Taproot calculated address {:?} | Actual pool address {:?}",
			address,
			current_pool_address
		);

		let utxos = Self::fetch_utxos(address);

		let tx_ins = Self::filter_needed_utxos(amount.into(), utxos);

		let recipient_address = String::from_utf8(recipient).expect("Found invalid UTF-8");
		let current_pool_address =
			String::from_utf8(current_pool_address).expect("Found invalid UTF-8");

		let mut tx = Transaction {
			version: 2,
			lock_time: bitcoin::PackedLockTime(0),
			input: tx_ins
				.0
				.iter()
				.map(|tx| TxIn {
					previous_output: tx.previous_output.clone(),
					script_sig: Script::new(),
					sequence: bitcoin::Sequence(0xFFFFFFFF),
					witness: Witness::default(),
				})
				.collect::<Vec<_>>(),
			output: vec![
				TxOut {
					value: amount.into(),
					script_pubkey: Address::from_str(&recipient_address).unwrap().script_pubkey(),
				},
				TxOut {
					value: tx_ins.1 - amount as u64,
					script_pubkey: Address::from_str(&current_pool_address)
						.unwrap()
						.script_pubkey(),
				},
			],
		};

		let prev_tx = tx
			.clone()
			.input
			.iter()
			.map(|tx_id| btc_client.transaction_get(&tx_id.previous_output.txid).unwrap())
			.collect::<Vec<Transaction>>();

		let tx_out_of_prev_tx =
			prev_tx.clone().iter().map(|tx| tx.output[0].clone()).collect::<Vec<TxOut>>();

		let binding = tx_out_of_prev_tx;
		let prevouts = Prevouts::All(&binding);

		let sighash_sig = SighashCache::new(&mut tx.clone())
			.taproot_script_spend_signature_hash(
				0,
				&prevouts,
				ScriptPath::with_defaults(&taproot_script),
				SchnorrSighashType::Default,
			)
			.unwrap();

		let key_sig = SighashCache::new(&mut tx.clone())
			.taproot_key_spend_signature_hash(0, &prevouts, SchnorrSighashType::Default)
			.unwrap();

		let actual_control = tap_info
			.control_block(&(taproot_script.clone(), LeafVersion::TapScript))
			.unwrap();

		let mut witnesses = vec![];

		// TODO : We need to add more checks here to ensure the sigs match the stack, the order is
		// not always guaranteed The order is maintained via the insertion but should check again to
		// be sure (it's a stack, so this is Last In First Out, and will be consumed by the first
		// CHECKSIGVERIFY)
		for signature in signatures {
			witnesses.push(
				SchnorrSig {
					sig: secp256k1::schnorr::Signature::from_slice(&signature.1).unwrap(),
					hash_ty: SchnorrSighashType::Default,
				}
				.to_vec(),
			)
		}

		witnesses.push(taproot_script.to_bytes());
		witnesses.push(actual_control.serialize());

		let wit = Witness::from_vec(witnesses);

		for mut input in tx.clone().input.into_iter() {
			input.witness = wit.clone();
		}

		// final sanity checks, ensure our fee is sane
        // TODO : Improve this to that we increase fee when a tx is delayed
		let min_fees = btc_client.estimate_fee(tx.get_size());
		let rec_base_fees = Self::get_network_recommended_fee().unwrap();
		let total_fees = rec_base_fees * tx.get_size() as u64;

		if (total_fees > MAX_PERMITTED_FEE_IN_SATS) {
			panic!("Cannot spend fee above limit!")
		}

		log::info!("BTC Pools : Calculated Fees {:?}", total_fees);

		// Broadcast tx
		let tx_id = btc_client.transaction_broadcast(&tx).unwrap();
		println!("transaction hash: {}", tx_id.to_string());

		Ok(tx_id)
	}

	fn get_network_recommended_fee() -> Result<u64, ()> {
		let api_url = "https://api.blockchain.info/mempool/fees";

		#[derive(serde::Serialize, serde::Deserialize)]
		struct MempoolFees {
			regular: u64,
			priority: u64,
		}

		// Make the HTTP GET request
		let response: MempoolFees = reqwest::blocking::get(api_url).unwrap().json().unwrap();
		return Ok(response.regular)
	}
}

// #[derive(Clone)]
pub struct BTCClientSignature {
	pub from: sr25519::Public,
	pub _signer: sr25519::Public,
}

impl BTCClientSignature {
	pub fn new(from: sr25519::Public, signer: &[u8]) -> Self {
		BTCClientSignature { from, _signer: sr25519::Public::try_from(signer).unwrap() }
	}

	pub fn sign(&self, hash: &[u8]) -> Result<sr25519::Signature, ()> {
		log::info!("Signer address is : {:?}", self.from);
		// TODO : We should handle this properly, if the signing is not possible maybe propogate the
		// error upstream
		let signed = sr25519_sign(BTC_OFFCHAIN_SIGNER_KEY_TYPE, &self.from, &hash).unwrap();
		Ok(signed)
	}
}

impl From<sr25519::Public> for BTCClientSignature {
	fn from(signer: sr25519::Public) -> Self {
		log::info!("PUBLIC KEY {:?}", signer);

		BTCClientSignature { _signer: signer, from: signer }
	}
}
