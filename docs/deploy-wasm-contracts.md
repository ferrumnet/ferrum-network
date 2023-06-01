# Deploy ink! contracts

Ferrum network integrates the `contracts` pallet from substrate. This means you can deploy any wasm compatible contract to the ferrum testnet.

## Ink! Development
Ink! is a Rust eDSL developed by Parity. It specifically targets smart contract development for Substrate’s pallet-contracts.

Ink! offers Rust procedural macros and a list of crates to facilitate development and allows developers to avoid writing boilerplate code.

It is currently the most widely supported eDSL, and will be highly supported in the future. (by Parity and builders community).

Ink! offers a broad range of features such as:

- idiomatic Rust code
- Ink! Macros & Attributes - #[ink::contract]
- Trait support
- Upgradeable contracts - Delegate Call
- Chain Extensions (interact with Substrate pallets inside a contract)
- Off-chain Testing - #[ink(test)]

You can learn more about using ink! at https://use.ink/

## Writing an ink! contract

Example contract 

```
#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
pub mod flipper {
    #[ink(storage)]
    pub struct Flipper {
        value: bool,
    }

    impl Flipper {
        /// Creates a new flipper smart contract initialized with the given value.
        #[ink(constructor)]
        pub fn new(init_value: bool) -> Self {
            Self { value: init_value }
        }

        /// Creates a new flipper smart contract initialized to `false`.
        #[ink(constructor)]
        pub fn new_default() -> Self {
            Self::new(Default::default())
        }

        /// Flips the current value of the Flipper's boolean.
        #[ink(message)]
        pub fn flip(&mut self) {
            self.value = !self.value;
        }

        /// Returns the current value of the Flipper's boolean.
        #[ink(message)]
        pub fn get(&self) -> bool {
            self.value
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[ink::test]
        fn default_works() {
            let flipper = Flipper::new_default();
            assert!(!flipper.get());
        }

        #[ink::test]
        fn it_works() {
            let mut flipper = Flipper::new(false);
            assert!(!flipper.get());
            flipper.flip();
            assert!(flipper.get());
        }
    }

    #[cfg(all(test, feature = "e2e-tests"))]
    mod e2e_tests {
        use super::*;
        use ink_e2e::build_message;

        type E2EResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

        #[ink_e2e::test]
        async fn it_works(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
            // given
            let constructor = FlipperRef::new(false);
            let contract_acc_id = client
                .instantiate("flipper", &ink_e2e::alice(), constructor, 0, None)
                .await
                .expect("instantiate failed")
                .account_id;

            let get = build_message::<FlipperRef>(contract_acc_id.clone())
                .call(|flipper| flipper.get());
            let get_res = client.call_dry_run(&ink_e2e::bob(), &get, 0, None).await;
            assert!(matches!(get_res.return_value(), false));

            // when
            let flip = build_message::<FlipperRef>(contract_acc_id.clone())
                .call(|flipper| flipper.flip());
            let _flip_res = client
                .call(&ink_e2e::bob(), flip, 0, None)
                .await
                .expect("flip failed");

            // then
            let get = build_message::<FlipperRef>(contract_acc_id.clone())
                .call(|flipper| flipper.get());
            let get_res = client.call_dry_run(&ink_e2e::bob(), &get, 0, None).await;
            assert!(matches!(get_res.return_value(), true));

            Ok(())
        }

        #[ink_e2e::test]
        async fn default_works(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
            // given
            let constructor = FlipperRef::new_default();

            // when
            let contract_acc_id = client
                .instantiate("flipper", &ink_e2e::bob(), constructor, 0, None)
                .await
                .expect("instantiate failed")
                .account_id;

            // then
            let get = build_message::<FlipperRef>(contract_acc_id.clone())
                .call(|flipper| flipper.get());
            let get_res = client.call_dry_run(&ink_e2e::bob(), &get, 0, None).await;
            assert!(matches!(get_res.return_value(), false));

            Ok(())
        }
    }
}
```

More examples can be found here : https://github.com/paritytech/ink-examples

## Deploy ink! contract to testnet

To deploy ink! contracts to testnet, install cargo-contract https://github.com/paritytech/cargo-contract

