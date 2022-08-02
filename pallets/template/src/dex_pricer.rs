use sp_runtime::traits::IntegerSquareRoot;
use sp_std::ops::{Mul, Div};

pub struct DexPricer;

const PRECISION: u128 = 1_000_000;

impl DexPricer {
	pub fn new_pool_function<T: IntegerSquareRoot + Mul<Output = T>>(
		a: T,
		b: T,
	) -> T {
		let constant_k = a * b;
		constant_k.integer_sqrt()
	}

	pub fn share_to<T: IntegerSquareRoot + Div<Output = T> + Mul<Output = T>>(
		a: T,
		b: T,
	) -> T {
		// todo! * PRECISION
		let incr_a = a;
		let share = incr_a / b;
		share
	}

	pub fn multiply_to<T: IntegerSquareRoot + Div<Output = T> + Mul<Output = T>>(
		share: T,
		lp_minted: T,
	) -> T {
		let almost_lp_reward = share * lp_minted;
		// todo! / PRECISION
		let lp_reward = almost_lp_reward;	
		lp_reward
	}
}