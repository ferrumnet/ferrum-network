#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

mod chain_queries;
mod chain_utils;
mod qp_types;
mod erc_20_client;
mod contract_client;
mod quantum_portal_client;
pub mod quantum_portal_service;

#[frame_support::pallet]
pub mod pallet {
	//! A demonstration of an offchain worker that sends onchain callbacks
	use core::{convert::TryInto};
	use sp_std::ops::Div;
	use parity_scale_codec::{Decode, Encode};
	use frame_support::pallet_prelude::*;
	use frame_system::{
		pallet_prelude::*,
		offchain::{
			AppCrypto, CreateSignedTransaction, SendSignedTransaction, SendUnsignedTransaction,
			SignedPayload, Signer, SigningTypes, SubmitTransaction, SignMessage,
		},
	};
	use libsecp256k1::Message;
	use sp_core::{crypto::KeyTypeId, H160, H256, U256};
	use sp_runtime::{offchain::{
		http,
		storage::StorageValueRef,
		storage_lock::{BlockAndTime, StorageLock},
		Duration,
	}, traits::BlockNumberProvider, transaction_validity::{
		InvalidTransaction, TransactionSource, TransactionValidity, ValidTransaction,
	}, RuntimeDebug, MultiSignature};
	use sp_std::{collections::vec_deque::VecDeque, prelude::*, str};
	use sp_core::{ecdsa};
	use serde::{Deserialize, Deserializer};
	use sp_arithmetic::traits::AtLeast32BitUnsigned;
	use sp_core::crypto::{AccountId32, ByteArray};
	use sp_runtime::MultiSignature::Ecdsa;
	use sp_runtime::traits::AccountIdConversion;
	use ethabi_nostd::Address;
	use crate::chain_queries::ChainQueries;
	use crate::chain_utils::{ChainUtils, EMPTY_HASH};
	use crate::contract_client::{ContractClient, ContractClientSignature};
	use crate::crypto::TestAuthId;
	use crate::erc_20_client::Erc20Client;
	use crate::quantum_portal_client::QuantumPortalClient;
	use crate::quantum_portal_service::{PendingTransaction, QuantumPortalService};

	/// Defines application identifier for crypto keys of this module.
	///
	/// Every module that deals with signatures needs to declare its unique identifier for
	/// its crypto keys.
	/// When an offchain worker is signing transactions it's going to request keys from type
	/// `KeyTypeId` via the keystore to sign the transaction.
	/// The keys can be inserted manually via RPC (see `author_insertKey`).
	pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"dem!");
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
		use libsecp256k1::{ECMULT_CONTEXT, Message};
		use log::log;
		use sp_core::{H256, sr25519};
		use crate::KEY_TYPE;
		use sp_core::ecdsa::{Signature as EcdsaSignagure};
		use sp_std::prelude::*;
		use sp_runtime::{
			app_crypto::{app_crypto, ecdsa},
			traits::Verify, MultiSignature, MultiSigner
		};
		use sp_runtime::MultiSigner::Ecdsa;
		use sp_std::str;
		use parity_scale_codec::{Decode, Encode};
		use crate::chain_utils::ChainUtils;

		app_crypto!(ecdsa, KEY_TYPE);
		// app_crypto!(sr25519, KEY_TYPE);

		pub struct TestAuthId;
		// implemented for runtime
		impl frame_system::offchain::AppCrypto<MultiSigner, MultiSignature> for TestAuthId {
			type RuntimeAppPublic = Public;
			type GenericSignature = sp_core::ecdsa::Signature; // sr25519::Signature;
			type GenericPublic = sp_core::ecdsa::Public;
			// sr25519::Public;
			// type GenericSignature = sp_core::sr25519::Signature;
			// type GenericPublic = sp_core::sr25519::Public;
			fn sign(payload: &[u8], public: MultiSigner) -> Option<MultiSignature> {
				let ecdsa_pub = match public {
					Ecdsa(p) => p,
					_ => panic!("Wrong public type"),
				};
				let hash = H256::from_slice(payload); // ChainUtils::keccack(payload);
				let sig = ChainUtils::sign_transaction_hash(
					&ecdsa_pub, &hash).unwrap();
				log::info!("Pub is {:?}", ecdsa_pub.0);
				// Get the long pub and hence the address of the signer
				let lk = libsecp256k1::PublicKey::parse_slice(&ecdsa_pub.0, None).unwrap();
				let lks = lk.serialize();
				log::info!("Pub long {:?}", lks);
				let addr0 = ChainUtils::eth_address_from_public_key(&lks[1..]);
				log::info!("Addr {:?}", addr0.as_slice());
				log::info!("Addr {:?}", str::from_utf8(ChainUtils::bytes_to_hex(addr0.as_slice()).as_slice()).unwrap());

				let mut buf: [u8; 65] = [0; 65];
				buf.copy_from_slice(sig.as_slice());
				let signature = ecdsa::Signature(buf);
				log::info!("Signing msg: {} - {}",
					str::from_utf8(ChainUtils::bytes_to_hex(hash.as_bytes()).as_slice()).unwrap(),
					str::from_utf8(ChainUtils::bytes_to_hex(sig.as_slice()).as_slice()).unwrap(),
				);

				// Recover to make sure
				let msg = Message::parse_slice(&hash.0).unwrap();
				let sig2 = libsecp256k1::Signature::parse_standard_slice(&sig.as_slice()[..64]).unwrap();
				let rec = &libsecp256k1::RecoveryId::parse(sig[64]).unwrap();
				let rec = ECMULT_CONTEXT.recover_raw(&sig2.r, &sig2.s,
				rec.serialize(), &msg.0).unwrap();
				let the_pub = libsecp256k1::PublicKey::try_from(rec).unwrap();
				let the_pub = the_pub.serialize();
				log::info!("the pub {:?}", the_pub);
				let the_pub_s = ChainUtils::bytes_to_hex(the_pub.as_slice());
				log::info!("pubsize is {}", &the_pub.len());
				let add_s = ChainUtils::eth_address_from_public_key(&the_pub.as_slice()[1..]);
				log::info!("Recovered pub is {}", str::from_utf8(the_pub_s.as_slice()).unwrap());
				log::info!("Recovered addr is {}", str::from_utf8(ChainUtils::bytes_to_hex(add_s.as_slice()).as_slice()).unwrap());
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
			// type GenericSignature = sp_core::sr25519::Signature;
			// type GenericPublic = sp_core::sr25519::Public;
			// fn sign(payload: &[u8], public: <EcdsaSignagure as Verify>::Signer) -> Option<EcdsaSignagure> {
			// 	log::info!("SIGN REQEUSTED!");
			// 	// todo!()
			// 	None
			// }
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
	pub trait Config: CreateSignedTransaction<Call<Self>> + frame_system::Config {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// The overarching dispatch call type.
		type Call: From<frame_system::Call<Self>>;
		// /// The identifier type for an offchain worker.
		type AuthorityId: AppCrypto<Self::Public, Self::Signature>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	// The pallet's runtime storage items.
	// https://substrate.dev/docs/en/knowledgebase/runtime/storage
	#[pallet::storage]
	// Learn more about declaring storage items:
	// https://substrate.dev/docs/en/knowledgebase/runtime/storage#declaring-storage-items
	#[pallet::getter(fn numbers)]
	pub(super) type Numbers<T> = StorageValue<_, u64, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn pending_transactions)]
	pub(super) type PendingTransactions<T: Config> = StorageMap<_,
		Identity, u64, PendingTransaction, ValueQuery>;

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

	impl<T: Config> Pallet<T> {
		pub fn test_qp(block_number: u64) {
			let signer_rinkeby = Signer::<T, T::AuthorityId>::any_account();
			let rpc_endpoint_rinkeby = "https://rinkeby.infura.io/v3/18b15ac5b3e8447191c6b233dcd2ce14";
			let lgr_mgr_rinkeby = ChainUtils::hex_to_address(
				b"d36312d594852462d6760042e779164eb97301cd");
			log::info!("contract address is {:?}", lgr_mgr_rinkeby);
			let client_rinkeby = ContractClient::new(
				rpc_endpoint_rinkeby.clone(), &lgr_mgr_rinkeby, 4);
			let c_rinkeby = QuantumPortalClient::new(
				client_rinkeby,
				ContractClientSignature::from(signer_rinkeby),
				sp_io::offchain::timestamp().unix_millis(),
				block_number,
			);

			let signer_bsctestnet = Signer::<T, T::AuthorityId>::any_account();
			let rpc_endpoint_bsctestnet = "https://data-seed-prebsc-1-s1.binance.org:8545";
			let lgr_mgr_bsctestnet = ChainUtils::hex_to_address(
				b"d36312d594852462d6760042e779164eb97301cd");
			log::info!("contract address is {:?}", lgr_mgr_bsctestnet);
			let client_bsctestnet = ContractClient::new(
				rpc_endpoint_bsctestnet.clone(), &lgr_mgr_bsctestnet, 97);
			let c_bsctestnet = QuantumPortalClient::new(
				client_bsctestnet,
				ContractClientSignature::from(signer_bsctestnet),
				sp_io::offchain::timestamp().unix_millis(),
				block_number,
			);

			let svc = QuantumPortalService::<T>::new(
				vec![c_rinkeby, c_bsctestnet]
			);
			svc.process_pair(4, 97).unwrap();
		}

		fn is_block_ready(c: &QuantumPortalClient) {
			let ibr = c.is_local_block_ready(4).unwrap();
			log::info!("Is block read {:?}", ibr)
		}

		fn last_remote_mined_block(c: &QuantumPortalClient) {
			c.last_remote_mined_block(4).unwrap();
		}

		fn local_block_by_nonce(c: &QuantumPortalClient) {
			c.local_block_by_nonce(4, 0).unwrap();
		}

		fn create_finalize_transaction(c: &QuantumPortalClient) {
			c.create_finalize_transaction(
				4,
				0,
				H256::zero(),
				&[],
			).unwrap();
		}

		fn finalize_block(c: &QuantumPortalClient, chain_id: u64) {
			c.finalize(chain_id).unwrap();
		}

		fn mine_block(c: &QuantumPortalClient, chain1: u64, chain2: u64) {
			c.mine(chain1, chain2).unwrap();
		}

		fn run_mine_and_finalize(block_number: u64) {
			// Create a number of clients
			let supported_chains = [4, 69, 2600];

			// For each chain get an rpc endpoint
			// For each chain get a signer (for now use the same signer)
			// For each chain create a client
			let rpc_endpoint = "https://rinkeby.infura.io/v3/18b15ac5b3e8447191c6b233dcd2ce14";
			let lgr_mgr = ChainUtils::hex_to_address(
				b"d36312d594852462d6760042e779164eb97301cd");
			log::info!("000===contract address is {:?}", lgr_mgr);
			let client = ContractClient::new(
				rpc_endpoint.clone(), &lgr_mgr, 4);
			log::info!("001===contract address is {}", str::from_utf8(
				ChainUtils::address_to_hex(client.contract_address).as_slice()).unwrap());
			let signer = Signer::<T, T::AuthorityId>::any_account();

			let c = QuantumPortalClient::new(
				client,
				ContractClientSignature::from(signer),
				sp_io::offchain::timestamp().unix_millis(),
				block_number,
			);

			let pairs = [[4, 2600], [2600, 4]];

			// For the pair
			let clients = vec![c];
			let qps = QuantumPortalService::<T>::new(clients);
			pairs.into_iter().for_each(
				|p| qps.process_pair(p[0], p[1]).unwrap()
			);
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn offchain_worker(block_number: T::BlockNumber) {
			log::info!("Hello from pallet-ocw.");
			let bno = block_number.try_into().map_or(0 as u64, |f| f);
			Self::test_qp(bno);

			// Run a chain query as an example.
			// let rpc_endpoint = "https://test1234.requestcatcher.com/test";
			// let rpc_endpoint = "https://rinkeby.infura.io/v3/18b15ac5b3e8447191c6b233dcd2ce14";

			// let alice_adress = Address::from_slice(
			// 	hex::decode("0xd43593c715fdd31c61141abd04a99fd6822c8558")
			// 		.unwrap().as_slice());
			// let signer = Signer::<T, T::AuthorityId>::any_account();
			// // let msg = sp_core::keccak_256(b"Some msg");
			// log::info!("Signer is {:?}", &signer.can_sign());
			// log::info!("ID IS {:?}", str::from_utf8(&sp_core::ecdsa::CRYPTO_ID.0).unwrap());
			// let signed = signer.sign_message(&EMPTY_HASH.0);
			// let signed_f = signed;
			// match signed_f {
			// 	Some((v, s)) => {
			// 		// let mut buf: [u8; 33] = [0 as u8; 33];
			// 		// log::info!("PRE buf.copy_from_slice");
			// 		// buf.copy_from_slice(v.public.encode().as_slice());
			// 		let public_key = v.public.encode();
			// 		let public_key = &public_key.as_slice()[1..];
			// 		log::info!("Pub key dangled {:?}", str::from_utf8(ChainUtils::bytes_to_hex(
			// 			public_key).as_slice()).unwrap());
			// 		let addr = ChainUtils::eth_address_from_public_key(&public_key);
			// 		log::info!("Signer address is {:?}", str::from_utf8(ChainUtils::bytes_to_hex(
			// 			addr.as_slice()).as_slice()).unwrap());
			// 		log::info!("And then signature is: {:?}",
			// 			str::from_utf8(ChainUtils::bytes_to_hex(s.encode().as_slice()).as_slice()).unwrap());
			// 	},
			// 	None => {
			// 		log::info!("No signed msg!");
			// 	}
			// };

			// let chain_id = ChainQueries::chain_id(rpc_endpoint);
			// match chain_id {
			// 	Ok(cid) =>
			// 		log::info!("Chain ID fetched: {} ", cid),
			// 	Err(e) =>
			// 		log::error!("Error!: {:?} ", e)
			// }

			// let contract_f = ChainUtils::hex_to_address(
			// 	b"00bdf74f702723a880e46efec4982b3fe9414795");
			// let alice_address = ChainUtils::hex_to_address(
			// 	b"e04cc55ebee1cbce552f250e85c57b70b2e2625b");
			// let client = ContractClient::new(
			// 	rpc_endpoint.clone(), &contract_f, 4);
			// let erc_20 = Erc20Client::new(client);
			// log::info!("Erc20 address got");
			// let ts = erc_20.approve(
			// 	alice_address,
			// 	U256::from(999999999 as u64),
			// 	alice_address,
			// 	|h| {
			// 		let signer = Signer::<T, T::AuthorityId>::any_account();
			// 		log::info!("Signer is {:?}", &signer.can_sign());
			// 		let signed = signer.sign_message(&h.0);
			// 		let signed_m = match signed {
			// 			None => panic!("No signature"),
			// 			Some((a, b)) => {
			// 				let public_key = a.public.encode();
			// 				let public_key = &public_key.as_slice()[1..];
			// 				let addr = ChainUtils::eth_address_from_public_key(
			// 					public_key);
			// 				log::info!("Signer address is {:?}", str::from_utf8(ChainUtils::bytes_to_hex(
			// 					addr.as_slice()).as_slice()).unwrap());
			// 				b
			// 			},
			// 		};
			// 		let sig_bytes = signed_m.encode();
			// 		log::info!("Got a signature of size {}: {}", sig_bytes.len(),
			// 			str::from_utf8(ChainUtils::bytes_to_hex(sig_bytes.as_slice()).as_slice())
			// 			.unwrap());
			// 		ecdsa::Signature::try_from(&sig_bytes.as_slice()[1..]).unwrap()
			// 	},
			// ).unwrap();
			// log::info!("Sent 'approve' and got tx hash: {}", str::from_utf8(
			// 	ChainUtils::h256_to_hex_0x(&ts).as_slice()).unwrap());

			// let ts = erc_20.total_supply();
			// log::info!("Total supply got {:?}", &ts);
			// match ts {
			// 	Ok(cid) => {
			// 		log::info!("Total Supply fetched: {} ", cid);
			// 		let num = cid.div(U256::from((10 as u64).pow(18))).as_u64();
			// 		log::info!("Total Supply fetched human readable: {} ", num);
			// 	},
			// 	Err(e) =>
			// 		log::error!("Error getting ts!: {:?} ", e)
			// }

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
