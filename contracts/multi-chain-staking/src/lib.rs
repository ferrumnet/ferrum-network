//! QP Staking EVM contract interoperability using XVM interface.
#![cfg_attr(not(feature = "std"), no_std)]

pub use self::qp_staking::{
    QpStaking,
    QpStakingRef,
};

/// EVM ID (from astar runtime)
const EVM_ID: u8 = 0x0F;

/// The EVM ERC20 delegation contract.
#[ink::contract(env = xvm_environment::XvmDefaultEnvironment)]
mod qp_staking {
    // ======= IERC20.sol:IERC20 =======
    // Quantum portal Function signatures:
    // function runWithValue(uint256 fee, uint64 remoteChain, address remoteContract, address beneficiary, address token, bytes memory method) external;
    // c154c628: runWithValue(uint256,uint64,address,address,address,bytes)
    const QP_SELECTOR: [u8; 4] = hex!["c154c628"];
    const TRANSFER_SELECTOR: [u8; 4] = hex!["a9059cbb"];

    use ethabi::{
        ethereum_types::{
            H160,
            U256,
        },
        Token,
    };
    use hex_literal::hex;
    use ink::prelude::vec::Vec;

    #[ink(storage)]
    pub struct QpStaking {
        qp_contract_address: [u8; 20],
        master_chain_id: u128,
        master_contract_address: [u8; 20],
        base_token: [u8; 20],
    }

    /// The error types.
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        /// Returned if not enough balance to fulfill a request is available.
        InsufficientBalance,
        /// Remote execution failed
        RemoteExecutionFailed,
    }

    impl QpStaking {
        /// Create new ERC20 abstraction from given contract address.
        #[ink(constructor)]
        pub fn new(
            qp_contract_address: [u8; 20],
            master_chain_id: u128,
            master_contract_address: [u8; 20],
            base_token: [u8; 20],
        ) -> Self {
            Self {
                qp_contract_address,
                master_chain_id: master_chain_id.into(),
                master_contract_address,
                base_token,
            }
        }

        #[ink(message)]
        pub fn get_base_token(&self) -> [u8; 20] {
            // actual implementation
            self.base_token
        }

        /// Send `transfer_from` call to ERC20 contract.
        #[ink(message)]
        pub fn stake(&mut self, amount: u128, fee: u128) -> Result<(), Error> {
            let caller = self.env().caller();

            // transfer the amount to the QP contract
            let encoded_input =
                Self::transfer_encode(self.qp_contract_address.into(), amount.into());

            self.env()
                .extension()
                .xvm_call(
                    super::EVM_ID,
                    Vec::from(self.base_token.as_ref()),
                    encoded_input,
                )
                .map_err(|_| Error::InsufficientBalance)?;

            let encoded_input = Self::qp_encode(self, fee.into(), Self::h160(&caller));

            let qp_result = self
                .env()
                .extension()
                .xvm_call(
                    super::EVM_ID,
                    Vec::from(self.qp_contract_address.as_ref()),
                    encoded_input,
                )
                .is_ok();

            qp_result.then_some(()).ok_or(Error::RemoteExecutionFailed)
        }

        fn qp_encode(&mut self, fee: U256, sender_address: H160) -> Vec<u8> {
            let mut encoded = QP_SELECTOR.to_vec();
            // 3183e730 : stakeRemote()
            let encoded_method: [u8; 4] = hex!["3183e730"];
            let input = [
                Token::Uint(fee),
                Token::Uint(self.master_chain_id.into()),
                Token::Address(self.master_contract_address.into()),
                Token::Address(sender_address),
                Token::Address(self.base_token.into()),
                Token::Bytes(encoded_method.to_vec()),
            ];
            encoded.extend(&ethabi::encode(&input));
            encoded
        }

        fn transfer_encode(to: H160, value: U256) -> Vec<u8> {
            let mut encoded = TRANSFER_SELECTOR.to_vec();
            let input = [Token::Address(to), Token::Uint(value)];
            encoded.extend(&ethabi::encode(&input));
            encoded
        }

        fn h160(from: &AccountId) -> H160 {
            let mut dest: H160 = [0; 20].into();
            dest.as_bytes_mut()
                .copy_from_slice(&<AccountId as AsRef<[u8]>>::as_ref(from)[..20]);
            dest
        }
    }
}