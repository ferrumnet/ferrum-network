use crate::{
    chain_queries::{fetch_json_rpc, CallResponse, JsonRpcRequest},
    chain_utils::{ChainRequestError, ChainUtils, JsonSer},
    Config,
};
use ethabi_nostd::{encoder, Address, Token};
use ethereum::{LegacyTransaction, TransactionAction};
use frame_system::offchain::{ForAny, SignMessage, Signer};
use parity_scale_codec::Encode;
use rlp::Encodable;
use serde::Deserialize;
use sp_core::{ecdsa, H160, H256, U256};
use sp_std::{
    ops::{Div, Mul},
    prelude::*,
    str,
};
use crate::eip_712::{EIP712, hash_structured_data};

#[derive(Debug, Clone)]
pub struct ContractClient {
    pub http_api: Vec<u8>,
    pub contract_address: Address,
    pub chain_id: u64,
}

// #[derive(Clone)]
pub struct ContractClientSignature<T: Config> {
    pub from: Address,
    pub _signer: Box<Signer<T, T::AuthorityId>>,
    // pub _signer: Box<Signer<Types::T, Types::C>>,
    // pub _signer: fn(&H256) -> ecdsa::Signature,
}

impl<C: Config> ContractClientSignature<C> {
    pub fn new(
        from: Address,
        signer: Signer<C, C::AuthorityId>,
        // signer: fn(&H256) -> ecdsa::Signature,
    ) -> Self {
        ContractClientSignature {
            from,
            _signer: Box::new(signer),
        }
    }

    pub fn signer(&self, hash: &H256) -> ecdsa::Signature {
        log::info!("Signer is {:?}", &self._signer.can_sign());
        let signed = self._signer.sign_message(&hash.0);
        let signed_m = match signed {
            None => panic!("No signature"),
            Some((a, b)) => {
                let public_key = a.public.encode();
                let public_key = &public_key.as_slice()[1..];
                let addr = ChainUtils::eth_address_from_public_key(public_key);
                log::info!(
                    "Signer address is {:?}",
                    str::from_utf8(ChainUtils::bytes_to_hex(addr.as_slice()).as_slice()).unwrap()
                );
                b
            }
        };
        let sig_bytes = signed_m.encode();
        log::info!(
            "Got a signature of size {}: {}",
            sig_bytes.len(),
            str::from_utf8(ChainUtils::bytes_to_hex(sig_bytes.as_slice()).as_slice()).unwrap()
        );
        ecdsa::Signature::try_from(&sig_bytes.as_slice()[1..]).unwrap()
    }
}

// impl <T: SigningTypes + crate::Config, C: AppCrypto<T::Public, T::Signature>> From<Signer<T, C,
// ForAny>> for ContractClientSignature {
impl<T: Config> From<Signer<T, T::AuthorityId, ForAny>> for ContractClientSignature<T> {
    fn from(signer: Signer<T, T::AuthorityId, ForAny>) -> Self {
        log::info!("Signer is {:?}", &signer.can_sign());
        let signed = signer.sign_message(&H256::zero().0);
        let acc = signed.unwrap().0;
        let public_key = acc.public.encode();
        let public_key = &public_key.as_slice()[1..];
        let addr = ChainUtils::eth_address_from_public_key(public_key);
        let from = Address::from(H160::from_slice(addr.as_slice()));

        ContractClientSignature {
            _signer: Box::new(signer),
            from,
        }
    }
}

impl ContractClient {
    pub fn new(http_api: Vec<u8>, contract_address: &Address, chain_id: u64) -> Self {
        ContractClient {
            http_api,
            contract_address: contract_address.clone(),
            chain_id,
        }
    }

    pub fn call<T>(
        &self,
        method_signature: &[u8],
        inputs: &[Token],
    ) -> Result<Box<T>, ChainRequestError>
    where
        T: for<'de> Deserialize<'de>,
    {
        let encoded_bytes = encoder::encode_function_u8(method_signature, inputs);
        let encoded_bytes_0x = ChainUtils::bytes_to_hex(encoded_bytes.as_slice());
        let encoded_bytes_slice = encoded_bytes_0x.as_slice();
        let encoded_bytes_slice = ChainUtils::hex_add_0x(encoded_bytes_slice);
        let encoded = str::from_utf8(encoded_bytes_slice.as_slice()).unwrap();
        log::info!("encoded {}", encoded);
        log::info!(
            "contract address is {}",
            str::from_utf8(ChainUtils::address_to_hex(self.contract_address).as_slice()).unwrap()
        );
        let call_json = JsonSer::new()
            .start()
            .string("data", encoded)
            .string(
                "to",
                str::from_utf8(ChainUtils::address_to_hex(self.contract_address).as_slice())
                    .unwrap(),
            )
            .end()
            .to_vec();
        log::info!("call_json is {}", str::from_utf8(&call_json).unwrap());
        let req = JsonRpcRequest {
            id: 1,
            params: Vec::from([call_json, Vec::from("\"latest\"".as_bytes())]),
            method: b"eth_call".to_vec(),
        };
        log::info!(
            "Have request {:?}",
            str::from_utf8(method_signature).unwrap()
        );
        let http_api = str::from_utf8(&self.http_api[..]).unwrap();
        fetch_json_rpc(http_api, &req)
    }

    pub fn send<T: Config>(
        &self,
        method_signature: &[u8],
        inputs: &[Token],
        gas_limit: Option<U256>,
        gas_price: Option<U256>,
        value: U256,
        nonce: Option<U256>,
        from: Address,
        signing: &ContractClientSignature<T>,
    ) -> Result<H256, ChainRequestError> {
        let encoded_bytes = encoder::encode_function_u8(method_signature, inputs);
        let encoded_bytes_0x = ChainUtils::bytes_to_hex(&encoded_bytes.as_slice());
        let encoded_bytes_slice = encoded_bytes_0x.as_slice();
        let encoded_bytes_slice = ChainUtils::hex_add_0x(encoded_bytes_slice);
        let encoded = str::from_utf8(encoded_bytes_slice.as_slice()).unwrap();
        log::info!("encoded {}", encoded);
        let nonce_val = match nonce {
            None => self.nonce(from)?,
            Some(v) => v,
        };
        let gas_limit_val = match gas_limit {
            None => self.estimate_gas(encoded_bytes_slice.as_slice(), &value, from)?,
            Some(v) => v,
        };
        let gas_price_val = match gas_price {
            None => self
                .gas_price()?
                .mul(U256::from(125 as u32))
                .div(U256::from(100 as u32)),
            Some(v) => v,
        };
        let mut tx = LegacyTransaction {
            nonce: nonce_val,
            gas_price: gas_price_val,
            gas_limit: gas_limit_val,
            action: TransactionAction::Call(self.contract_address),
            value,
            input: encoded_bytes,
            signature: ChainUtils::empty_signature(),
        };
        let hash = ChainUtils::tx_hash_to_sign(&tx, self.chain_id);
        let sig_bytes: ecdsa::Signature = signing.signer(&hash);
        let sig = ChainUtils::decode_transaction_signature(&sig_bytes.0, self.chain_id)?;
        tx.signature = sig;

        let raw_tx = tx.rlp_bytes();
        let hex_tx = ChainUtils::bytes_to_hex(&raw_tx);
        let hex_tx_fmtd =
            ChainUtils::wrap_in_quotes(ChainUtils::hex_add_0x(hex_tx.as_slice()).as_slice());
        let req = JsonRpcRequest {
            id: 1,
            params: Vec::from([hex_tx_fmtd]),
            method: b"eth_sendRawTransaction".to_vec(),
        };
        // log::info!("Have request {:?}", &req);
        let http_api = str::from_utf8(&self.http_api[..]).unwrap();
        let rv: Box<CallResponse> = fetch_json_rpc(http_api, &req)?;
        log::info!("Have response {:?}", &rv);
        Ok(H256::from_slice(
            ChainUtils::hex_to_bytes(rv.result.as_slice())?.as_slice(),
        ))
    }

    pub fn nonce(&self, from: Address) -> Result<U256, ChainRequestError> {
        let req = JsonRpcRequest {
            id: 1,
            params: Vec::from([
                ChainUtils::wrap_in_quotes(ChainUtils::address_to_hex(from).as_slice()),
                b"\"latest\"".to_vec(),
            ]),
            method: b"eth_getTransactionCount".to_vec(),
        };
        let http_api = str::from_utf8(&self.http_api[..]).unwrap();
        let rv: Box<CallResponse> = fetch_json_rpc(http_api, &req)?;
        let nonce = ChainUtils::hex_to_u64(rv.result.as_slice())?;
        Ok(U256::from(nonce))
    }

    pub fn gas_price(&self) -> Result<U256, ChainRequestError> {
        let req = JsonRpcRequest {
            id: 1,
            params: Vec::new(),
            method: b"eth_gasPrice".to_vec(),
        };
        let http_api = str::from_utf8(&self.http_api[..]).unwrap();
        let rv: Box<CallResponse> = fetch_json_rpc(http_api, &req)?;
        let gp = ChainUtils::hex_to_u256(rv.result.as_slice())?;
        Ok(U256::from(gp))
    }

    pub fn produce_eip_712_signature(&self,
    chainId: &[u8],
    networkId: &[u8],
    contractAddress: Address,
    eip_params : EIP712
    ) -> Result<U256, ChainRequestError> {
        // prepare message to sign
        let encoded_bytes = encoder::encode_function_u8(method_signature, inputs);
        let encoded_bytes_0x = ChainUtils::bytes_to_hex(&encoded_bytes.as_slice());
        let encoded_bytes_slice = encoded_bytes_0x.as_slice();
        let encoded_bytes_slice = ChainUtils::hex_add_0x(encoded_bytes_slice);
        let encoded = str::from_utf8(encoded_bytes_slice.as_slice()).unwrap();
        log::info!("encoded {}", encoded);

        let signature = b"finalize(uint256,uint256,bytes32,address[],bytes32,uint64,bytes)";
        let hash_data = b"finalize(uint256,uint256,bytes32,address[],bytes32,uint64,bytes)";
        let hash = ChainUtils::keccack(hash_data);
        log::info!("Finalize transaction hash {:?})", hash);

        // ensure we have keys in keystore to sign
        let signer = Signer::<T, T::AuthorityId>::any_account();
        if !signer.can_sign() {
            return Err(ChainRequestError::JsonRpcError(Vec::from(
                "No keys found in keystore! Insert keys via `author_insertKey` RPC.",
            )));
        }

        let ecdsa_signer = ContractClientSignature::from(signer);

        // sign the message
        let signature: ecdsa::Signature = ecdsa_signer.signer(&hash);
        let sig_bytes: &[u8] = &signature.0;
        log::info!("Finalize transaction signature {:?})", sig_bytes);
    }

    pub fn get_eip_712_types(&self, _encoded: &[u8]) -> Result<eip_712::EIP712> {
        let json = r#"{
            "types": {
                [
                    { type: 'uint256', name: 'action', value: '1' },
				    { type: 'bytes32', name: 'msgHash', value: msgHash },
                    { type: 'bytes32', name:'salt', value: salt},
				    { type: 'uint64', name: 'expiry', value: expiry },
                ]
            }
        }"#;
        let typed_data = from_str::<EIP712>(json).unwrap();    
    }

    pub fn estimate_gas(
        &self,
        encoded: &[u8],
        value: &U256,
        from: Address,
    ) -> Result<U256, ChainRequestError> {
        let call_json = JsonSer::new()
            .start()
            .string("input", str::from_utf8(encoded).unwrap())
            .string(
                "from",
                str::from_utf8(ChainUtils::address_to_hex(from).as_slice()).unwrap(),
            )
            .string(
                "to",
                str::from_utf8(ChainUtils::address_to_hex(self.contract_address).as_slice())
                    .unwrap(),
            )
            .string(
                "value",
                str::from_utf8(ChainUtils::u256_to_hex_0x(value).as_slice()).unwrap(),
            )
            .end()
            .to_vec();
        log::info!(
            "estimateGas json is {}",
            str::from_utf8(&call_json).unwrap()
        );
        let req = JsonRpcRequest {
            id: 1,
            params: Vec::from([call_json, Vec::from("\"latest\"".as_bytes())]),
            method: b"eth_estimateGas".to_vec(),
        };
        let http_api = str::from_utf8(&self.http_api[..]).unwrap();
        let rv: Box<CallResponse> = fetch_json_rpc(http_api, &req)?;
        let gp = ChainUtils::hex_to_u256(rv.result.as_slice())?;
        Ok(U256::from(gp))
    }
}
