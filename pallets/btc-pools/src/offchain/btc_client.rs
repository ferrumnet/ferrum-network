// No-std compatible client for bitcoin rpc
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::iter::FromIterator;
use std::path::PathBuf;
use std::{fmt, result};

use crate::{bitcoin, deserialize_hex};
use bitcoin::hex::DisplayHex;
use jsonrpc;
use serde;
use serde_json;

use crate::bitcoin::address::{NetworkUnchecked, NetworkChecked};
use crate::bitcoin::hashes::hex::FromHex;
use crate::bitcoin::secp256k1::ecdsa::Signature;
use crate::bitcoin::{
    Address, Amount, Block, OutPoint, PrivateKey, PublicKey, Script, Transaction,
};
use log::Level::{Debug, Trace, Warn};
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};

// Replace these values with your own
const RPC_URL: &str = "http://127.0.0.1:8332";
const RPC_USER: &str = "bitcoin";
const RPC_PASSWORD: &str = "talk";

use crate::error::*;
use crate::json;
use crate::queryable;

/// Crate-specific Result type, shorthand for `std::result::Result` with our
/// crate-specific Error type;
pub type Result<T> = result::Result<T, Error>;

/// Outpoint that serializes and deserializes as a map, instead of a string,
/// for use as RPC arguments
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JsonOutPoint {
    pub txid: bitcoin::Txid,
    pub vout: u32,
}

impl From<OutPoint> for JsonOutPoint {
    fn from(o: OutPoint) -> JsonOutPoint {
        JsonOutPoint {
            txid: o.txid,
            vout: o.vout,
        }
    }
}

impl Into<OutPoint> for JsonOutPoint {
    fn into(self) -> OutPoint {
        OutPoint {
            txid: self.txid,
            vout: self.vout,
        }
    }
}

/// Shorthand for converting a variable into a serde_json::Value.
fn into_json<T>(val: T) -> Result<serde_json::Value>
    where
        T: serde::ser::Serialize,
{
    Ok(serde_json::to_value(val)?)
}

/// Shorthand for converting an Option into an Option<serde_json::Value>.
fn opt_into_json<T>(opt: Option<T>) -> Result<serde_json::Value>
    where
        T: serde::ser::Serialize,
{
    match opt {
        Some(val) => Ok(into_json(val)?),
        None => Ok(serde_json::Value::Null),
    }
}

/// Shorthand for `serde_json::Value::Null`.
fn null() -> serde_json::Value {
    serde_json::Value::Null
}

/// Shorthand for an empty serde_json::Value array.
fn empty_arr() -> serde_json::Value {
    serde_json::Value::Array(vec![])
}

/// Shorthand for an empty serde_json object.
fn empty_obj() -> serde_json::Value {
    serde_json::Value::Object(Default::default())
}

// Function to get a raw transaction by transaction ID
fn get_raw_transaction(txid: &str) -> Result<String, reqwest::Error> {
    // Create the RPC request URL
    let url = format!("{}/{}", RPC_URL, "rest/tx/".to_owned() + txid + ".json");

    // Create the RPC authorization header
    let mut headers = HeaderMap::new();
    let auth_value = format!(
        "Basic {}",
        base64::encode(format!("{}:{}", RPC_USER, RPC_PASSWORD))
    );
    headers.insert(AUTHORIZATION, HeaderValue::from_str(&auth_value).unwrap());

    // Make the RPC request
    let client = Client::new();
    let response = client.get(&url).headers(headers).send()?;

    // Check if the request was successful
    if response.status().is_success() {
        // Parse the JSON response to extract the raw transaction
        let json: serde_json::Value = response.json()?;
        if let Some(tx) = json.get("rawtx") {
            if let Some(raw_transaction) = tx.as_str() {
                return Ok(raw_transaction.to_string());
            }
        }
    }

    Err(reqwest::Error::new(reqwest::StatusCode::INTERNAL_SERVER_ERROR, "Failed to get raw transaction"))
}

// Function to scan for UTXOs associated with a given address
fn scan_for_utxos(address: &str) -> Result<Vec<Utxo>, reqwest::Error> {
    // Create the RPC request URL
    let url = format!("{}/{}", RPC_URL, "rest/scantxoutset/scan");

    // Create the RPC request payload
    let payload = format!(
        r#"{{"action":"start","scanobjects":["addr({})"]}}"#,
        address
    );

    // Create the RPC authorization header
    let mut headers = HeaderMap::new();
    let auth_value = format!(
        "Basic {}",
        base64::encode(format!("{}:{}", RPC_USER, RPC_PASSWORD))
    );
    headers.insert(AUTHORIZATION, HeaderValue::from_str(&auth_value).unwrap());

    // Make the RPC request
    let client = Client::new();
    let response = client
        .post(&url)
        .headers(headers)
        .body(payload)
        .send()?;

    // Check if the request was successful
    if response.status().is_success() {
        println!("Scan for UTXOs successful!");
        let utxos = response.unwrap().unspents;
	    println!("Fetched utxos {:?}", utxos);
    } else {
        eprintln!(
            "Error scanning for UTXOs: {}",
            response.text().unwrap_or_else(|_| String::from("Unknown error"))
        );
    }

    Ok(())
}

fn test_mempool_accept(tx_hex: &str) -> Result<(), reqwest::Error> {
    // Create the RPC request URL
    let url = format!("{}/{}", RPC_URL, "rest/testmempoolaccept");

    // Create the RPC request payload
    let payload = format!(r#"{{"rawtx":"{}"}}"#, tx_hex);

    // Create the RPC authorization header
    let mut headers = HeaderMap::new();
    let auth_value = format!(
        "Basic {}",
        base64::encode(format!("{}:{}", RPC_USER, RPC_PASSWORD))
    );
    headers.insert(AUTHORIZATION, HeaderValue::from_str(&auth_value).unwrap());

    // Make the RPC request
    let client = Client::new();
    let response = client
        .post(&url)
        .headers(headers)
        .body(payload)
        .send()?;

    // Check if the request was successful
    if response.status().is_success() {
        println!("Test mempool accept successful!");
    } else {
        eprintln!(
            "Error sending test mempool accept: {}",
            response.text().unwrap_or_else(|_| String::from("Unknown error"))
        );
    }

    Ok(())
}

// Function to generate a Bitcoin taproot transaction with multisig script
fn generate_taproot_transaction(signer_key: PublicKey, utxos : Vec<Utxo>, amount : Amount, recipient: Address) -> Transaction {
    // Bitcoin network
    let network = Network::Testnet;

    let out_point = OutPoint {
        txid: Default::default(),
        vout: 0,
    };
    let tx_in = TxIn {
        previous_output: out_point,
        script_sig: Default::default(),
        sequence: 0xFFFFFFFF,
        witness: Vec::new(),
    };

    // Transaction output (dummy values, replace with real values)
    let script_pubkey = Builder::new().push_opcode(opcodes::all::OP_TRUE).into_script();
    let tx_out = TxOut {
        value: amount,
        script_pubkey,
    };

    // Create a taproot transaction
    let mut transaction = Transaction {
        version: 2,
        lock_time: 0,
        input: vec![tx_in],
        output: vec![tx_out],
    };

    // Add tapscript (multisig script) to witness
    let secp = Secp256k1::new();
    let witness_script = Builder::new()
        .push_key(&signer1_key)
        .push_key(&signer2_key)
        .push_opcode(opcodes::all::OP_2)
        .push_opcode(opcodes::all::OP_CHECKMULTISIG)
        .into_script();
    let mut witness_data = Vec::new();
    witness_data.push(witness_script.to_bytes().unwrap());
    transaction.input[0].witness = witness_data;

    // Sign the transaction
    let mut rng = OsRng::new().expect("Failed to create OS RNG");
    let message = transaction.txid();
    let sig1 = secp.sign_recoverable(&message, &signer1_key, &mut rng);
    let sig2 = secp.sign_recoverable(&message, &signer2_key, &mut rng);

    // Add signatures to witness
    transaction.input[0].witness.push(serialize(&sig1).unwrap());
    transaction.input[0].witness.push(serialize(&sig2).unwrap());

    transaction
}