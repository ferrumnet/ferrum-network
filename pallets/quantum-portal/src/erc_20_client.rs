#![cfg_attr(not(feature = "std"), no_std)]

use crate::{
    chain_queries::de_string_to_bytes, chain_utils::ChainRequestError,
    contract_client::ContractClient,
};
use serde::Deserialize;
use sp_core::U256;
use sp_std::{prelude::*, str};

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct TotalSupplyResponse {
    #[serde(deserialize_with = "de_string_to_bytes")]
    pub result: Vec<u8>,
}

pub struct Erc20Client {
    pub contract: ContractClient,
}

#[allow(dead_code)]
impl Erc20Client {
    pub fn new(client: ContractClient) -> Self {
        Erc20Client { contract: client }
    }

    pub fn total_supply(&mut self) -> Result<U256, ChainRequestError> {
        let signature = b"totalSupply()";
        let mut res: Box<TotalSupplyResponse> = self.contract.call(signature, &[], None)?;
        res.result.remove(0);
        res.result.remove(0);
        let res_str = str::from_utf8(res.result.as_slice()).unwrap();
        log::info!("result {}", res_str);
        let mut bytes: [u8; 32] = [0_u8; 32];
        log::info!("result as u256 {:?}", &res.result);
        hex::decode_to_slice(res_str, &mut bytes).unwrap();
        log::info!("result as u256 {:?}", &bytes);
        Ok(U256::from(bytes))
    }

    // #[allow(dead_code)]
    // pub fn approve(&self,
    //     approvee: Address,
    //     amount: U256,
    //     from: Address,
    //     signer: fn(&H256) -> ecdsa::Signature,
    // ) -> Result<H256, ChainRequestError> {
    //     let signature = b"approve(address,uint256)";
    //     let res = self.contract.send(
    //     signature,
    //         &[
    //             Token::Address(approvee), // TODO convert address
    //             Token::Uint(amount),
    //         ],
    //         None, // Some(U256::from(100000)),
    //         None, // Some(U256::from(2000000000 as u64)), // TODO: Get the gas price
    //         U256::zero(),
    //         None,
    //         from,
    //         signer,
    //     )?;
    //     Ok(res)
    // }
}

// #[cfg(test)]
// mod tests {
//     use crate::{contract_client::ContractClient, erc_20_client::Erc20Client};
//     use parity_scale_codec::Encode;
//     use sp_core::{
//         offchain::{testing, OffchainWorkerExt},
//         H160,
//     };
//     use sp_io::TestExternalities;
//     use sp_std::prelude::*; //vec::{Vec};

//     #[test]
//     fn test_total_supply() {
//         let (offchain, _) = testing::TestOffchainExt::new();
//         let mut t = TestExternalities::default();
//         t.register_extension(OffchainWorkerExt::new(offchain));

//         t.execute_with(|| {
//             let rpc_endpoint = "https://rinkeby.infura.io/v3/18b15ac5b3e8447191c6b233dcd2ce14";
//             // println!("==<>====<>===<>===<>===<>===<>===<>===<>===<>===<>==========");
//             // println!("USING {}", &rpc_endpoint);
//             let mut addr_f = [0u8; 32];
//             hex::decode_to_slice(
//                 "f6832ea221ebfdc2363729721a146e6745354b14000000000000000000000000",
//                 &mut addr_f as &mut [u8],
//             )
//             .unwrap();
//             let client =
//                 ContractClient::new(rpc_endpoint.clone().encode(), &H160::zero(), 4 as u64);
//             let erc_20 = Erc20Client::new(client);
//             log::info!("Erc20 address got");
//             let ts = erc_20.total_supply();
//             log::info!("Total supply got {:?}", &ts);
//         })
//     }
// }
