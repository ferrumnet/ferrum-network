

use core::marker::PhantomData;
use fp_evm::PrecompileHandle;
use precompile_utils::EvmResult;

pub struct Precompile<R>(PhantomData<R>);

#[precompile_utils_macro::precompile]
impl<R> Precompile<R> {
	#[precompile::public("foo()")]
	fn foo(test: &mut impl PrecompileHandle) -> EvmResult<String> {
		todo!()
	}
}

fn main() { }