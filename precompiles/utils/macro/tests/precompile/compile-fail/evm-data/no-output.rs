

use core::marker::PhantomData;

pub struct Precompile<R>(PhantomData<R>);

#[precompile_utils_macro::precompile]
impl<R> Precompile<R> {
	#[precompile::public("foo()")]
	fn foo(test: &mut impl PrecompileHandle) {
		todo!()
	}
}

fn main() { }