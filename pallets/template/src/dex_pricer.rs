use sp_arithmetic::traits::{CheckedAdd, CheckedMul, CheckedDiv, IntegerSquareRoot};
use frame_support::*;

pub struct DexPricer;

const PRECISION: u32 = 1_000_000;

impl DexPricer {

	// pub fn new_pool_function<t: integersquareroot + checkedadd + checkedmul + checkeddiv>(
	// 	a: t,
	// 	b: t,
	// ) -> result<t, _> {
	// 	let maybe_value = a.checked_mul(&b).unwrap();
	// 	if maybe_value.is_err() {
	// 		return err(_);
	// 	}	
	// 	let constant_k = maybe_value.expect("value checked to be 'some'; eqd");
	// 	constant_k.integer_sqrt()
	// }

	pub fn new_pool_function<T: IntegerSquareRoot + CheckedAdd + CheckedMul + CheckedDiv>(
		a: T,
		b: T,
	) -> T {
		let constant_k = a.checked_mul(&b).unwrap();
		constant_k.integer_sqrt()
	}
	
	pub fn share_to<T: IntegerSquareRoot + CheckedAdd + CheckedMul<Output = T> + From<u32> + CheckedDiv>(
		a: T,
		b: T,
	) -> T {
		let incr_a = a.checked_mul(&PRECISION.into()).unwrap();
		let share = incr_a.checked_div(&b).unwrap();
		share
	}

	pub fn multiply_to<T: IntegerSquareRoot + CheckedAdd + CheckedMul + From<u32> + CheckedDiv<Output = T>>(
		share: T,
		lp_minted: T,
	) -> T {
		let almost_lp_reward = share.checked_mul(&lp_minted).unwrap();
		let lp_reward = almost_lp_reward.checked_div(&PRECISION.into()).unwrap();	
		lp_reward
	}
}