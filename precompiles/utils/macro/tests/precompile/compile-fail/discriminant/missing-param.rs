

use core::marker::PhantomData;

pub struct Precompile<R>(PhantomData<R>);

#[precompile_utils_macro::precompile]
#[precompile::precompile_set]
impl<R> Precompile<R> {
	#[precompile::discriminant]
	fn discriminant() -> Option<u64> {
		Some(42)
	}

	#[precompile::public("foo()")]
	fn foo(_discriminant: u32, test: &mut impl PrecompileHandle) -> EvmResult {
		todo!()
	}
}

fn main() { }