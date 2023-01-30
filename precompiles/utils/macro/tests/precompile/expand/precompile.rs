

use {
	core::marker::PhantomData,
	precompile_utils::{EvmResult, prelude::*},
	sp_core::{H160, U256},
	frame_support::pallet_prelude::{Get, ConstU32},
};

// Based on Batch with stripped code.

struct BatchPrecompile<Runtime>(PhantomData<Runtime>);

type GetCallDataLimit = ConstU32<42>;
type GetArrayLimit = ConstU32<42>;


#[precompile_utils_macro::precompile]
impl<Runtime> BatchPrecompile<Runtime>
where
	Runtime: Get<u32>,
{
	#[precompile::pre_check]
	fn pre_check(handle: &mut impl PrecompileHandle) -> EvmResult {
		todo!("pre_check")
	}

	#[precompile::public("batchSome(address[],uint256[],bytes[],uint64[])")]
	fn batch_some(
		handle: &mut impl PrecompileHandle,
		to: BoundedVec<Address, GetArrayLimit>,
		value: BoundedVec<U256, GetArrayLimit>,
		call_data: BoundedVec<BoundedBytes<GetCallDataLimit>, GetArrayLimit>,
		gas_limit: BoundedVec<u64, GetArrayLimit>,
	) -> EvmResult {
		todo!("batch_some")
	}

	#[precompile::public("batchSomeUntilFailure(address[],uint256[],bytes[],uint64[])")]
	fn batch_some_until_failure(
		handle: &mut impl PrecompileHandle,
		to: BoundedVec<Address, GetArrayLimit>,
		value: BoundedVec<U256, GetArrayLimit>,
		call_data: BoundedVec<BoundedBytes<GetCallDataLimit>, GetArrayLimit>,
		gas_limit: BoundedVec<u64, GetArrayLimit>,
	) -> EvmResult {
		todo!("batch_some_until_failure")
	}

	#[precompile::public("batchAll(address[],uint256[],bytes[],uint64[])")]
	fn batch_all(
		handle: &mut impl PrecompileHandle,
		to: BoundedVec<Address, GetArrayLimit>,
		value: BoundedVec<U256, GetArrayLimit>,
		call_data: BoundedVec<BoundedBytes<GetCallDataLimit>, GetArrayLimit>,
		gas_limit: BoundedVec<u64, GetArrayLimit>,
	) -> EvmResult {
		todo!("batch_all")
	}

	// additional function to check fallback
	#[precompile::fallback]
	fn fallback(
		handle: &mut impl PrecompileHandle,
	) -> EvmResult {
		todo!("fallback")
	}
}