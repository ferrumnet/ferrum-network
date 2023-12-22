//! This is the keygen implemented in the [FROST paper](https://eprint.iacr.org/2020/852.pdf) in Figure 1.
//! This is a slight addition to the DKG based on Feldman VSS as it contains a Schnorr proof of
//! knowledge of the secret key.

use crate::{
	common::{CommitmentToCoefficients, ParticipantId, Share, ShareId, Shares},
	error::SSError,
	feldman_dvss_dkg, feldman_vss,
};
use ark_ec::{AffineRepr, CurveGroup};
use ark_ff::PrimeField;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_std::{collections::BTreeMap, io::Write, rand::RngCore, vec, vec::Vec, UniformRand};
use digest::Digest;
use dock_crypto_utils::serde_utils::ArkObjectBytes;
use schnorr_pok::{
	compute_random_oracle_challenge, error::SchnorrError, impl_proof_of_knowledge_of_discrete_log,
};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use zeroize::{Zeroize, ZeroizeOnDrop};

impl_proof_of_knowledge_of_discrete_log!(SecretKeyKnowledgeProtocol, SecretKeyKnowledge);

/// State of a participant during Round 1
#[serde_as]
#[derive(
	Clone, Debug, PartialEq, Eq, CanonicalSerialize, CanonicalDeserialize, Serialize, Deserialize,
)]
#[serde(bound = "")]
pub struct Round1State<G: AffineRepr> {
	pub id: ParticipantId,
	pub threshold: ShareId,
	pub shares: Shares<G::ScalarField>,
	/// Stores the commitment to the coefficients of the polynomial by each participant
	pub coeff_comms: BTreeMap<ParticipantId, CommitmentToCoefficients<G>>,
	/// Secret chosen by the participant
	#[serde_as(as = "ArkObjectBytes")]
	pub secret: G::ScalarField,
}

/// Message sent by a participant during Round 1
#[derive(
	Clone, Debug, PartialEq, Eq, CanonicalSerialize, CanonicalDeserialize, Serialize, Deserialize,
)]
#[serde(bound = "")]
pub struct Round1Msg<G: AffineRepr> {
	pub sender_id: ParticipantId,
	pub comm_coeffs: CommitmentToCoefficients<G>,
	/// Proof of knowledge of the secret key for the public key
	pub schnorr_proof: SecretKeyKnowledge<G>,
}

/// State of a participant during Round 2
#[derive(
	Clone, Debug, PartialEq, Eq, CanonicalSerialize, CanonicalDeserialize, Serialize, Deserialize,
)]
#[serde(bound = "")]
pub struct Round2State<G: AffineRepr> {
	pub id: ParticipantId,
	pub threshold: ShareId,
	/// Stores the shares sent by each participant
	pub shares: BTreeMap<ParticipantId, Share<G::ScalarField>>,
	/// Stores the commitment to the coefficients of the polynomial by each participant. Created
	/// during Round 1
	pub coeff_comms: BTreeMap<ParticipantId, CommitmentToCoefficients<G>>,
}

impl<G: AffineRepr> Round1State<G> {
	/// Start Phase 1 with a randomly generated secret. `schnorr_proof_ctx` is the context used in
	/// the Schnorr proof to prevent replay attacks. `pk_gen` is the EC group generator for the
	/// public key
	pub fn start_with_random_secret<'a, R: RngCore, D: Digest>(
		rng: &mut R,
		participant_id: ParticipantId,
		threshold: ShareId,
		total: ShareId,
		schnorr_proof_ctx: &[u8],
		pk_gen: impl Into<&'a G> + Clone,
	) -> Result<(Self, Round1Msg<G>), SSError> {
		let secret = G::ScalarField::rand(rng);
		Self::start_with_given_secret::<R, D>(
			rng,
			participant_id,
			secret,
			threshold,
			total,
			schnorr_proof_ctx,
			pk_gen,
		)
	}

	/// Similar to `Self::start_with_random_secret` except it expects a secret from the caller.
	pub fn start_with_given_secret<'a, R: RngCore, D: Digest>(
		rng: &mut R,
		id: ParticipantId,
		secret: G::ScalarField,
		threshold: ShareId,
		total: ShareId,
		schnorr_proof_ctx: &[u8],
		pk_gen: impl Into<&'a G> + Clone,
	) -> Result<(Self, Round1Msg<G>), SSError> {
		if id == 0 || id > total {
			return Err(SSError::InvalidParticipantId(id))
		}
		// Create shares of the secret and commit to it
		let (shares, commitments, _) =
			feldman_vss::deal_secret::<R, G>(rng, secret, threshold, total, pk_gen.clone())?;
		let mut coeff_comms = BTreeMap::new();
		coeff_comms.insert(id, commitments.clone());

		let pk_gen = pk_gen.into();
		// Create the proof of knowledge for the secret key
		let blinding = G::ScalarField::rand(rng);
		let schnorr = SecretKeyKnowledgeProtocol::init(secret, blinding, pk_gen);
		let mut challenge_bytes = vec![];
		schnorr
			.challenge_contribution(
				pk_gen,
				commitments.commitment_to_secret(),
				&mut challenge_bytes,
			)
			.map_err(SSError::SchnorrError)?;
		challenge_bytes.extend_from_slice(schnorr_proof_ctx);
		let challenge = compute_random_oracle_challenge::<G::ScalarField, D>(&challenge_bytes);
		let schnorr_proof = schnorr.gen_proof(&challenge);
		Ok((
			Round1State { id, threshold, shares, coeff_comms, secret },
			Round1Msg { sender_id: id, comm_coeffs: commitments, schnorr_proof },
		))
	}

	/// Called by a participant when it receives a message during Round 1
	pub fn add_received_message<'a, D: Digest>(
		&mut self,
		msg: Round1Msg<G>,
		schnorr_proof_ctx: &[u8],
		pk_gen: impl Into<&'a G>,
	) -> Result<(), SSError> {
		if msg.sender_id == self.id {
			return Err(SSError::SenderIdSameAsReceiver(msg.sender_id, self.id))
		}
		if !msg.comm_coeffs.supports_threshold(self.threshold) {
			return Err(SSError::DoesNotSupportThreshold(self.threshold))
		}
		let pk_gen = pk_gen.into();
		// Verify Schnorr proof
		let mut challenge_bytes = vec![];
		msg.schnorr_proof
			.challenge_contribution(
				pk_gen,
				msg.comm_coeffs.commitment_to_secret(),
				&mut challenge_bytes,
			)
			.map_err(SSError::SchnorrError)?;
		challenge_bytes.extend_from_slice(schnorr_proof_ctx);
		let challenge = compute_random_oracle_challenge::<G::ScalarField, D>(&challenge_bytes);
		if !msg
			.schnorr_proof
			.verify(msg.comm_coeffs.commitment_to_secret(), pk_gen, &challenge)
		{
			return Err(SSError::InvalidProofOfSecretKeyKnowledge)
		}

		// Store commitments
		self.coeff_comms.insert(msg.sender_id, msg.comm_coeffs);
		Ok(())
	}

	/// Participant finishes Round 1 and starts Round 2.
	pub fn finish(self) -> Result<(Round2State<G>, Shares<G::ScalarField>), SSError> {
		// Check that sufficient shares present
		let len = self.shares.0.len() as ShareId;
		if self.threshold > (len + 1) {
			// + 1 because its own share will be added later
			return Err(SSError::BelowThreshold(self.threshold, len))
		}
		let mut shares = BTreeMap::new();
		shares.insert(self.id, self.shares.0[self.id as usize - 1].clone());
		Ok((
			Round2State {
				id: self.id,
				threshold: self.threshold,
				shares,
				coeff_comms: self.coeff_comms,
			},
			self.shares,
		))
	}

	pub fn total_participants(&self) -> usize {
		self.coeff_comms.len()
	}
}

impl<G: AffineRepr> Round2State<G> {
	/// Called by a participant when it receives its share during Round 1
	pub fn add_received_share<'a>(
		&mut self,
		sender_id: ShareId,
		share: Share<G::ScalarField>,
		pk_gen: impl Into<&'a G>,
	) -> Result<(), SSError> {
		if sender_id == self.id {
			return Err(SSError::SenderIdSameAsReceiver(sender_id, self.id))
		}
		if self.shares.contains_key(&sender_id) {
			return Err(SSError::AlreadyProcessedFromSender(sender_id))
		}
		if self.id != share.id {
			return Err(SSError::UnequalParticipantAndShareId(self.id, share.id))
		}
		if self.threshold != share.threshold {
			return Err(SSError::UnequalThresholdInReceivedShare(self.threshold, share.threshold))
		}
		if let Some(comm) = self.coeff_comms.get(&sender_id) {
			share.verify(comm, pk_gen.into())?;
			self.shares.insert(sender_id, share);
			Ok(())
		} else {
			Err(SSError::ParticipantNotAllowedInPhase2(sender_id))
		}
	}

	/// Participant finishes Round 1 and outputs final share that contains its own secret key, its
	/// own public key and the threshold public key
	pub fn finish<'a>(
		self,
		pk_gen: impl Into<&'a G>,
	) -> Result<(Share<G::ScalarField>, G, G), SSError> {
		feldman_dvss_dkg::SharesAccumulator::gen_final_share_and_public_key(
			self.id,
			self.threshold,
			self.shares,
			self.coeff_comms,
			pk_gen.into(),
		)
	}
}
