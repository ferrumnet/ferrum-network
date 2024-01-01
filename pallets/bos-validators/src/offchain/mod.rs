// Copyright 2019-2023 Ferrum Inc.
// This file is part of Ferrum.

// Ferrum is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Ferrum is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Ferrum.  If not, see <http://www.gnu.org/licenses/>.
use super::*;
use crate::Config;
use frost_secp256k1 as frost;
use rand::thread_rng;
use sp_runtime::DispatchResult;
use sp_std::collections::BTreeMap;
use crypto_box::{
    aead::{generic_array::GenericArray, Aead, AeadInPlace, OsRng},
    PublicKey, SecretKey,
};
use curve25519_dalek::EdwardsPoint;
use hex_literal::hex;
// pub mod types;

// pub mod client;
// pub mod frost_dkg;
// pub mod signer;
pub mod types;

impl<T: Config> Pallet<T> {
	pub fn execute_threshold_offchain_worker(
		block_number: u64,
		config: types::ThresholdConfig,
	) -> OffchainResult<()> {
		let current_pool_address = CurrentPoolAddress::<T>::get();

		let current_pub_key = current_pub_key();

		if current_pub_key.is_none() {
			Self::initiate_keygen(config);
		}

		Ok(())
	}

	pub fn initiate_keygen(config: types::ThresholdConfig) -> DispatchResult {
		// if we have all round 1 shares, start round 2
		let round_1_shares = Round1Shares::<T>::iter().len();
		let round_2_shares = Round2Shares::<T>::iter().len();

		// TODO : Change this calculate dynamically
		if round_2_shares == 2 {
			keygen_complete(config);
		} else if round_1_shares == 2 {
			keygen_round_two(config);
		} else {
			keygen_round_one(config);
		}
	}

	pub fn keygen_round_one(config: types::ThresholdConfig) -> DispatchResult {
		let participants = CurrentQuorom::<T>::get();
		let threshold = CurrentPoolThreshold::<T>::get();

		// TODO : Ensure we have not already done round 1
		// Round1Shares::<T>::get(
		// 	receiver_participant_identifier,
		// 	participant_identifier,
		// 	round1_package,
		// );

		// TODO : Move this to DB
		let mut round1_secret_packages = BTreeMap::new();

		////////////////////////////////////////////////////////////////////////////
		// Key generation, Round 1
		////////////////////////////////////////////////////////////////////////////
		let participant_index = participants.find_by_index(caller).unwrap();
		let participant_identifier = participant_index.try_into().expect("should be nonzero");
		// ANCHOR: dkg_part1
		let (round1_secret_package, round1_package) = frost::keys::dkg::part1(
			participant_identifier,
			participants.len(),
			threshold,
			&mut rng,
		)?;
		// ANCHOR_END: dkg_part1

		// Store the participant's secret package for later use.
		// In practice each participant will store it in their own environment.
		round1_secret_packages.insert(participant_identifier, round1_secret_package);

		// "Send" the round 1 package to all other participants.
		for receiver_participant_index in 1..=participants.len() {
			if receiver_participant_index == participant_index {
				continue
			}
			let receiver_participant_identifier: frost::Identifier =
				receiver_participant_index.try_into().expect("should be nonzero");

			// push everyone shares to storage
			Round1Shares::<T>::insert(
				receiver_participant_identifier,
				participant_identifier,
				round1_package,
			);
		}

		// save our round1 secret to offchain worker storage
		let key = Self::derived_key(frame_system::Module::<T>::block_number());
		let data = IndexingData(b"round_1_share".to_vec(), number);
		offchain_index::set(&key, &data.encode());

		Ok(())
	}

	pub fn keygen_round_two(config: types::ThresholdConfig) -> DispatchResult {
		let participants = CurrentQuorom::<T>::get();
		let threshold = CurrentPoolThreshold::<T>::get();

		// TODO : Ensure we did not already complete round 2

		// TODO : Move this to DB
		// TODO : Ensure this key can be saved and restored
		let mut round2_secret_packages = BTreeMap::new();

		////////////////////////////////////////////////////////////////////////////
		// Key generation, Round 2
		////////////////////////////////////////////////////////////////////////////
		let participant_index = participants.find_by_index(caller).unwrap();
		let participant_identifier = participant_index.try_into().expect("should be nonzero");

		// get all shares sent to us
		let round_1_packages = Round1Shares::<T>::iter_prefix(participant_index);

		// get our round1 secret to offchain worker storage
		let key = Self::derived_key(frame_system::Module::<T>::block_number());
		let data = IndexingData(b"round_1_share".to_vec(), number);
		let round1_secret_package = offchain_index::get(&key);

		// ANCHOR: dkg_part2
		let (round2_secret_package, round2_packages) =
			frost::keys::dkg::part2(round1_secret_package, round1_packages)?;
		// ANCHOR_END: dkg_part2

		// "Send" the round 1 package to all other participants.
		for receiver_participant_index in 1..=participants.len() {
			if receiver_participant_index == participant_index {
				continue
			}
			let receiver_participant_identifier: frost::Identifier =
				receiver_participant_index.try_into().expect("should be nonzero");

			// Fetch the receiver participants pub key
			// then encrypt with their private key
			let secret_key = SecretKey::from(key);
            let public_key = PublicKey::from(receiver_participant_index);
            let nonce = GenericArray::from_slice(random_nonce());
            let mut buffer = round_1_package.to_vec();

            let tag = <Box>::new(&public_key, &secret_key)
                .encrypt_in_place_detached(nonce, round_1_package, &mut buffer)
                .unwrap();

			// push everyone shares to storage
			Round2Shares::<T>::insert(
				receiver_participant_identifier,
				participant_identifier,
				tag,
			);
		}

		// save our round2 secret to offchain worker storage
		let key = Self::derived_key(frame_system::Module::<T>::block_number());
		let data = IndexingData(b"round_2_share".to_vec(), number);
		offchain_index::set(&key, &data.encode());

		Ok(())
	}

	pub fn keygen_complete(config: types::ThresholdConfig) -> DispatchResult {
		let participants = CurrentQuorom::<T>::get();
		let threshold = CurrentPoolThreshold::<T>::get();

		// TODO : Move this to DB
		let mut round2_secret_packages = BTreeMap::new();

		////////////////////////////////////////////////////////////////////////////
		// Key generation, Round 2
		////////////////////////////////////////////////////////////////////////////
		let participant_index = participants.find_by_index(caller).unwrap();
		let participant_identifier = participant_index.try_into().expect("should be nonzero");

		// get all shares sent to us
		let round_1_packages = Round1Shares::<T>::iter_prefix(participant_index);
		let round_2_packages = Round2Shares::<T>::iter_prefix(participant_index);

		// get our round2 secret to offchain worker storage
		let key = Self::derived_key(frame_system::Module::<T>::block_number());
		let data = IndexingData(b"round_2_share".to_vec(), number);
		let round1_secret_package = offchain_index::get(&key);

		// decrypt key share
		let secret_key = SecretKey::from(key);
       	let public_key = PublicKey::from(receiver_participant_index);
		let nonce = GenericArray::from_slice(random_nonce());
		let mut buffer = round_1_package.to_vec();

		let round2_package = <Box>::new(&public_key, &secret_key)
		 	.decrypt(nonce, round_2_packages, &mut buffer)
		    .unwrap();

		// ANCHOR: dkg_part3
		let (key_package, pubkey_package) = frost::keys::dkg::part3(
			round2_secret_package,
			round1_packages,
			round2_packages,
		)?;
		// ANCHOR_END: dkg_part3

		// push the key to storage
		CurrentPubKey::<T>::set(pubkey_package)

		Ok(())
	}
}
