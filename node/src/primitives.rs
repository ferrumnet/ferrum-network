/// Common primitives for runtime

/// An index to a block.
pub type BlockNumber = u32;

/// Header type.
pub type Header = sp_runtime::generic::Header<BlockNumber, sp_runtime::traits::BlakeTwo256>;

/// Block type.
pub type Block = sp_runtime::generic::Block<Header, sp_runtime::OpaqueExtrinsic>;

/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;

/// Balance of an account.
pub type Balance = u128;

/// Some way of identifying an account on the chain.
pub type AccountId = ferrum_primitives::AccountId20;
