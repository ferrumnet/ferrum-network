

use core::marker::PhantomData;

pub struct Precompile<R>(PhantomData<R>);

#[precompile_utils_macro::precompile]
impl<R> Precompile<R> {
	#[precompile::pre_check]
	#[precompile::view]
	fn foo(handle: &mut impl PrecompileHandle) -> EvmResult {
		Ok(())
	}
}

fn main() { }