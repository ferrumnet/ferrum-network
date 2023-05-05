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

## Deploy ink! contract to testnet

TBD