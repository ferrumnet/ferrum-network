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
use sp_core::H160;
use sp_runtime::traits::{Dispatchable, StaticLookup};
use frame_system::pallet_prelude::BlockNumberFor;
use pallet_evm::AddressMapping;
use pallet_quantum_portal::Call as QuantumPortalCall;
use sp_core::H256;
use sp_std::{marker::PhantomData, vec::Vec};
use precompile_utils::{prelude::*, solidity::revert::revert_as_bytes};

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub const SUBMISSION_SIZE_LIMIT: u32 = u32::MAX;
type GetSubmissionSizeLimit = ConstU32<SUBMISSION_SIZE_LIMIT>;

/// A precompile to wrap the functionality from pallet-preimage.
pub struct QuantumPortalPrecompile<Runtime>(PhantomData<Runtime>);

#[precompile_utils::precompile]
impl<Runtime> QuantumPortalPrecompile<Runtime>
where
	Runtime: pallet_quantum_portal::Config + pallet_evm::pallet::Config + frame_system::pallet::Config,
	<Runtime as frame_system::Config>::RuntimeCall:
		Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
	<<Runtime as frame_system::Config>::RuntimeCall as Dispatchable>::RuntimeOrigin:
		From<Option<<Runtime as frame_system::Config>::AccountId>>,
		<Runtime as frame_system::Config>::AccountId: Into<H160>,
		<Runtime as frame_system::Config>::RuntimeCall: From<QuantumPortalCall<Runtime>>,
{
	#[precompile::public("registerFinalizer(u64)")]
	fn register_finalizer(
		handle: &mut impl PrecompileHandle,
		chain_id: u64,
	) -> EvmResult {
		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);

		let call = QuantumPortalCall::<Runtime>::register_finalizer { finalizer : origin.clone(), chain_id };

		// Dispatch the call using the RuntimeHelper
		<RuntimeHelper<Runtime>>::try_dispatch(handle, Some(origin).into(), call.into(), 0)?;

		Ok(())
	}

	#[precompile::public("removeFinalizer(u64)")]
	fn remove_finalizer(
		handle: &mut impl PrecompileHandle,
		chain_id: u64,
	) -> EvmResult {
		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);

		let call = QuantumPortalCall::<Runtime>::remove_finalizer { finalizer : origin.clone(), chain_id };

		// Dispatch the call using the RuntimeHelper
		<RuntimeHelper<Runtime>>::try_dispatch(handle, Some(origin).into(), call.into(), 0)?;

		Ok(())
	}

	#[precompile::public("setThreshold(u64)")]
	fn set_threshold(
		handle: &mut impl PrecompileHandle,
		chain_id: u64,
		threshold: u32,
	) -> EvmResult {
		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);

		let call = QuantumPortalCall::<Runtime>::set_finalizer_threshold { chain_id, threshold };

		// Dispatch the call using the RuntimeHelper
		<RuntimeHelper<Runtime>>::try_dispatch(handle, Some(origin).into(), call.into(), 0)?;

		Ok(())
	}
}
