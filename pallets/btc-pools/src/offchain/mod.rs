use bitcoin::{
    absolute,
    bip32::{ChildNumber, DerivationPath, Fingerprint, Xpriv, Xpub},
    consensus,
    consensus::encode,
    ecdsa,
    ecdsa::Signature,
    hashes::Hash,
    key::{TapTweak, XOnlyPublicKey},
    opcodes::all::{
        OP_CHECKSIG, OP_CHECKSIGADD, OP_CHECKSIGVERIFY, OP_CLTV, OP_DROP, OP_GREATERTHANOREQUAL,
        OP_PUSHDATA1,
    },
    psbt::{self, Input, Output, Psbt, PsbtSighashType},
    script,
    secp256k1::{Secp256k1, Signing, Verification},
    sighash::{self, SighashCache, TapSighash, TapSighashType},
    taproot::{self, LeafVersion, TapLeafHash, TaprootBuilder, TaprootSpendInfo},
    transaction, Address, Amount, Network,
    Network::Regtest,
    OutPoint, PublicKey, Script, ScriptBuf, Transaction, TxIn, TxOut, Witness,
};
use sha256::{digest, try_digest};
use std::{env, thread, time};
use subxt_signer::sr25519::dev;

mod btc_rpc;
mod btc_client;

async fn btc_offchain_handler() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    println!("args {:?}", args);
    let key_type = args[1].clone();

    let key = if key_type == "alice" {
        dev::alice()
    } else {
        dev::bob()
    };

    let utxos =
        btc_rpc::fetch_utxos("bcrt1qyjnn9rkdnge9gnxjwghkk4zpp3e9yzrfnkld54".to_string()).await?;
    println!("Found {} utxos for address", utxos.len());

    loop {
        let withdrawals = subxt::get_pending_withdrawals().await?;

        if let Some((address, amount)) = withdrawals {
            let tx_hash = generate_taproot_output_transaction(address.clone(), amount.clone());
            // post tx on chain
            println!("Transaction hash {:?}", hex::encode(tx_hash.clone()));
            subxt::submit_transaction(address, amount, tx_hash.clone(), key.clone()).await?;

            // generate signature
            let sign = generate_signature(tx_hash.clone());
            println!("Transaction signature {:?}", hex::encode(sign.clone()));

            // post signature onchain
            subxt::submit_signature(tx_hash, sign, key.clone()).await?;
        } else {
            println!("No withdrawals found!")
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(5000)).await;
    }

    Ok(())
}

fn generate_taproot_output_transaction(
    recipient: Vec<u8>,
    amount: u32,
    utxos: Vec<Utxo>,
) -> Vec<u8> {
    println!("Generating a new taproot transaction");

    // TODO : Fetch this key from onchain
    let current_pool_key = "bcrt1q6wvagdd2k7un2ut8lvq8748mv7cvvxdrhaj46c";

    let selected_utxos = btc_rpc::filter_utxos(utxos, Amount::new(amount));

    // Your logic to create a Taproot transaction using the withdrawal and Taproot details.
    // This is a placeholder, replace it with actual implementation based on your requirements.

    let bytes = bincode::serialize(&transaction).unwrap();
    digest(bytes).into()
}

fn generate_signature(tx_hash: Vec<u8>) -> Vec<u8> {
    println!("Generating a new taproot signature");
    let from = dev::alice();
    let sign = from.sign(&tx_hash);
    return sign.as_ref().into();
}

// Define the struct
#[derive(serde::Serialize, serde::Deserialize)]
struct TaprootTransaction {
    // Transaction details
    version: u32,
    inputs: Vec<Utxo>,
    outputs: Vec<TaprootOutput>,
    lock_time: u32,
}

// Output struct
#[derive(serde::Serialize, serde::Deserialize)]
struct TaprootOutput {
    value: u64,
    script_pubkey: String,
}
