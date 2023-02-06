

use core::marker::PhantomData;

pub struct Precompile<R>(PhantomData<R>);

#[precompile_utils_macro::precompile]
#[precompile::precompile_set]
impl<R> Precompile<R> {
	#[precompile::public("foo()")]
	fn foo(_discriminant: u32, _handle: u32) {
		todo!()
	}
}

fn main() { }