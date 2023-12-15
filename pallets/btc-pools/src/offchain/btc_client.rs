use bincode::config::LittleEndian;
use bitcoin::{
	bech32::FromBase32,
	blockdata::{opcodes::all, script::Builder},
	hashes::{
		hex,
		hex::{FromHex, ToHex},
		sha256, Hash,
	},
	psbt::{serialize::Deserialize, Input, Output, PartiallySignedTransaction, TapTree},
	schnorr::{TapTweak, TweakedKeyPair, TweakedPublicKey, UntweakedKeyPair},
	secp256k1::{Message, Parity, Secp256k1, SecretKey, Signature},
	util::{
		bip32::ExtendedPrivKey,
		sighash::{Prevouts, ScriptPath, SighashCache},
		taproot::{
			ControlBlock, LeafVersion, LeafVersion::TapScript, NodeInfo, TapBranchHash,
			TapBranchTag, TapLeafHash, TapSighashHash, TaprootBuilder, TaprootMerkleBranch,
			TaprootSpendInfo,
		},
	},
	Address, AddressType, KeyPair, Network, OutPoint, PrivateKey, SchnorrSig, SchnorrSighashType,
	Script, Transaction, TxIn, TxOut, Txid, Witness, XOnlyPublicKey,
};
use electrum_client::{Client, ElectrumApi};
use miniscript::{
	psbt::{PsbtExt, PsbtInputSatisfier},
	Descriptor, DescriptorPublicKey, Miniscript, Tap, ToPublicKey,
};
use std::{collections::BTreeMap, str::FromStr};
use bitcoin::Amount;
use std::{env, thread, time};

#[derive(Debug, Clone)]
pub struct BTCClient {
    pub http_api: Vec<u8>,
}

impl BTCClient {
    /// Generate taproot script from given threshold and authorities
    fn generate_taproot_script(authorities : Vec<Vec<u8>>, threshold : u32) -> Vec<u8> {
        debug_assert!(authorities.len() > 2);

        // generate the taproot script
        let mut wallet_script = Builder::new();

        // the first authority signature is always required
        wallet_script.push_x_only_key(&authorities[0].into())
		wallet_script.push_opcode(all::OP_CHECKSIGVERIFY)

        // add all the remaining authorities except last one
        for authorities in authorities[1..=authorities.len() - 2] {
            wallet_script.push_x_only_key(&authorities[0].into())
		    wallet_script.push_opcode(all::OP_CHECKSIG)
        }

        // add the remaining authority
        wallet_script.push_x_only_key(&authorities[authorities.len() - 1].into())
		wallet_script.push_opcode(all::OP_CHECKSIGADD)

        wallet_script.push_int(threshold)
		wallet_script.push_opcode(all::OP_GREATERTHANOREQUAL)
		wallet_script.into_script()
    }


    // generate a wallet address from given taproot script
    fn generate_wallet_address(taproot_script : Vec<u8>) -> Vec<u8> {
        println!("Script {:?}", taproot_script);

        // TODO : genreate random key and hash two times
        let internal_secret =
		SecretKey::from_str("1229101a0fcf2104e8808dab35661134aa5903867d44deb73ce1c7e4eb925be8")
			.unwrap();

        let internal = KeyPair::from_secret_key(&secp, &internal_secret);

        let builder = TaprootBuilder::with_huffman_tree(vec![(1, taproot_script.clone())]).unwrap();
        let tap_tree = TapTree::from_builder(builder).unwrap();
        let tap_info = tap_tree.into_builder().finalize(&secp, internal.public_key().into()).unwrap();
        let merkle_root = tap_info.merkle_root();
        let tweak_key_pair = internal.tap_tweak(&secp, merkle_root).into_inner();

        let address = Address::p2tr(
            &secp,
            tap_info.internal_key(),
            tap_info.merkle_root(),
            bitcoin::Network::Testnet,
        );

        println!("Taproot wallet address {:?}", address);

        address
    }

    // fetch utxos for the given address
    fn fetch_utxos(address: Vec<u8>) -> Vec<Utxo> {
        // fetch the utxos for the address
        // TODO : Take address given by user config
        let client = Client::new("ssl://electrum.blockstream.info:60002").unwrap();
        let vec_tx_in = client
            .script_list_unspent(&address.script_pubkey())
            .unwrap()
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

        println!("Found UTXOS {:?}", vec_tx_in);

        vec_tx_in
    }

    fn generate_transaction(txins : Vex<Txin>, txouts : Vec<TxOut>) -> Vec<u8> {

        let prev_tx = txins
		.iter()
		.map(|tx_id| client.transaction_get(&tx_id.previous_output.txid).unwrap())
		.collect::<Vec<Transaction>>();

        let mut tx = Transaction {
            version: 2,
            lock_time: bitcoin::PackedLockTime(0),
            input: txins,
            output: txous,
        };

        let binding = vec![prev_tx[0].output[0].clone()];
	    let prevouts = Prevouts::All(&binding);

        let sighash_sig = SighashCache::new(&mut tx.clone())
            .taproot_script_spend_signature_hash(
                0,
                &prevouts,
                ScriptPath::with_defaults(&wallet_script),
                SchnorrSighashType::Default,
            )
            .unwrap();

            println!("Sighash Sig {:?}", sighash_sig);

            let msg = Message::from_slice(&sighash_sig).unwrap();
    
            println!("Msg {:?}", msg);
    
            msg
    }

    fn generate_signed_transaction(tx: Transaction, sigs : Vec<Vec<u8>>, wallet_script : Vec<u8>) -> Vec<u8> {
        
        let mut wit_vec = vec![];
        for sig in sigs {
            let sig = SchnorrSig { sig: sig, hash_ty: SchnorrSighashType::Default };
            // (it's a stack, so this is Last In First Out, and will be consumed by the first CHECKSIGVERIFY)
            wit_vec.push(sig)
        }
        let wit = Witness::from_vec(vec![
            sigs,
            wallet_script.to_bytes(),
            actual_control.serialize(),
        ]);

        return wit.to_vec()

    }
        

}

// #[derive(Clone)]
pub struct BTCClientSignature {
    pub from: Address,
    pub _signer: ecdsa::Public,
}

impl BTCClientSignature {
    pub fn new(from: Address, signer: &[u8]) -> Self {
        BTCClientSignature {
            from,
            _signer: ecdsa::Public::try_from(signer).unwrap(),
        }
    }

    pub fn signer(&self, hash: &H256) -> Result<ecdsa::Signature, TransactionCreationError> {
        log::info!("Signer address is : {:?}", self.from);
        // TODO : We should handle this properly, if the signing is not possible maybe propogate the error upstream
        let signed: Result<ecdsa::Signature, TransactionCreationError> =
            crypto::ecdsa_sign_prehashed(OFFCHAIN_SIGNER_KEY_TYPE, &self._signer, &hash.0)
                .ok_or(TransactionCreationError::SigningFailed);

        if signed.is_ok() {
            let sig_bytes = signed.as_ref().unwrap().encode();
            log::info!(
                "Got a signature of size {}: {}",
                sig_bytes.len(),
                str::from_utf8(ChainUtils::bytes_to_hex(sig_bytes.as_slice()).as_slice()).unwrap()
            );
        }

        signed
    }

    pub fn get_signer_address(&self) -> Vec<u8> {
        log::info!("Signer address is : {:?}", self.from);
        self._signer.as_ref().to_vec()
    }
}

