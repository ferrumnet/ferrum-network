use sp_application_crypto::KeyTypeId;

/// Defines application identifier for crypto keys of this module.
///
/// Every module that deals with signatures needs to declare its unique identifier for
/// its crypto keys.
/// When an offchain worker is signing transactions it's going to request keys from type
/// `KeyTypeId` via the keystore to sign the transaction.
/// The keys can be inserted manually via RPC (see `author_insertKey`).
pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"dem!");

/// Based on the above `KeyTypeId` we need to generate a pallet-specific crypto type wrapper.
/// We can utilize the supported crypto kinds (`sr25519`, `ed25519` and `ecdsa`) and augment
/// them with the pallet-specific identifier.
pub mod crypto {
		use sp_core::H256;
		use crate::KEY_TYPE;
		use sp_core::ecdsa::{Signature as EcdsaSignagure};
		use sp_std::prelude::*;
		use sp_runtime::{
			app_crypto::{app_crypto, ecdsa},
			traits::Verify, MultiSignature, MultiSigner
		};
		use sp_runtime::MultiSigner::Ecdsa;
		use sp_std::str;
		use crate::chain_utils::ChainUtils;

		app_crypto!(ecdsa, KEY_TYPE);

		pub struct TestAuthId;
		// implemented for runtime
		impl frame_system::offchain::AppCrypto<MultiSigner, MultiSignature> for TestAuthId {
			type RuntimeAppPublic = Public;
			type GenericSignature = sp_core::ecdsa::Signature; // sr25519::Signature;
			type GenericPublic = sp_core::ecdsa::Public;

			fn sign(payload: &[u8], public: MultiSigner) -> Option<MultiSignature> {
				let ecdsa_pub = match public {
					Ecdsa(p) => p,
					_ => panic!("Wrong public type"),
				};
				let hash = H256::from_slice(payload); // ChainUtils::keccack(payload);
				let sig = ChainUtils::sign_transaction_hash(
					&ecdsa_pub, &hash).unwrap();

				let mut buf: [u8; 65] = [0; 65];
				buf.copy_from_slice(sig.as_slice());
				let signature = ecdsa::Signature(buf);
				Some(MultiSignature::Ecdsa(signature))
			}
		}

		// implemented for mock runtime in test
		impl frame_system::offchain::AppCrypto<<EcdsaSignagure as Verify>::Signer, EcdsaSignagure>
		for TestAuthId
		{
			type RuntimeAppPublic = Public;
			type GenericSignature = sp_core::ecdsa::Signature;
			type GenericPublic = sp_core::ecdsa::Public;
		}
	}
