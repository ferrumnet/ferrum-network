

use core::marker::PhantomData;

pub struct PrecompileSet<R>(PhantomData<R>);

#[precompile_utils_macro::precompile]
#[precompile::precompile_set]
impl<R> PrecompileSet<R> {
	#[precompile::discriminant]
	#[precompile::view]
	fn foo(address: H160) -> Option<u32> {
		None
	}
}

fn main() { }