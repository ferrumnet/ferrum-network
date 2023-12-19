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
use electrum_client::{Client, ElectrumApi};
use sp_runtime::traits::Zero;
pub mod types;
use ark_ec::{AffineRepr, CurveGroup};
use ark_ff::PrimeField;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_std::{collections::BTreeMap, io::Write, rand::RngCore, vec, vec::Vec, UniformRand};
use bitcoin::Txid;
use digest::Digest;
use dock_crypto_utils::serde_utils::ArkObjectBytes;
use schnorr_pok::{
	compute_random_oracle_challenge, error::SchnorrError, impl_proof_of_knowledge_of_discrete_log,
};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use sp_core::sr25519;
pub use types::*;
use zeroize::{Zeroize, ZeroizeOnDrop};

pub mod client;
pub mod signer;
pub mod types;

impl<T: Config> Pallet<T> {
	pub fn execute_threshold_offchain_worker(
		block_number: u64,
		config: types::ThresholdConfig,
	) -> OffchainResult<()> {
		let current_pool_address = CurrentPoolAddress::<T>::get();

		// TODO : Fix this, we need to optimise this, starting every time is wasteful
		let _ = client::start();

		// if something in signing queue, then initiate signing
		let signing_queue = SigningQueue::<T>::get();

		if signing_queue.is_some() {
			initiate_signing(signing_queue, config);
			SigningQueue::<T>::clear();
		}

		Ok(())
	}

	pub fn initiate_keygen(caller: T::AccountId, config: types::ThresholdConfig) -> DispatchResult {
		let participants = CurrentQuorom::<T>::get();
		let threshold = CurrentPoolThreshold::<T>::get();
		let schnorr_ctx = b"test-ctx";
		let pub_key = signer::ThresholdSignature::new(config.signer_public_key);
		let client = client::ThresholdClient::new();

		let participant_id = participants.find_by_index(caller).unwrap();
		let (round1_state, round1_msg) =
			Round1State::start_with_random_secret::<StdRng, Blake2b512>(
				rng,
				participant_id as ParticipantId,
				threshold as ShareId,
				total as ShareId,
				schnorr_ctx,
				pub_key,
			)
			.unwrap();

		// Send the signature to the threshold signing function over the network
		client.broadcast_message(round1_state.secret);

		let mut all_round2_states = vec![];

		// TODO : Improve this, we will keep on looping till next block
		loop {
			// loop till we receive a message
			client.handle_client();

			for msg in client.message_queue {
				if !all_round2_states.contains(msg) {
					all_round2_states.push(msg)
				}
			}

			if all_round2_states.len() == threshold {
				break
			}
		}

		let (share, pk, t_pk) = all_round2_states[i].clone().finish(pub_key).unwrap();

		let key = feldman_dvss_dkg::reconstruct_threshold_public_key(all_round2_states, threshold)
			.unwrap();

		// post the key to chain
		CurrentPubKey::<T>::put(key.to_vec());

		client.prev_shares.set(all_round2_states);

		Ok(())
	}

	pub fn initiate_signing(msg: &str, config: types::ThresholdConfig) -> DispatchResult {
		let participants = CurrentQuorom::<T>::get();
		let threshold = CurrentPoolThreshold::<T>::get();
		let pub_key = signer::ThresholdSignature::new(config.signer_public_key);
		let client = client::ThresholdClient::new();

		let final_shares = Shares(client.prev_shares);
		let key = final_shares.reconstruct_secret().unwrap();

		let signature = key.sign(msg).expect("Signing with reconstructed key failed!");

		Signatures::<T>::push(signature);

		Ok(())
	}
}
