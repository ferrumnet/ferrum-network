#![cfg_attr(not(feature = "std"), no_std)]

#[ink::contract]
mod MultiChainStaking {
    use ethabi_nostd::Token;
    use ethereum_types::{H160, U256};
    use hex_literal::hex;
    use ink::prelude::vec::Vec;

    #[derive(Debug, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum To {
        EVM([u8; 20]),
        WASM(AccountId),
    }

    const REMOTE_STAKE_FUNCTION: [u8; 4] = hex!["a9059cbb"];

    impl From<To> for H160 {
        fn from(f: To) -> Self {
            return match f {
                To::EVM(a) => a.into(),
                To::WASM(a) => {
                    let mut dest: H160 = [0; 20].into();
                    dest.as_bytes_mut()
                        .copy_from_slice(&<AccountId as AsRef<[u8]>>::as_ref(&a)[..20]);
                    dest
                }
            };
        }
    }

    #[ink(storage)]
    pub struct MultiChainStakingClient {
        erc20_address: Address,
        reward_token: Address,
        totalRewards: u128,
        totalStake: u128,
        distributeRewards: bool,
        stakeClosed: bool,
        reserveAccount: AccountId,
    }

    /// Type alias for the contract's `Result` type.
    pub type Result<T> = core::result::Result<T, Error>;

    pub type Address = AccountId;

    /// The contract error types.
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        /// Returned if not enough balance to fulfill a request is available.
        InsufficientBalance,
    }

    /// Emitted when the sender starts closing the channel.
    #[ink(event)]
    pub struct SenderCloseStarted {
        expiration: Timestamp,
        close_duration: Timestamp,
    }

    impl MultiChainStakingClient {
        #[ink(constructor)]
        pub fn new(recipient: AccountId, close_duration: Timestamp) -> Self {
            Self {
                sender: Self::env().caller(),
                recipient,
                expiration: None,
                withdrawn: 0,
                close_duration,
            }
        }

        #[ink(message, payable)]
        pub fn stake(
            &mut self,
            from: AccountId,
            address: [u8; 20],
            value: Balance,
            fee: Balance,
        ) -> Result<()> {
            // transfer the tokens to the reserve address of the contract
            let from_balance = self.balances.get(from).unwrap_or_default();
            if from_balance < value {
                return Err(Error::InsufficientBalance);
            }
            self.balances.insert(from, &(from_balance - value));
            let to_balance = self.balance_of_impl(self.reserveAccount);
            self.balances
                .insert(self.reserveAccount, &(to_balance + value));

            // trigger xcm message to the remote stake contract
            let encoded_input = Self::transfer_encode(to.into(), value.into());
            self.env()
                .extension()
                .xvm_call(
                    super::EVM_ID,
                    Vec::from(self.erc20_address.as_ref()),
                    encoded_input,
                )
                .is_ok()
        }

        fn remote_stake_encode(to: H160, value: U256) -> Vec<u8> {
            let mut encoded = REMOTE_STAKE_FUNCTION.to_vec();
            let input = [Token::Address(to), Token::Uint(value)];
            encoded.extend(&ethabi_nostd::encode(&input));
            encoded
        }
    }
}
