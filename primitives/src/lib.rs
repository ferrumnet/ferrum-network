use sp_application_crypto::KeyTypeId;

/// Defines application identifier for crypto keys for the offchain signer
///
/// When an offchain worker is signing transactions it's going to request keys from type
/// `KeyTypeId` via the keystore to sign the transaction.
/// The keys can be inserted manually via RPC (see `author_insertKey`).
pub const OFFCHAIN_SIGNER_KEY_TYPE: KeyTypeId = KeyTypeId(*b"ofsg");

/// Based on the above `KeyTypeId` we need to generate a pallet-specific crypto type wrapper.
/// We can utilize the supported crypto kinds (`sr25519`, `ed25519` and `ecdsa`) and augment
/// them with the pallet-specific identifier.
pub mod crypto {
    use sp_runtime::app_crypto::{app_crypto, ecdsa};

    app_crypto!(ecdsa, crate::OFFCHAIN_SIGNER_KEY_TYPE);

    /// Identity for the offchain signer key
    pub type AuthorityId = Public;

    /// Signature associated with the offchain signer ecdsa key
    pub type AuthoritySignature = Signature;
}
