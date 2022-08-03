use crate::*;
use frame_support::sp_runtime::traits::AtLeast32Bit;
use sp_arithmetic::traits::{CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, IntegerSquareRoot};
pub struct DexPricer;

const PRECISION: u32 = 1_000_000_000;

impl DexPricer {

	pub fn new_pool_function<T: IntegerSquareRoot + CheckedAdd + CheckedMul + CheckedDiv + From<u32>>(
		a: T,
		b: T,
	) -> Option<T> {
		let k = a.checked_mul(&b);
		match k {
			Some(k) => k.integer_sqrt_checked(),
			None => None,
		}
	}
	
	pub fn existing_pool_function<T: IntegerSquareRoot + CheckedAdd + CheckedMul + CheckedDiv + From<u32>>(
		wallet_a: T,
		pool_a: T,
		lp_minted: T,
	) -> Option<T> {
		let incr_wallet_a = wallet_a.checked_mul(&PRECISION.into());
		let share = incr_wallet_a?.checked_div(&pool_a);
		match share {
			Some(x) => x.checked_mul(&lp_minted)?.checked_div(&PRECISION.into()),
			None => None,
		}
	}

	pub fn liquidity_reward<T: IntegerSquareRoot + CheckedAdd + CheckedMul + CheckedDiv + From<u32>>(
		lp_tokens: T,
		lp_minted: T,
		pool: T,
	) -> Option<T> {
		let incr_lp_tokens = lp_tokens.checked_mul(&PRECISION.into());	
		let share = incr_lp_tokens?.checked_div(&lp_minted);
		match share {
			Some(x) => x.checked_mul(&pool)?.checked_div(&PRECISION.into()),
			None => return None,
		}
	}

	pub fn swap<T: IntegerSquareRoot + CheckedAdd + CheckedMul + CheckedDiv + From<u32>>(
		tokens: T,
		liquidity: (T, T),
	) -> Option<T> {
		let liquidity_ratio = liquidity.0.checked_mul(&PRECISION.into())?.checked_div(&liquidity.1);
		match liquidity_ratio {
			Some(liquidity_ratio) => tokens.checked_mul(&PRECISION.into())?.checked_div(&liquidity_ratio),
			None => None,
		}
	}

}