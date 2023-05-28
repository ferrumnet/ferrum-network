use crate::{NegativeImbalance, Runtime, RuntimeCall};
use frame_support::traits::{Contains, Currency, Imbalance, OnUnbalanced};

/// Logic for the author to get a portion of fees.
pub struct ToAuthor<R>(sp_std::marker::PhantomData<R>);
impl<R> OnUnbalanced<NegativeImbalance<R>> for ToAuthor<R>
where
    R: pallet_balances::Config + pallet_authorship::Config,
    <R as frame_system::Config>::RuntimeEvent: From<pallet_balances::Event<R>>,
{
    fn on_nonzero_unbalanced(amount: NegativeImbalance<R>) {
        if let Some(author) = <pallet_authorship::Pallet<R>>::author() {
            <pallet_balances::Pallet<R>>::resolve_creating(&author, amount);
        }
    }
}

pub struct DealWithFees<R>(sp_std::marker::PhantomData<R>);
impl<R> OnUnbalanced<NegativeImbalance<R>> for DealWithFees<R>
where
    R: pallet_balances::Config + pallet_authorship::Config,
    //pallet_treasury::Pallet<R>: OnUnbalanced<NegativeImbalance<R>>,
    <R as frame_system::Config>::RuntimeEvent: From<pallet_balances::Event<R>>,
{
    fn on_unbalanceds<B>(mut fees_then_tips: impl Iterator<Item = NegativeImbalance<R>>) {
        if let Some(fees) = fees_then_tips.next() {
            // for fees, 50% to treasury, 50% to author
            let mut split = fees.ration(50, 50);
            if let Some(tips) = fees_then_tips.next() {
                // for tips, if any, 100% to author
                tips.merge_into(&mut split.1);
            }

            //<pallet_treasury::Pallet<R> as OnUnbalanced<_>>::on_unbalanced(split.0);
            <ToAuthor<R> as OnUnbalanced<_>>::on_unbalanced(split.1);
        }
    }
}

pub struct FerrumCallFilter;
impl Contains<RuntimeCall> for FerrumCallFilter {
    fn contains(call: &RuntimeCall) -> bool {
        let is_core_call = matches!(
            call,
            RuntimeCall::System(_) | RuntimeCall::Timestamp(_) | RuntimeCall::ParachainSystem(_)
        );
        if is_core_call {
            // always allow core call
            return true;
        }

        let is_paused =
            pallet_transaction_pauser::PausedTransactionFilter::<Runtime>::contains(call);
        if is_paused {
            // no paused call
            return false;
        }

        if let RuntimeCall::PolkadotXcm(xcm_method) = call {
            match xcm_method {
                pallet_xcm::Call::send { .. }
                | pallet_xcm::Call::execute { .. }
                | pallet_xcm::Call::teleport_assets { .. }
                | pallet_xcm::Call::reserve_transfer_assets { .. }
                | pallet_xcm::Call::limited_reserve_transfer_assets { .. }
                | pallet_xcm::Call::limited_teleport_assets { .. } => {
                    return false;
                }
                pallet_xcm::Call::force_xcm_version { .. }
                | pallet_xcm::Call::force_default_xcm_version { .. }
                | pallet_xcm::Call::force_subscribe_version_notify { .. }
                | pallet_xcm::Call::force_unsubscribe_version_notify { .. } => {
                    return true;
                }
                pallet_xcm::Call::__Ignore { .. } => {
                    unimplemented!()
                }
            }
        }

        true
    }
}
