

use {
	core::marker::PhantomData,
	frame_support::pallet_prelude::Get,
	precompile_utils::prelude::*,
};

pub struct Precompile<R>(PhantomData<R>);

#[precompile_utils_macro::precompile]
impl<R: Get<u32>> Precompile<R> {
	#[precompile::public("foo(bytes)")]
	fn foo(handle: &mut impl PrecompileHandle, arg: BoundedBytes<R>) -> EvmResult {
		Ok(())
	}
}

fn main() { }