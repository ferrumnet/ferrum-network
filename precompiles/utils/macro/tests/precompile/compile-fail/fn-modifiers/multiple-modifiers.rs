

use core::marker::PhantomData;

pub struct Precompile<R>(PhantomData<R>);

#[precompile_utils_macro::precompile]
impl<R> Precompile<R> {
	#[precompile::public("foo()")]
	#[precompile::view]
	#[precompile::payable]
	fn foo(_handle: &mut impl PrecompileHandle) -> EvmResult {
		Ok(())
	}
}

fn main() { }