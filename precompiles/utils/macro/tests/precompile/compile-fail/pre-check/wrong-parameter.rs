

use core::marker::PhantomData;

pub struct Precompile<R>(PhantomData<R>);

#[precompile_utils_macro::precompile]
impl<R> Precompile<R> {
	#[precompile::pre_check]
	fn pre_check(_: u32) {
		todo!()
	}

	#[precompile::public("foo()")]
	fn foo(_handle: &mut impl PrecompileHandle) {
		todo!()
	}
}

fn main() { }