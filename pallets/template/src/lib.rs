#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;
mod dex_pricer;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod help_test;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
	use crate::dex_pricer::{DexPricer};
	use codec::MaxEncodedLen;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use frame_support::{PalletId, Hashable};
	use crate::pallet::vec::Vec;
	use scale_info::prelude::vec;
	use sp_runtime::traits::{AccountIdConversion, AtLeast32Bit};
	use frame_support::traits::tokens::fungibles::{Inspect, Transfer, Mutate, Create};
	use frame_support::traits::tokens::currency::Currency;
	use sp_arithmetic::traits::{CheckedAdd, CheckedMul, CheckedDiv, IntegerSquareRoot}; 

	type TokenIdOf<T: Config> = <T::Tokens as Inspect<T::AccountId>>::AssetId;
	type BalanceOf<T: Config> = <T::Tokens as Inspect<T::AccountId>>::Balance;
	
	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type Tokens: Inspect<Self::AccountId>
		+ Transfer<Self::AccountId>
		+ Mutate<Self::AccountId>
		+ Create<Self::AccountId>;
		type Balances: Currency<Self::AccountId>;
		type PalletId: Get<PalletId>;
		type MaxLiqProviders: Get<u32>;	
	}

	#[derive(Encode, Decode, TypeInfo, DebugNoBound, CloneNoBound, EqNoBound, PartialEqNoBound)]
	#[scale_info(skip_type_params(T))]
	pub struct Deposit<T: crate::Config> {
		tokenpair: Vec<TokenIdOf<T>>,
		tokenpair_id: [u8; 16],
		quantity_token_a: BalanceOf<T>,
		quantity_token_b: BalanceOf<T>,
	}

	#[derive(Encode, Decode, TypeInfo, DebugNoBound, CloneNoBound, EqNoBound, PartialEqNoBound)]
	#[scale_info(skip_type_params(T))]
	pub struct Withdrawal<T: crate::Config> {
		tokenpair: Vec<TokenIdOf<T>>,
		tokenpair_id: [u8; 16],
		lp_token: TokenIdOf<T>,
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);
	
	// STORAGE
	#[pallet::storage]
	pub(super) type AllPools<T: Config> = StorageMap<_, Blake2_128Concat, [u8; 16], T::AccountId>;

	#[pallet::storage]
	pub(super) type LiquidityProviders<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, BoundedVec<T::AccountId, T::MaxLiqProviders>, ValueQuery>;

	// EVENTS
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		// LiquidityWithdrawn
		LiquidityWithdrawn {
			from: T::AccountId,
			to: T::AccountId,
			lp_token: TokenIdOf<T>,
		},
		// LiquidityDeposited
		LiquidityDeposited {
			from: T::AccountId,
			to: T::AccountId,
			token_a: TokenIdOf<T>,
			token_b: TokenIdOf<T>,
		},
		// SwapAccured
		SwapOccured {
			from: T::AccountId,
			to: T::AccountId,
		}
	}

	// ERROR
	#[pallet::error]
	pub enum Error<T> {
		/// When user wants to provide liquidity with identical tokens.
		IdenticalTokens,
		/// When user doesn't have enough funds for both tokens.
		NotEnoughFunds,
		/// When user doesn't have enough funds for token a.
		NotEnoughFundsTokenA,
		/// When user doesn't have enough funds for token b.
		NotEnoughFundsTokenB,
		/// For withdrawing liquidity from a pool that doesn't exist.
		PoolNotFound,
		/// To many liquidity providers which shouldn't be possible.
		LiqProvidersOverflow,
		/// Can't provide liquidity with this token. 
		InvalidToken,
		/// Defensive error.
		NoTokens,
		/// Wallet has not provided liquidity to this pool
		NoLiquidityProvided,
		/// Math problem
		MathProblem,
	}

	// HOOKS
	#[pallet::call]
	impl<T: Config> Pallet<T> 
		where TokenIdOf<T>: AtLeast32Bit + Encode + MaxEncodedLen + CheckedAdd + CheckedMul + CheckedDiv + IntegerSquareRoot {
		/// Funtion to provide liquidity.
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn deposit_liquidity(
			origin: OriginFor<T>,
			token_a: TokenIdOf<T>,
			token_b: TokenIdOf<T>,
			quantity_token_a: BalanceOf<T>,
			quantity_token_b: BalanceOf<T>,
		) -> DispatchResult {
			
			// Check if extrinsic was signed
			let wallet = ensure_signed(origin)?;

			// Check if tokens are not the same
			ensure!(token_a != token_b, Error::<T>::IdenticalTokens);

			// Check for other tokens than the allowed tokens to provide liquidity with (DOT, ETH, ADA, BTC)
			Self::check_if_valid_tokens(token_a, token_b)?;

			// Check if wallet has enough funds
			Self::check_balances(&wallet, token_a, token_b, quantity_token_a, quantity_token_b)?;

			// Create deposit struct where tokens are ordered, the amounts are ordered to the tokens.
			// In addition, a tokenpair ID is created.
			let deposit = Self::create_deposit(token_a, token_b, quantity_token_a, quantity_token_b);

			// Check if pool already exists
			if let Ok(pool) = AllPools::<T>::try_get(&deposit.tokenpair_id) {
				// Deposit to existing pool
				Self::deposit(deposit, wallet, pool, false)?;
			} else {
				// Create and deposit to new pool
				let pool_id: T::AccountId = T::PalletId::get().into_sub_account_truncating(&deposit.tokenpair_id);
				T::Balances::make_free_balance_be(&pool_id, 1_000u32.into());
				AllPools::<T>::insert(&deposit.tokenpair_id, pool_id.clone());
				Self::deposit(deposit, wallet, pool_id, true)?;
			}
			Ok(())
		}

		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn withdraw_liquidity(
			origin: OriginFor<T>,
			token_a: TokenIdOf<T>,
			token_b: TokenIdOf<T>,
			lp_token: TokenIdOf<T>,
		) -> DispatchResult {

			// Check if extrinsic was signed.
			let wallet = ensure_signed(origin)?;

			// Check if tokens are not the same.
			ensure!(token_a != token_b, Error::<T>::IdenticalTokens);

			// Check for other tokens than the allowed tokens to provide liquidity with.
			Self::check_if_valid_tokens(token_a, token_b)?;

			// Check if user has lp tokens.
			let lp_balance = T::Tokens::balance(lp_token, &wallet);
			if lp_balance == 0u32.into() {
				// If not throw error
				ensure!(false, Error::<T>::NoTokens);
			}

			// Create withdrawal struct where tokens are ordered, the amounts are ordered to the tokens.
			// In addition, a tokenpair ID is created.
			let withdrawal = Self::create_withdrawal(token_a, token_b, lp_token);

			// Check if pool already exists.
			if let Ok(pool) = AllPools::<T>::try_get(&withdrawal.tokenpair_id) {
				// Check if wallet has provided liquidity to this pool.
				Self::check_if_liq_is_provided(&wallet, &pool)?;

				// Deposit to existing pool.
				Self::withdraw(withdrawal, wallet, pool)?;
			} else {
				ensure!(false, Error::<T>::PoolNotFound);
			}
			Ok(())
		}
		
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn swap(
			origin: OriginFor<T>,
			from_token: TokenIdOf<T>,
			to_token: TokenIdOf<T>,
			swap_amount: BalanceOf<T>,
		) -> DispatchResult {
			// Check if extrinsic was signed.
			let wallet = ensure_signed(origin)?;

			// Check for other tokens than the allowed tokens to provide liquidity with.
			Self::check_if_valid_tokens(from_token, to_token)?;

			// Check is user has token balance
			let from_token_balance = T::Tokens::balance(from_token, &wallet);
			if from_token_balance < swap_amount {
				// If not throw error
				ensure!(false, Error::<T>::NotEnoughFunds);
			}

			let mut tokenpair = vec![from_token, to_token];
			tokenpair.sort();

			// Create token pair ID
			let token_pair_id = Self::create_token_pair_id(tokenpair[0], tokenpair[1]);

			// Check if pool already exists
			if let Ok(pool) = AllPools::<T>::try_get(&token_pair_id) {
				// Make swap
				Self::make_swap(wallet, pool, (from_token, to_token), swap_amount)?;	
			} else {
				// Pool does not exist yet
				ensure!(false, Error::<T>::PoolNotFound);
			}
			Ok(())
		}
	}

	// FUNCTIONS
    impl<T: Config> Pallet<T>
		where TokenIdOf<T>: Ord + PartialOrd + AtLeast32Bit + Copy {
		fn make_swap(
			wallet: T::AccountId,
			pool_id: T::AccountId,
			token_swap: (TokenIdOf<T>, TokenIdOf<T>),
			swap_amount: BalanceOf<T>,
		) -> DispatchResult {

			let pool_balance_a = T::Tokens::balance(token_swap.0, &wallet);
			let pool_balance_b = T::Tokens::balance(token_swap.1, &wallet);
			
			let mut swap_reward = 0u32.into();
			// Calculate swap
			match DexPricer::swap(swap_amount, (pool_balance_a, pool_balance_b)) {
				Some(x) => swap_reward = x,
				None => ensure!(false, Error::<T>::MathProblem),
			}
			
			// Transfer tokens from user's wallet to pool's wallet
			T::Tokens::transfer(
				token_swap.0, 
				&wallet,
				&pool_id,
				swap_amount,
				true
			)?;

			// Transfer tokens from pool's wallet to user's wallet
			T::Tokens::transfer(
				token_swap.1, 
				&pool_id,
				&wallet,
				swap_reward,
				true
			)?;

			// Swap succesful
			Self::deposit_event(Event::SwapOccured {
				from: wallet,
				to: pool_id,
			});
			Ok(())
		}

		fn check_balances(
			wallet: &T::AccountId,
			token_a: TokenIdOf<T>,
			token_b: TokenIdOf<T>,
			quantity_token_a: BalanceOf<T>,
			quantity_token_b: BalanceOf<T>,
		) -> DispatchResult {
			// Check if wallet has enough balance of token_a and token_b
			let valid_token_a = Self::check_balance(wallet, token_a, quantity_token_a);
			let valid_token_b = Self::check_balance(wallet, token_b, quantity_token_b);

			// Specify the error
			if !valid_token_a && !valid_token_b {
				ensure!(false, Error::<T>::NotEnoughFunds);	
			} else if !valid_token_a {
				ensure!(false, Error::<T>::NotEnoughFundsTokenA);
			} else if !valid_token_b {
				ensure!(false, Error::<T>::NotEnoughFundsTokenB);
			}
			Ok(())
		}

		fn check_balance(
			wallet: &T::AccountId,
			token: TokenIdOf<T>,
			quantity_token: BalanceOf<T>,	
		) -> bool {
			// Check balance
			let balance = T::Tokens::balance(token, &wallet);

			// compare with desired quantity
			balance >= quantity_token
		}

		fn check_if_valid_tokens(token_a: TokenIdOf<T>, token_b: TokenIdOf<T>) -> DispatchResult {
			// Create DOT, ETH, ADA, BTC
			let dot: TokenIdOf<T> = 1u32.into();
			let eth: TokenIdOf<T> = 2u32.into();
			let ada: TokenIdOf<T> = 3u32.into();
			let btc: TokenIdOf<T> = 4u32.into();

			// Check for other tokens than DOT, ETH, ADA, BTC
			ensure!(token_a == dot || token_a == eth || token_a == ada || token_a == btc, Error::<T>::InvalidToken);	
			ensure!(token_b == dot || token_b == eth || token_b == ada || token_b == btc, Error::<T>::InvalidToken);	
			Ok(())
		}

		fn create_deposit(
			token_a: TokenIdOf<T>,
			token_b: TokenIdOf<T>,
			unsorted_quant_token_a: BalanceOf<T>,
			unsorted_quant_token_b: BalanceOf<T>,
		) -> Deposit<T> {
			// Sort the token pair: this is to prevent an (ETH, BTC) and a (BTC, ETH) pool
			let mut tokenpair = vec![token_a, token_b];
			let cloned_tokenpair = tokenpair.clone();
			tokenpair.sort();

			// Create a token pair ID by using blake2
			let tokenpair_id = Self::create_token_pair_id(tokenpair[0], tokenpair[1]);
			let mut quantity_token_a = unsorted_quant_token_a;
			let mut quantity_token_b = unsorted_quant_token_b;

			// If tokenpair is sorted differently, the token amounts need to swap
			if tokenpair != cloned_tokenpair {
				quantity_token_a = unsorted_quant_token_b;
				quantity_token_b = unsorted_quant_token_a;
			}
			Deposit {
				tokenpair,
				tokenpair_id,
				quantity_token_a,
				quantity_token_b,
			}
		}

		fn create_token_pair_id(token_a: TokenIdOf<T>, token_b: TokenIdOf<T>) -> [u8; 16] {
			// Use blake2 to create a deterministic token pair ID
			// First hash both token ID's (u32)
			let mut hash1 = token_a.blake2_128_concat();
			let mut hash2 = token_b.blake2_128_concat();

			// Append both hashes
			hash1.append(&mut hash2);

			// Create final token pair ID (hash)
			let pool_id = hash1.blake2_128();
			pool_id
		}

		fn create_withdrawal(
			token_a: TokenIdOf<T>,
			token_b: TokenIdOf<T>,
			lp_token: TokenIdOf<T>,
		) -> Withdrawal<T> {
			let mut tokenpair = vec![token_a, token_b];
			tokenpair.sort();
			let tokenpair_id = Self::create_token_pair_id(tokenpair[0], tokenpair[1]);
			Withdrawal {
				tokenpair,
				tokenpair_id,
				lp_token,
			}
		}

		fn check_swap_token_a(from_token: TokenIdOf<T>, to_token: TokenIdOf<T>) -> (TokenIdOf<T>, TokenIdOf<T>, bool) {
			// Check for from_token in sorted manner
			let mut tokenpair = vec![from_token, to_token];
			tokenpair.sort();
			if tokenpair[0] == from_token {
				(from_token, to_token, true)
			} else {
				(to_token, from_token, false)
			}
		}

		fn check_if_liq_is_provided(wallet: &T::AccountId, pool_id: &T::AccountId) -> DispatchResult {
			let liq_providers = LiquidityProviders::<T>::get(pool_id);
			if let Some(_) = liq_providers.iter().position(|id| id == wallet) {
				return Ok(());
			} 
			ensure!(false, Error::<T>::NoLiquidityProvided);
			Ok(())
		}

		fn deposit(
			deposit: Deposit<T>,
			wallet: T::AccountId,
			pool_id: T::AccountId,
			new_pool_bool: bool,
		) -> DispatchResult {
			// Specified whether deposit is made to a new pool or already existing
			// (Matters for the calculation)
			if new_pool_bool {
				Self::deposit_to_new_pool(&deposit, &wallet, pool_id.clone())?;	
			} else {
				// Self::deposit_calculations(deposit, &wallet, pool)
				Self::deposit_to_existing_pool(&deposit, &wallet, &pool_id)?;
			}

			// Transfer tokens from user's wallet to pool's wallet
			T::Tokens::transfer(
				deposit.tokenpair[0], 
				&wallet,
				&pool_id,
				deposit.quantity_token_a,
				true
			)?;

			// Transfer tokens from user's wallet to pool's wallet
			T::Tokens::transfer(
				deposit.tokenpair[1], 
				&wallet,
				&pool_id,
				deposit.quantity_token_b,
				true
			)?;

			// Deposit succesful
			Self::deposit_event(Event::LiquidityDeposited {
				from: wallet,
				to: pool_id,
				token_a: deposit.tokenpair[0],
				token_b: deposit.tokenpair[1],
			});
			Ok(())
		}

		fn deposit_to_new_pool(deposit: &Deposit<T>, wallet: &T::AccountId, pool_id: T::AccountId) -> DispatchResult {
			let mut lp_reward = 0u32.into();
			// Calculate lp reward
			match DexPricer::new_pool_function(deposit.quantity_token_a, deposit.quantity_token_b) {
				Some(x) => lp_reward = x,
				None => ensure!(false, Error::<T>::MathProblem),
			}

			// A funny but not perfect way of creating a save lp token id by decoding the token pair id
			let maybe_value = u32::decode(&mut &*deposit.tokenpair_id.to_vec());
			if maybe_value.is_err() {
				return Err(sp_runtime::DispatchError::BadOrigin);
			}
			let value = maybe_value.expect("value checked to be 'Some'; eqd");
			let lp_token_id: TokenIdOf<T> = value.into();

			// Create a new LP token linked to the token pair ID
			T::Tokens::create(lp_token_id, pool_id.clone(), true, 1u32.into())?;

			// Mint token reward amount into user's wallet and into the pool's wallet
			// The latter is for keeping track of the amount of lp tokens minted
			T::Tokens::mint_into(lp_token_id, wallet, lp_reward)?;
			T::Tokens::mint_into(lp_token_id, &pool_id, lp_reward)?;

			// Making sure that a pool has no more than 4 liquidity providers (Only 4 users exist)
			LiquidityProviders::<T>::try_append(&pool_id, wallet).map_err(|_| Error::<T>::LiqProvidersOverflow)?;
			Ok(())
		}

		fn deposit_to_existing_pool(deposit: &Deposit<T>, wallet: &T::AccountId, pool_id: &T::AccountId) -> DispatchResult {
			// Function to check for no more than 4 liq providers.
			// Shouldn't be possible because there are only 4 users.
			// Wasn't really sure whether I needed it.
			Self::check_liq_providers_overflow(wallet, pool_id)?;

			// Obtain token a & token b amount from the pool
			let pool_amount_a = T::Tokens::balance(deposit.tokenpair[0], &pool_id);

			// Again, a funny but deterministic way of obtaining the lp token ID
			let maybe_value = u32::decode(&mut &*deposit.tokenpair_id.to_vec());
			if maybe_value.is_err() {
				return Err(sp_runtime::DispatchError::BadOrigin);
			}
			let value = maybe_value.expect("value checked to be 'Some'; eqd");
			let lp_token_id: TokenIdOf<T> = value.into();

			// Get amount of lp tokens given out already by the pool
			let lp_minted = T::Tokens::balance(lp_token_id, &pool_id);

			let mut lp_reward = 0u32.into();
			// Calculate lp reward
			match DexPricer::existing_pool_function(deposit.quantity_token_a, pool_amount_a, lp_minted) {
				Some(x) => lp_reward = x,
				None => ensure!(false, Error::<T>::MathProblem),
			}

			// Give wallet lp reward as well as updating the total amount of lp tokens given out (by minting the token)
			T::Tokens::mint_into(lp_token_id, &wallet, lp_reward)?;
			T::Tokens::mint_into(lp_token_id, &pool_id, lp_reward)?;
			Ok(())
		}

		fn check_liq_providers_overflow(wallet: &T::AccountId, pool_id: &T::AccountId) -> DispatchResult {
			// Get all liq providers' wallets addresses
			let liq_providers = LiquidityProviders::<T>::get(pool_id);

			// See if wallet already provided liquidity to this pool
			if let Some(_) = liq_providers.iter().position(|id| id == wallet) {
				// If so, continue
				();
			} else {
				// If not, add wallet to the list 
				LiquidityProviders::<T>::try_append(pool_id, wallet).map_err(|_| Error::<T>::LiqProvidersOverflow)?;
			}
			Ok(())
		}

		fn withdraw(
			withdrawal: Withdrawal<T>,
			wallet: T::AccountId,
			pool_id: T::AccountId,
		) -> DispatchResult {
			// Get total amount of liquidity provided for token a and token b
			let quantity_token_a = T::Tokens::balance(withdrawal.tokenpair[0], &pool_id);
			let quantity_token_b = T::Tokens::balance(withdrawal.tokenpair[1], &pool_id);

			// Again, a funny but deterministic way of obtaining the lp token ID
			let maybe_value = u32::decode(&mut &*withdrawal.tokenpair_id.to_vec());
			if maybe_value.is_err() {
				return Err(sp_runtime::DispatchError::BadOrigin);
			}
			let value = maybe_value.expect("value checked to be 'Some'; eqd");
			let lp_token_id: TokenIdOf<T> = value.into();

			// Get amount of lp tokens in wallet
			let lp_tokens = T::Tokens::balance(lp_token_id, &wallet);

			// Get amount of lp tokens given out by the pool
			let lp_minted = T::Tokens::balance(lp_token_id, &pool_id);

			// If more lp tokens than lp minted my calculations were not precise enough and total pool is for wallet.
			// Would have done this differently if I had tested the math more properly
			if lp_tokens >= lp_minted {
				// Update tokens given out by pool and burn tokens from wallet
				T::Tokens::burn_from(lp_token_id, &pool_id, lp_tokens)?;
				T::Tokens::burn_from(lp_token_id, &wallet, lp_tokens)?;

				// Withdrawal succesful
				Self::withdrawal_event(&withdrawal, &wallet, &pool_id, quantity_token_a, quantity_token_b)?;
			} else {
				
				// Setting reward variables so I have them in this scope
				let mut liq_reward_a = 0u32.into();
				let mut liq_reward_b = 0u32.into();
				
				// Calculating the liquidity rewards 
				// Calculate lp reward a
				match DexPricer::liquidity_reward(lp_tokens, lp_minted, quantity_token_a) {
					Some(x) => liq_reward_a = x,
					None => ensure!(false, Error::<T>::MathProblem),
				}
				// Calculate lp reward b
				match DexPricer::liquidity_reward(lp_tokens, lp_minted, quantity_token_b) {
					Some(x) => liq_reward_b = x,
					None => ensure!(false, Error::<T>::MathProblem),
				}
				
				// Update tokens given out by pool and burn tokens from wallet
				T::Tokens::burn_from(lp_token_id, &pool_id, lp_tokens)?;
				T::Tokens::burn_from(lp_token_id, &wallet, lp_tokens)?;

				// Make transfers
				Self::withdrawal_event(&withdrawal, &wallet, &pool_id, liq_reward_a, liq_reward_b)?;
			}

			// Withdrawal succesful
			Self::deposit_event(Event::LiquidityWithdrawn {
				from: wallet,
				to: pool_id,
				lp_token: lp_token_id,
			});
			Ok(())
		}

		fn withdrawal_event(
			withdrawal: &Withdrawal<T>,
			wallet: &T::AccountId,
			pool_id: &T::AccountId,
			token_a_reward: BalanceOf<T>,
			token_b_reward: BalanceOf<T>,
		) -> DispatchResult {

			// Transfer tokens from pool's wallet to user's wallet
			T::Tokens::transfer(
				withdrawal.tokenpair[0], 
				pool_id,
				wallet,
				token_a_reward,
				true
			)?;

			// Transfer tokens from pool's wallet to user's wallet
			T::Tokens::transfer(
				withdrawal.tokenpair[1], 
				pool_id,
				wallet,
				token_b_reward,
				true
			)?;
			Ok(())
		}
	}
}