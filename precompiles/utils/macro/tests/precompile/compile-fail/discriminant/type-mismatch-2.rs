

use core::marker::PhantomData;

pub struct Precompile<R>(PhantomData<R>);

#[precompile_utils_macro::precompile]
#[precompile::precompile_set]
impl<R> Precompile<R> {
	#[precompile::public("foo()")]
	fn foo(_discriminant: u32, test: &mut impl PrecompileHandle) -> EvmResult {
		todo!()
	}

	#[precompile::discriminant]
	fn discriminant(address: H160) -> Option<u64> {
		Some(42)
	}
}

fn main() { }