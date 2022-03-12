use ethereum::{LegacyTransaction, LegacyTransactionMessage, TransactionSignature};
use parity_scale_codec::Encode;
use crate::chain_utils::ChainRequestError::ConversionError;

pub struct ChainUtils;
use sp_std::{str};
use sp_std::prelude::*;
use ethabi_nostd::{U256, H256, Address}; //vec::{Vec};
use numtoa::NumToA;
use sp_core::{ecdsa};
use sp_io::crypto;
use libsecp256k1;
use frame_system::offchain::CreateSignedTransaction;
use libsecp256k1::Signature;
use log::log;
use tiny_keccak::{Hasher, Keccak};
use crate::KEY_TYPE;

#[derive(Debug)]
pub enum ChainRequestError {
    ErrorGettingJsonRpcResponse,
    BadRemoteData,
    ConversionError,
	ErrorCreatingTransaction,
	RemoteBlockAlreadyMined,
}

pub trait ToJson {
    type BaseType;
    fn to_json(&self) -> Vec<u8>;
}

pub type ChainRequestResult<T> = Result<T, ChainRequestError>;

fn u64_to_str(num: u64) -> Vec<u8> {
    let mut num_buffer = [0u8; 20];
    num.numtoa_str(10, &mut num_buffer);
    log::info!("num2str1 : {:?}", num_buffer);
    let mut s: Vec<u8> = Vec::new();
    num_buffer.into_iter().filter(|u| *u != 0)
        .for_each(|u| { s.push(u); () });
    log::info!("num2str2 : {:?}", s);
    s
}

const HEX_TABLE: [u8;16] = [
    '0' as u8, '1' as u8, '2' as u8, '3' as u8, '4' as u8, '5' as u8,
    '6' as u8, '7' as u8, '8' as u8, '9' as u8, 'a' as u8, 'b' as u8,
    'c' as u8, 'd' as u8, 'e' as u8, 'f' as u8];

pub const EMPTY_HASH: H256 = H256([
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x01,
]);

impl ChainUtils {
    pub fn hex_to_u64(s: &[u8]) -> Result<u64, ChainRequestError> {
        if s.len() < 2 {
            return Err(ChainRequestError::ConversionError);
        }
        let hexb = if s[0] == '0' as u8 && s[1] == 'x' as u8 {
            &s[2..] } else { s };
        let hex = str::from_utf8(&hexb)
            .map_err(|e| {
                log::error!("Error when converting from hex: {:?}", e);
                ChainRequestError::ConversionError
            })?;
        let rv = u64::from_str_radix(hex, 16)
            .map_err(|e| {
                log::error!("{:?}", e);
                ChainRequestError::ConversionError
            })?;
        Ok(rv)
    }

    pub fn bytes_to_hex(s: &[u8]) -> Vec<u8> {
        let mut rv = Vec::new();
        s.into_iter().for_each(|u| {
            rv.push(HEX_TABLE[((u & 0xf0) >> 4) as usize]);
            rv.push(HEX_TABLE[(u & 0x0f) as usize]);
        });
        rv
    }

    pub fn address_to_hex(address: Address) -> Vec<u8> {
        Self::hex_add_0x(Self::bytes_to_hex(&address.0).as_slice())
    }

    pub fn hex_to_address(hex: &[u8]) -> Address {
        let mut addr_bytes: [u8; 20] = [0; 20];
        hex::decode_to_slice(hex, &mut addr_bytes);
        Address::from_slice(&addr_bytes)
    }

    pub fn hex_add_0x(s: &[u8]) -> Vec<u8> {
        if s.len() >= 2 && s[0] == '0' as u8 && s[1] == 'x' as u8 {
            return Vec::from(s.clone());
        }
        let mut zx = vec!['0' as u8, 'x' as u8];
        zx.extend(s);
        zx
    }

    pub fn wrap_in_quotes(s: &[u8]) -> Vec<u8> {
        let mut zx = vec!['"' as u8];
        zx.extend(s);
        zx.extend(vec!['"' as u8]);
        zx
    }

    pub fn u256_to_hex_0x(i: &U256) -> Vec<u8> {
        let fmted = i.encode();
        let mut zx = vec!['0' as u8, 'x' as u8];
        zx.extend(fmted.into_iter().map(|i| i + '0' as u8));
        // println!("FMTED {:?}", &zx);
        zx
    }

    pub fn u256_to_hex_str(i: &U256) -> Vec<u8> {
        let fmted = i.encode();
        let mut zx = vec![];
        zx.extend(fmted.into_iter().map(|i| i + '0' as u8));
        zx
    }

    pub fn h256_to_hex_0x(i: &H256) -> Vec<u8> {
        let fmted = i.encode();
        let mut zx = vec!['0' as u8, 'x' as u8];
        zx.extend(fmted.into_iter().map(|i| i + '0' as u8));
        zx
    }

    pub fn empty_signature() -> TransactionSignature {
        const LOWER: H256 = H256([
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x01,
        ]);
        TransactionSignature::new(28, LOWER, LOWER).unwrap()
    }

    pub fn sign_transaction_hash (
        key_pair: &ecdsa::Public,
        hash: &H256) -> ChainRequestResult<Vec<u8>> {
        let sig: ecdsa::Signature = crypto::ecdsa_sign_prehashed(
            KEY_TYPE,
            key_pair,
            &hash.0,
        ).unwrap();
        let sig_bytes: &[u8] = &sig.0;
        Ok(Vec::from(sig_bytes))
    }

    pub fn tx_hash_to_sign(
        tx: &LegacyTransaction,
        chain_id: u64,
    ) -> H256 {
        let mut msg: LegacyTransactionMessage = ethereum::TransactionV0::from(tx.clone()).into();
        msg.chain_id = Some(chain_id);
        msg.hash()
        // let chain_id_b = chain_id.encode();
        // s.append(&tx.gas_price);
        // s.append(&tx.gas_limit);
        // s.append(&tx.action);
        // s.append(&tx.value);
        // s.append(&tx.input);
        // s.append(s.encoder().encode_value());
        //
        // // s.append(&self.signature.v.0);
        // // s.append(&U256::from_big_endian(&self.signature.r[..]));
        // // s.append(&U256::from_big_endian(&self.signature.s[..]));
        // let res = s.out();
        // Self::keccack(&res)
    }

    pub fn decode_transaction_signature(
        signature: &[u8;65],
        chain_id: u64,
    ) -> ChainRequestResult<TransactionSignature> {
        let sig = libsecp256k1::Signature::parse_standard_slice(&signature[..64])
            .map_err(|e| {
                log::error!("Error sign_transaction_hash {:?}", e);
                ChainRequestError::ErrorCreatingTransaction
            })?;
        let recovery_id = libsecp256k1::RecoveryId::parse(signature[64])
            .map_err(|e| {
                log::error!("Error sign_transaction_hash {:?}", e);
                ChainRequestError::ErrorCreatingTransaction
            })?;
        let rid = chain_id * 2 + 35 + recovery_id.serialize() as u64;
        Ok(TransactionSignature::new(
            rid,
            H256::from_slice(&signature[0..32]),
            H256::from_slice(&signature[32..64]),
        ).ok_or_else(|| ChainRequestError::ErrorCreatingTransaction)?)
    }

    pub fn eth_address_from_public_key(pk: &[u8]) -> Vec<u8> {
        let mut uncomp: [u8; 65] = [0; 65];
        let pk = match pk.len() {
            64 => {
                pk
            },
            33 => {
                let pk = libsecp256k1::PublicKey::parse_slice(pk, None).unwrap();
                uncomp = pk.serialize();
                &uncomp[1..]
            },
            _ => {
                panic!("Bad size for public key. Must be 64 or 33")
            }
        };
        let mut signed: [u8; 32] = [0; 32];
        let mut sponge = Keccak::v256();
        sponge.update(pk);
        sponge.finalize(&mut signed);
        Vec::from(&signed[12..32])
    }

    pub fn keccack(msg: &[u8]) -> H256 {
        let mut buf: [u8; 32] = [0; 32];
        let mut sponge = Keccak::v256();
        sponge.update(msg);
        sponge.finalize(&mut buf);
        H256::from(buf)
    }

    // /// Generate a crypto pair from seed.
    // pub fn get_from_seed(seed: &str) -> ecdsa::Public {
    //     ecdsa::Pair::from_string(&format!("//{}", seed), None)
    //         .expect("static values are valid; qed")
    //         .public()
    // }
}

pub struct JsonSer {
    buff: Vec<u8>,
    empty: bool,
}

impl JsonSer {
    pub fn new() -> Self {
        JsonSer { buff: Vec::new(), empty: true }
    }

    pub fn start(&mut self) -> &mut Self {
        if self.buff.len() > 0 { panic!("JSON already started") }
        self.buff.push('{' as u8);
        self
    }

    pub fn end(&mut self) -> &mut Self {
        self.buff.push('}' as u8);
        self
    }

    pub fn comma(&mut self) -> &mut Self {
        if !self.empty {
            self.buff.push(',' as u8);
        }
        self
    }

    pub fn name(&mut self, name: &str) -> &mut Self {
        self.comma();
        self.buff.push('"' as u8);
        name.as_bytes().into_iter().for_each(|b| self. buff.push(b.clone()));
        self.buff.push('"' as u8);
        self.buff.push(':' as u8);
        self
    }

    pub fn arr_string(&mut self, val: &str) -> &mut Self {
        self.comma();
        self.buff.push('"' as u8);
        val.as_bytes().into_iter().for_each(|b| self. buff.push(b.clone()));
        self.buff.push('"' as u8);
        self.empty = false;
        self
    }

    pub fn arr_val(&mut self, val: &str) -> &mut Self {
        self.comma();
        val.as_bytes().into_iter().for_each(|b| self. buff.push(b.clone()));
        self.empty = false;
        self
    }

    pub fn string(&mut self, name: &str, val: &str) -> &mut Self {
        self.name(name);
        self.buff.push('"' as u8);
        val.as_bytes().into_iter().for_each(|b| self. buff.push(b.clone()));
        self.buff.push('"' as u8);
        self.empty = false;
        self
    }

    pub fn u256(&mut self, name: &str, value: &U256) -> &mut Self {
        self.string(
            name,
            str::from_utf8(ChainUtils::u256_to_hex_0x(&value).as_slice()).unwrap())
    }

    pub fn num(&mut self, name: &str, val: u64) -> &mut Self {
        let v = u64_to_str(val);
        self.val(name, str::from_utf8(&v.as_slice()).unwrap())
    }

    pub fn val(&mut self, name: &str, val: &str) -> &mut Self {
        self.name(name);
        val.as_bytes().into_iter().for_each(|b| self. buff.push(b.clone()));
        self.empty = false;
        self
    }

    pub fn arr(&mut self, name: &str, val: &str) -> &mut Self {
        self.name(name);
        self.arr_start();
        val.as_bytes().into_iter().for_each(|b| self. buff.push(b.clone()));
        self.arr_end();
        self.empty = false;
        self
    }

    pub fn arr_start(&mut self) -> &mut Self {
        self.buff.push('[' as u8);
        self.empty = true;
        self
    }

    pub fn arr_end(&mut self) -> &mut Self {
        self.buff.push(']' as u8);
        self.empty = false;
        self
    }

    pub fn to_vec(&self) -> Vec<u8> {
        self.buff.clone()
    }
}

#[cfg(test)]
mod tests {
    use hex_literal::hex;
    use crate::chain_utils::{ChainUtils, JsonSer};
    use sp_std::{collections::vec_deque::VecDeque, prelude::*, str};

    #[test]
    fn jsonify_num() {
        let jo = JsonSer::new()
            .start()
            .num("id", 1)
            .end()
            .to_vec();
        log::info!("Jos is {:?}", jo);
        println!("Jo is {:?}", jo);
        let jos = str::from_utf8(jo.as_slice());
        println!("Jos is {}", jos.unwrap());
    }

    #[test]
    fn eth_addr_from_public_key() {
        let d = hex::decode(
            "836b35a026743e823a90a0ee3b91bf615c6a757e2b60b9e1dc1826fd0dd16106f7bc1e8179f665015f43c6c81f39062fc2086ed849625c06e04697698b21855e").unwrap();
        let mut pk: [u8; 64] = [0; 64];
        pk.copy_from_slice(d.as_slice());
        let addr = ChainUtils::eth_address_from_public_key(&pk);
        let addrh = hex::encode(addr.as_slice());
        assert_eq!("0bed7abd61247635c1973eb38474a2516ed1d884", addrh);
    }

    #[test]
    fn eth_addr_from_public_key2() {
        let d = hex::decode(
            "84885a1311fe34c65565247d25a09cee8c25168c7febd3e3ff8253bfd3496f74").unwrap();
        let p0: &[u8] = &[02];
        let addr = ChainUtils::eth_address_from_public_key(
            [p0, d.as_slice()].concat().as_slice());
        let addrh = hex::encode(addr.as_slice());
        assert_eq!("1458e7bde6e509e4f8c122642bd61629aa46fa7c", addrh);
    }
}