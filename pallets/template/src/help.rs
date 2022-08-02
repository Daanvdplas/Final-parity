#[pallet::storage]
	pub(super) type AllPools<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, (u32, u32), ValueQuery>;

pub fn add_liquidity(
	origin: OriginFor<T>,
	token_a: u32,
	token_b: u32,
	token_a_amount: u128,
	token_b_amount: u128,
) -> DispatchResult {
	
	// Check if extrinsic was signed
	let wallet = ensure_signed(origin)?;
	// Check if tokens are not the same
	ensure!(token_a != token_b, Error::<T>::IdenticalTokensError);

	// Self::check_balance(wallet, token_a, token_b, token_a_amount, token_b_amount)?;
	// if let Some(poolid) = Self::check_if_pool_exists(token_a, token_b) {
	// 	todo!("stake to poolid");
	// }
	// todo!("create new pool and poolid");

	// new_pool = Self::new_pool(String::from("pool"));


	Ok(())
}

fn check_if_pool_exists(token_a: u32, token_b: u32) -> T::AccountId {
	let mut vec = vec![token_a, token_b];
	vec.sort();
	let tokenpair = (vec[0], vec[1]);
	if let Some(poolid) = AllPools::<T>::get(&tokenpair) {
		return poolid;
	}
	let mut hashed_tokena = vec[0].twox_64_concat();
	let mut hashed_tokenb = vec[1].twox_64_concat();
	hashed_tokena.append(&mut hashed_tokenb);
	let new_pool_id = sp_std::str::from_utf8(&hashed_tokena).unwrap();
	T::PalletId::get().into_sub_account_truncating(new_pool_id)
}

fn help_deposit_liq(tokenpair, wallet, poolid, lp_token, flag) -> DispatchResult {
	let mut lp_reward = 0;
	let token_a_amount_user = balance(tokenpair.0, wallet);	
	if flag {
		let token_a_amount_user = balance(tokenpair.1, wallet);
		lp_reward = sqrt(token_a_amount_user, token_b_amount_user);
	} else {
		let token_a_amount_pool = balance(tokenpair.0, poolid);
		let share_token_a = token_a_amount_user / token_a_amount_pool;
		let lp_minted = balance(lp_token, poolid);
		let lp_reward = share_token_a * lp_minted; 
	}
	transfer()
	Ok(())
}

fn help_withdraw_liq(tokenpair, wallet, lp_token) -> DispatchResult {
	let poolid = AllPools::<T>::get(&tokenpair).ok_er(Error);
	let lp_token_minted = LpTokenPool::<T>::get(&poolid).ok_er(Error);
	amount_of_lp_tokens_user = balance(lp_token, wallet).ok_er(Error);
	// calculate share of wallet to pool
	let share = amount_of_lp_tokens_user / lp_token_minted;
	// calculate all tokens a
	amount_of_token_a = balance(tokenpair.0, poolid).ok_er(Error);
	share_token_a = amount_of_token_a * share;
	transfer(tokenpair.0, poolid, wallet, share_token_a).ok_er(Error);
	// calculate all tokens b
	amount_of_token_b = balance(tokenpair.1, poolid).ok_er(Error);
	share_token_b = amount_of_token_b * share;
	transfer(tokenpair.1, poolid, wallet, share_token_b).ok_er(Error);
	transfer(lp_token, wallet, poolid, amount_of_lp_tokens_user).ok_er(Error);
	destroy(lp_token, poolid, true, amount_of_lp_tokens_user).ok_er(Error);
	Ok(())
}


				let hashed_tokenpair = deposit.tokenpair_id.blake2_128();
				let decoded = decode(hashed_tokenpair);
				let tmp: TokenIdOf<T> = decoded.into();
				// let new_minted_lp = minted_lp.checked_add(lp_reward.into()).ok_or(Error::<T>::Overflow)?;





				pub struct Deposit<T: crate::Config> {
					tokenpair: Vec<TokenIdOf<T>>,
					tokenpair_id: [u8; 16],
					quantity_token_a: BalanceOf<T>,
					quantity_token_b: BalanceOf<T>,
				}

				pub struct Withdrawal<T: crate::Config> {
					tokenpair: Vec<TokenIdOf<T>>,
					tokenpair_id: [u8; 16],
					lp_token: TokenIdOf<T>,
				}