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
use codec::{Decode, Encode};
use serde::{Deserialize, Serialize};

/// ## BTCConfig
///
/// The `BTCConfig` structure represents the configuration for Bitcoin-related settings.
///
/// Fields:
/// - `btc_rpc_url`: A vector of bytes representing the Bitcoin RPC URL.
/// - `signer_public_key`: A vector of bytes representing the public key of the signer.
#[derive(
	Clone,
	Eq,
	PartialEq,
	Decode,
	Encode,
	Debug,
	Serialize,
	Deserialize,
	scale_info::TypeInfo,
	Default,
)]
pub struct BTCConfig {
	pub btc_rpc_url: Vec<u8>,
	pub signer_public_key: Vec<u8>,
}

/// ## OffchainErr
///
/// The `OffchainErr` enum represents errors that can occur during off-chain operations.
///
/// Variants:
/// - `RPCError`: Indicates an RPC error.
/// - `FailedSigning`: Indicates a failure in signing a transaction.
pub enum OffchainErr {
	RPCError,
	FailedSigning,
}

impl sp_std::fmt::Debug for OffchainErr {
	fn fmt(&self, fmt: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
		match *self {
			OffchainErr::FailedSigning => write!(fmt, "Unable to sign transaction"),
			OffchainErr::RPCError => write!(fmt, "RPC error"),
		}
	}
}

/// ## OffchainResult
///
/// The `OffchainResult` type alias is a shorthand for `Result` with the `OffchainErr` error type.
pub type OffchainResult<A> = Result<A, OffchainErr>;
