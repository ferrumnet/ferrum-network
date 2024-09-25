use frame_support::parameter_types;
use pallet_evm_precompile_balances_erc20::{Erc20BalancesPrecompile, Erc20Metadata};
use pallet_evm_precompile_blake2::Blake2F;
use pallet_evm_precompile_bn128::{Bn128Add, Bn128Mul, Bn128Pairing};
use pallet_evm_precompile_modexp::Modexp;
use pallet_evm_precompile_quantum_portal::QuantumPortalPrecompile;
use pallet_evm_precompile_sha3fips::Sha3FIPS256;
use pallet_evm_precompile_simple::{ECRecover, ECRecoverPublicKey, Identity, Ripemd160, Sha256};
use precompile_utils::precompile_set::*;

// use crate::QuantumPortal;

pub struct NativeErc20Metadata;

/// ERC20 metadata for the native token.
impl Erc20Metadata for NativeErc20Metadata {
	/// Returns the name of the token.
	fn name() -> &'static str {
		"FRM Token"
	}

	/// Returns the symbol of the token.
	fn symbol() -> &'static str {
		"FRM"
	}

	/// Returns the decimals places of the token.
	fn decimals() -> u8 {
		18
	}

	/// Must return `true` only if it represents the main native currency of
	/// the network. It must be the currency used in `pallet_evm`.
	fn is_native_currency() -> bool {
		true
	}
}

/// The asset precompile address prefix. Addresses that match against this prefix will be routed
/// to Erc20AssetsPrecompileSet being marked as foreign
pub const FOREIGN_ASSET_PRECOMPILE_ADDRESS_PREFIX: &[u8] = &[255u8; 4];
/// The asset precompile address prefix. Addresses that match against this prefix will be routed
/// to Erc20AssetsPrecompileSet being marked as local
pub const LOCAL_ASSET_PRECOMPILE_ADDRESS_PREFIX: &[u8] = &[255u8, 255u8, 255u8, 254u8];

parameter_types! {
	pub ForeignAssetPrefix: &'static [u8] = FOREIGN_ASSET_PRECOMPILE_ADDRESS_PREFIX;
	pub LocalAssetPrefix: &'static [u8] = LOCAL_ASSET_PRECOMPILE_ADDRESS_PREFIX;
}

type EthereumPrecompilesChecks = (AcceptDelegateCall, CallableByContract, CallableByPrecompile);

/// The PrecompileSet installed in the Ferrum runtime.
/// We include the nine Istanbul precompiles
/// (https://github.com/ethereum/go-ethereum/blob/3c46f557/core/vm/contracts.go#L69)
/// as well as a special precompile for dispatching Substrate extrinsics
/// The following distribution has been decided for the precompiles
/// 0-1023: Ethereum Mainnet Precompiles
/// 1024-2047 Precompiles that are not in Ethereum Mainnet but are neither Ferrum specific
/// 2048-4095 Ferrum specific precompiles
pub type FrontierPrecompiles<R> = PrecompileSetBuilder<
	R,
	(
		// Skip precompiles if out of range.
		PrecompilesInRangeInclusive<
			(AddressU64<1>, AddressU64<4095>),
			(
				// Ethereum precompiles:
				// We allow DELEGATECALL to stay compliant with Ethereum behavior.
				PrecompileAt<AddressU64<1>, ECRecover, EthereumPrecompilesChecks>,
				PrecompileAt<AddressU64<2>, Sha256, EthereumPrecompilesChecks>,
				PrecompileAt<AddressU64<3>, Ripemd160, EthereumPrecompilesChecks>,
				PrecompileAt<AddressU64<4>, Identity, EthereumPrecompilesChecks>,
				PrecompileAt<AddressU64<5>, Modexp, EthereumPrecompilesChecks>,
				PrecompileAt<AddressU64<6>, Bn128Add, EthereumPrecompilesChecks>,
				PrecompileAt<AddressU64<7>, Bn128Mul, EthereumPrecompilesChecks>,
				PrecompileAt<AddressU64<8>, Bn128Pairing, EthereumPrecompilesChecks>,
				PrecompileAt<AddressU64<9>, Blake2F, EthereumPrecompilesChecks>,
				// Non-Ferrum specific nor Ethereum precompiles :
				PrecompileAt<
					AddressU64<1024>,
					Sha3FIPS256,
					(CallableByContract, CallableByPrecompile),
				>,
				// PrecompileAt<AddressU64<1025>, Dispatch<R>>,
				PrecompileAt<
					AddressU64<1026>,
					ECRecoverPublicKey,
					(CallableByContract, CallableByPrecompile),
				>,
				// Ferrum specific precompiles:
				PrecompileAt<
					AddressU64<2050>,
					Erc20BalancesPrecompile<R, NativeErc20Metadata>,
					(CallableByContract, CallableByPrecompile),
				>,
				// Ferrum specific precompiles:
				PrecompileAt<
					AddressU64<2066>,
					QuantumPortalPrecompile<R>,
					(CallableByContract, CallableByPrecompile),
				>,
			),
		>,
	),
>;
