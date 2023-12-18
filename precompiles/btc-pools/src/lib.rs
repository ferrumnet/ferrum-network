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
#![cfg_attr(not(feature = "std"), no_std)]

use fp_evm::{ExitRevert, PrecompileFailure, PrecompileHandle};
use frame_support::{
    dispatch::{GetDispatchInfo, PostDispatchInfo},
    traits::ConstU32,
};
use frame_system::pallet_prelude::BlockNumberFor;
use pallet_btc_pools::Call as BtcPoolsCall;
use pallet_evm::AddressMapping;
use precompile_utils::{prelude::*, solidity::revert::revert_as_bytes};
use sp_core::H256;
use sp_runtime::traits::Dispatchable;
use sp_std::{marker::PhantomData, vec::Vec};

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
