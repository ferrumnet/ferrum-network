#![cfg_attr(not(feature = "std"), no_std)]

use crate::chain_queries::{ChainQueries, fetch_json_rpc, JsonRpcRequest, CallResponse, de_string_to_bytes};
use sp_core::{ecdsa, H160, H256, U256};
use sp_std::{str};
use ethereum::{Account, LegacyTransaction, TransactionAction, TransactionSignature, TransactionV2};
use frame_system::offchain::SigningTypes;
use hex_literal::hex;
use serde_json::json;
use ethabi_nostd::{Address, encoder, Token};
use crate::chain_utils::{ChainRequestError, ChainUtils, JsonSer, ToJson};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sp_core::crypto::{AccountId32};
use sp_std::prelude::*;
use crate::contract_client::ContractClient; //vec::{Vec};

#[derive(Debug, Deserialize)]
struct TotalSupplyResponse {
    #[serde(deserialize_with = "de_string_to_bytes")]
    pub result: Vec<u8>,
}

pub struct Erc20Client {
    contract: ContractClient,
}

impl Erc20Client {
    pub fn new(client: ContractClient) -> Self {
        Erc20Client {
            contract: client,
        }
    }

    pub fn total_supply(&self) -> Result<U256, ChainRequestError> {
        let signature = b"totalSupply()";
        let mut res: Box<TotalSupplyResponse> = self.contract.call(signature, &[])?;
        res.result.remove(0);
        res.result.remove(0);
        let res_str = str::from_utf8(res.result.as_slice()).unwrap();
        log::info!("result {}", res_str);
        let mut bytes: [u8;32] = [0 as u8;32];
        log::info!("result as u256 {:?}", &res.result);
        hex::decode_to_slice(res_str, &mut bytes);
        log::info!("result as u256 {:?}", &bytes);
        Ok(U256::from(bytes))
    }

    pub fn approve(&self,
        approvee: Address,
        amount: U256,
        from: Address,
        signer: fn(&H256) -> ecdsa::Signature,
    ) -> Result<H256, ChainRequestError> {
        let signature = b"approve(address,uint256)";
        let res = self.contract.send(
        signature,
            &[
                Token::Address(approvee), // TODO convert address
                Token::Uint(amount),
            ],
            Some(U256::from(100000)),
            U256::from(2000000000 as u64), // TODO: Get the gas price
            U256::zero(),
            None,
            from,
            signer,
        )?;
        Ok(res)
    }
}

#[cfg(test)]
mod tests {
    use crate::erc_20_client::{Erc20Client};
    use sp_std::prelude::*; //vec::{Vec};
    use sp_core::crypto::{AccountId32, ByteArray};
    use ethereum::{TransactionRecoveryId, TransactionSignature, TransactionV2,
                   LegacyTransaction, TransactionAction};
    use sp_core::H256;
    use crate::chain_queries::{CallResponse, fetch_json_rpc, JsonRpcRequest};
    use sp_io::TestExternalities;
    use sp_core::offchain::{testing, OffchainWorkerExt};
    use crate::contract_client::ContractClient;

    #[test]
    fn test_total_supply() {
        let (offchain, state) = testing::TestOffchainExt::new();
        let mut t = TestExternalities::default();
        t.register_extension(OffchainWorkerExt::new(offchain));

        t.execute_with(|| {
            let rpc_endpoint = "https://rinkeby.infura.io/v3/18b15ac5b3e8447191c6b233dcd2ce14";
            // println!("==<>====<>===<>===<>===<>===<>===<>===<>===<>===<>==========");
            // println!("USING {}", &rpc_endpoint);
            let mut addr_f = [0u8; 32];
            hex::decode_to_slice("f6832ea221ebfdc2363729721a146e6745354b14000000000000000000000000", &mut addr_f as &mut [u8]);
            let contract_f = AccountId32::from_slice(&addr_f).unwrap();
            let client = ContractClient::new(
                rpc_endpoint.clone(),
                "",
                4 as u64);
            let erc_20 = Erc20Client::new(client);
            log::info!("Erc20 address got");
            let ts = erc_20.total_supply();
            log::info!("Total supply got {:?}", &ts);
        })
    }
}