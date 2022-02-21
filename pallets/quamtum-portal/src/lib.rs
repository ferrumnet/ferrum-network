#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

mod chain_queries;
mod chain_utils;
mod qp_types;

#[frame_support::pallet]
pub mod pallet {
	//! A demonstration of an offchain worker that sends onchain callbacks
	use core::{convert::TryInto};
	use parity_scale_codec::{Decode, Encode};
	use frame_support::pallet_prelude::*;
	use frame_system::{
		pallet_prelude::*,
		offchain::{
			AppCrypto, CreateSignedTransaction, SendSignedTransaction, SendUnsignedTransaction,
			SignedPayload, Signer, SigningTypes, SubmitTransaction,
		},
	};
	use sp_core::{crypto::KeyTypeId};
	use sp_runtime::{
		offchain::{
			http,
			storage::StorageValueRef,
			storage_lock::{BlockAndTime, StorageLock},
			Duration,
		},
		traits::BlockNumberProvider,
		transaction_validity::{
			InvalidTransaction, TransactionSource, TransactionValidity, ValidTransaction,
		},
		RuntimeDebug,
	};
	use sp_std::{collections::vec_deque::VecDeque, prelude::*, str};

	use serde::{Deserialize, Deserializer};
	use crate::chain_queries::ChainQueries;

	/// Defines application identifier for crypto keys of this module.
	///
	/// Every module that deals with signatures needs to declare its unique identifier for
	/// its crypto keys.
	/// When an offchain worker is signing transactions it's going to request keys from type
	/// `KeyTypeId` via the keystore to sign the transaction.
	/// The keys can be inserted manually via RPC (see `author_insertKey`).
	pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"demo");
	const NUM_VEC_LEN: usize = 10;
	/// The type to sign and send transactions.
	const UNSIGNED_TXS_PRIORITY: u64 = 100;

	const HTTP_REMOTE_REQUEST: &str = "http://0.0.0.0:8000/test.html";

	const FETCH_TIMEOUT_PERIOD: u64 = 3000; // in milli-seconds
	const LOCK_TIMEOUT_EXPIRATION: u64 = FETCH_TIMEOUT_PERIOD + 1000; // in milli-seconds
	const LOCK_BLOCK_EXPIRATION: u32 = 3; // in block number



	/// Based on the above `KeyTypeId` we need to generate a pallet-specific crypto type wrapper.
	/// We can utilize the supported crypto kinds (`sr25519`, `ed25519` and `ecdsa`) and augment
	/// them with the pallet-specific identifier.
	pub mod crypto {
		use crate::KEY_TYPE;
		use sp_core::sr25519::Signature as Sr25519Signature;
		use sp_std::prelude::*;
		use sp_runtime::{
			app_crypto::{app_crypto, sr25519},
			traits::Verify, MultiSignature, MultiSigner
		};

		app_crypto!(sr25519, KEY_TYPE);

		pub struct TestAuthId;
		// implemented for runtime
		impl frame_system::offchain::AppCrypto<MultiSigner, MultiSignature> for TestAuthId {
			type RuntimeAppPublic = Public;
			type GenericSignature = sp_core::sr25519::Signature;
			type GenericPublic = sp_core::sr25519::Public;
		}

		// implemented for mock runtime in test
		impl frame_system::offchain::AppCrypto<<Sr25519Signature as Verify>::Signer, Sr25519Signature>
		for TestAuthId
		{
			type RuntimeAppPublic = Public;
			type GenericSignature = sp_core::sr25519::Signature;
			type GenericPublic = sp_core::sr25519::Public;
		}
	}

	#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
	pub struct Payload<Public> {
		number: u64,
		public: Public,
	}

	impl<T: SigningTypes> SignedPayload<T> for Payload<T::Public> {
		fn public(&self) -> T::Public {
			self.public.clone()
		}
	}

	// ref: https://serde.rs/container-attrs.html#crate
	#[derive(Deserialize, Encode, Decode, Default, RuntimeDebug, scale_info::TypeInfo)]
	struct SnapshotInfo {
		// Specify our own deserializing function to convert JSON string to vector of bytes
		#[serde(deserialize_with = "de_string_to_bytes")]
		icon_address: Vec<u8>,
		amount: u32,
		defi_user: bool,
		vesting_percentage: u32,
	}

	#[derive(Debug, Deserialize, Encode, Decode, Default)]
	struct IndexingData(Vec<u8>, u64);

	pub fn de_string_to_bytes<'de, D>(de: D) -> Result<Vec<u8>, D::Error>
		where
			D: Deserializer<'de>,
	{
		let s: &str = Deserialize::deserialize(de)?;
		Ok(s.as_bytes().to_vec())
	}

	#[pallet::config]
	pub trait Config: frame_system::Config /* + CreateSignedTransaction<Call<Self>> */ {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// The overarching dispatch call type.
		type Call: From<frame_system::Call<Self>>;
		// /// The identifier type for an offchain worker.
		// type AuthorityId: AppCrypto<Self::Public, Self::Signature>;
	}

	#[pallet::pallet]
	// #[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	// The pallet's runtime storage items.
	// https://substrate.dev/docs/en/knowledgebase/runtime/storage
	// #[pallet::storage]
	// #[pallet::getter(fn numbers)]
	// Learn more about declaring storage items:
	// https://substrate.dev/docs/en/knowledgebase/runtime/storage#declaring-storage-items
	// pub type Numbers<T> = StorageValue<_, VecDeque<u64>, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		NewNumber(Option<T::AccountId>, u64),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		// Error returned when not sure which ocw function to executed
		UnknownOffchainMux,

		// Error returned when making signed transactions in off-chain worker
		NoLocalAcctForSigning,
		OffchainSignedTxError,

		// Error returned when making unsigned transactions in off-chain worker
		OffchainUnsignedTxError,

		// Error returned when making unsigned transactions with signed payloads in off-chain worker
		OffchainUnsignedTxSignedPayloadError,

		// Error returned when fetching github info
		HttpFetchingError,
		DeserializeToObjError,
		DeserializeToStrError,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn offchain_worker(block_number: T::BlockNumber) {

			log::info!("Hello from pallet-ocw.");
			// Run a chain query as an example.
			let rpc_endpoint = "https://rinkeby.infura.io/v3/18b15ac5b3e8447191c6b233dcd2ce14";
			let chain_id = ChainQueries::chain_id(rpc_endpoint);
			match chain_id {
				Ok(cid) =>
					log::info!("Chain ID fetched: {} ", cid),
				Err(e) =>
					log::error!("Error!: {:?} ", e)
			}

			// const TX_TYPES: u32 = 1;
			// let modu = block_number.try_into().map_or(TX_TYPES, |bn: usize| (bn as u32) % TX_TYPES);
			// let result = match modu {
			// 	0 => Self::fetch_remote_info(),
			// 	_ => Err(Error::<T>::UnknownOffchainMux),
			// };
			//
			// if let Err(e) = result {
			// 	log::error!("offchain_worker error: {:?}", e);
			// }
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {}

	impl<T: Config> Pallet<T> {
		/// Check if we have fetched the data before. If yes, we can use the cached version
		///   stored in off-chain worker storage `storage`. If not, we fetch the remote info and
		///   write the info into the storage for future retrieval.
		fn fetch_remote_info() -> Result<(), Error<T>> {
			// Create a reference to Local Storage value.
			// Since the local storage is common for all offchain workers, it's a good practice
			// to prepend our entry with the pallet name.
			let s_info = StorageValueRef::persistent(b"offchain-demo::hn-info");

			// Local storage is persisted and shared between runs of the offchain workers,
			// offchain workers may run concurrently. We can use the `mutate` function to
			// write a storage entry in an atomic fashion.
			//
			// With a similar API as `StorageValue` with the variables `get`, `set`, `mutate`.
			// We will likely want to use `mutate` to access
			// the storage comprehensively.
			//
			if let Ok(Some(info)) = s_info.get::<SnapshotInfo>() {
				// hn-info has already been fetched. Return early.
				log::info!("cached snapshot-info: {:?}", info);
				return Ok(());
			}

			// Since off-chain storage can be accessed by off-chain workers from multiple runs, it is important to lock
			//   it before doing heavy computations or write operations.
			//
			// There are four ways of defining a lock:
			//   1) `new` - lock with default time and block exipration
			//   2) `with_deadline` - lock with default block but custom time expiration
			//   3) `with_block_deadline` - lock with default time but custom block expiration
			//   4) `with_block_and_time_deadline` - lock with custom time and block expiration
			// Here we choose the most custom one for demonstration purpose.
			let mut lock = StorageLock::<BlockAndTime<Self>>::with_block_and_time_deadline(
				b"offchain-demo::lock", LOCK_BLOCK_EXPIRATION,
				Duration::from_millis(LOCK_TIMEOUT_EXPIRATION)
			);

			// We try to acquire the lock here. If failed, we know the `fetch_n_parse` part inside is being
			//   executed by previous run of ocw, so the function just returns.
			if let Ok(_guard) = lock.try_lock() {
				match Self::fetch_n_parse() {
					Ok(info) => { s_info.set(&info); }
					Err(err) => { return Err(err); }
				}
			}
			Ok(())
		}

		/// Fetch from remote and deserialize the JSON to a struct
		fn fetch_n_parse() -> Result<SnapshotInfo, Error<T>> {

			let resp_bytes = Self::fetch_from_remote().map_err(|e| {
				log::error!("fetch_from_remote error: {:?}", e);
				<Error<T>>::HttpFetchingError
			})?;

			let resp_str = str::from_utf8(&resp_bytes).map_err(|_| <Error<T>>::DeserializeToStrError)?;
			// Print out our fetched JSON string

			// Deserializing JSON to struct, thanks to `serde` and `serde_derive`

			let info: SnapshotInfo =
				serde_json::from_str(&resp_str).map_err(|_| <Error<T>>::DeserializeToObjError)?;
			Ok(info)
		}

		/// This function uses the `offchain::http` API to query the remote endpoint information,
		///   and returns the JSON response as vector of bytes.
		fn fetch_from_remote() -> Result<Vec<u8>, Error<T>> {
			// Initiate an external HTTP GET request. This is using high-level wrappers from `sp_runtime`.
			let request = http::Request::get(HTTP_REMOTE_REQUEST);

			// Keeping the offchain worker execution time reasonable, so limiting the call to be within 3s.
			let timeout = sp_io::offchain::timestamp()
				.add(Duration::from_millis(FETCH_TIMEOUT_PERIOD));


			let pending = request
				// .add_header("User-Agent", HTTP_HEADER_USER_AGENT)
				.deadline(timeout) // Setting the timeout time
				.send() // Sending the request out by the host
				.map_err(|e| {
					log::error!("{:?}", e);
					<Error<T>>::HttpFetchingError
				})?;

			// By default, the http request is async from the runtime perspective. So we are asking the
			//   runtime to wait here
			// The returning value here is a `Result` of `Result`, so we are unwrapping it twice by two `?`
			//   ref: https://docs.substrate.io/rustdocs/latest/sp_runtime/offchain/http/struct.PendingRequest.html#method.try_wait

			let response = pending
				.try_wait(timeout)
				.map_err(|e| {
					// log::error!("{:?}", e);
					panic!("panicked 1: {:?}", e);
					<Error<T>>::HttpFetchingError
				})?
				.map_err(|e| {
					// log::error!("{:?}", e);
					panic!("panicked 2: {:?}", e);
					<Error<T>>::HttpFetchingError
				})?;

			if response.code != 200 {
				log::error!("Unexpected http request status code: {}", response.code);
				return Err(<Error<T>>::HttpFetchingError);
			}

			// Next we fully read the response body and collect it to a vector of bytes.
			Ok(response.body().collect::<Vec<u8>>())
		}
	}

	impl<T: Config> BlockNumberProvider for Pallet<T> {
		type BlockNumber = T::BlockNumber;

		fn current_block_number() -> Self::BlockNumber {
			<frame_system::Pallet<T>>::block_number()
		}
	}
}
