// Copyright 2022 Webb Technologies Inc.
//
// This file is part of pallet-evm-precompile-preimage package, originally developed by Purestake
// Inc. Pallet-evm-precompile-preimage package used in Tangle Network in terms of GPLv3.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
#![cfg_attr(not(feature = "std"), no_std)]

use fp_evm::{ExitRevert, PrecompileFailure, PrecompileHandle};
use frame_support::{
	dispatch::{GetDispatchInfo, PostDispatchInfo},
	traits::ConstU32,
};
use sp_runtime::traits::Dispatchable;
use frame_system::pallet_prelude::BlockNumberFor;
use pallet_evm::AddressMapping;
use pallet_btc_pools::Call as BtcPoolsCall;
use precompile_utils::{prelude::*, solidity::revert::revert_as_bytes};
use sp_core::H256;
use sp_std::{marker::PhantomData, vec::Vec};

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub const SUBMISSION_SIZE_LIMIT: u32 = 2u32.pow(16);
type GetSubmissionSizeLimit = ConstU32<SUBMISSION_SIZE_LIMIT>;

/// A precompile to wrap the functionality from pallet-preimage.
pub struct BtcPoolsPrecompile<Runtime>(PhantomData<Runtime>);

#[precompile_utils::precompile]
impl<Runtime> BtcPoolsPrecompile<Runtime>
where
	Runtime: pallet_btc_pools::Config + pallet_evm::Config + frame_system::Config,
	<Runtime as frame_system::Config>::Hash: TryFrom<H256> + Into<H256>,
	<Runtime as frame_system::Config>::RuntimeCall:
		Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
	<<Runtime as frame_system::Config>::RuntimeCall as Dispatchable>::RuntimeOrigin:
		From<Option<Runtime::AccountId>>,
	<Runtime as frame_system::Config>::Hash: Into<H256>,
	<Runtime as frame_system::Config>::RuntimeCall: From<BtcPoolsCall<Runtime>>,
	BlockNumberFor<Runtime>: From<u64>,
{
	#[precompile::public("registerBtcValidator(bytes32)")]
	fn register_btc_validator(
		handle: &mut impl PrecompileHandle,
        submission: BoundedBytes<GetSubmissionSizeLimit>,
	) -> EvmResult {
		// // Convert Ethereum address to Substrate account ID
		// let permitted_caller = Runtime::AddressMapping::into_account_id(submission.0);

		// let submission: Vec<u8> = submission.into();

		// let call = BtcPoolsCall::<Runtime>::register_btc_validator { submission };

		// // Dispatch the call using the RuntimeHelper
		// <RuntimeHelper<Runtime>>::try_dispatch(handle, Some(origin).into(), call)?;

		Ok(())
	}
}
