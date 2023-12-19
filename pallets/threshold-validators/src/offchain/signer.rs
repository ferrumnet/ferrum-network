use super::*;

// #[derive(Clone)]
pub struct ThresholdSignature {
	pub from: sr25519::Public,
	pub _signer: sr25519::Public,
}

impl ThresholdSignature {
	pub fn new(from: sr25519::Public, signer: &[u8]) -> Self {
		ThresholdSignature { from, _signer: sr25519::Public::try_from(signer).unwrap() }
	}

	pub fn sign(&self, hash: &[u8]) -> Result<sr25519::Signature, ()> {
		log::info!("Signer address is : {:?}", self.from);
		// TODO : We should handle this properly, if the signing is not possible maybe propogate the
		// error upstream
		let signed = sr25519_sign(BTC_OFFCHAIN_SIGNER_KEY_TYPE, &self.from, &hash).unwrap();
		Ok(signed)
	}
}

impl From<sr25519::Public> for ThresholdSignature {
	fn from(signer: sr25519::Public) -> Self {
		log::info!("PUBLIC KEY {:?}", signer);

		ThresholdSignature { _signer: signer, from: signer }
	}
}
