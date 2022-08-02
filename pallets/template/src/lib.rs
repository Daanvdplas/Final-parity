#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/v3/runtime/frame>
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
	use codec::{HasCompact, MaxEncodedLen};
	use frame_support::pallet_prelude::*;
	use frame_support::dispatch::EncodeLike;
	use frame_system::pallet_prelude::*;
	use frame_support::PalletId;
	use frame_support::Hashable;
	use scale_info::prelude::vec;
	use sp_runtime::traits::{AccountIdConversion, AtLeast32Bit};
	use frame_support::traits::tokens::fungibles::{Inspect, Transfer, Mutate};
	use sp_arithmetic::traits::{CheckedAdd, CheckedMul, CheckedDiv, IntegerSquareRoot}; 

	type TokenIdOf<T: Config> = <T::Tokens as Inspect<T::AccountId>>::AssetId;
	type BalanceOf<T: Config> = <T::Tokens as Inspect<T::AccountId>>::Balance;
	
	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type Tokens: Inspect<Self::AccountId> + Transfer<Self::AccountId> + Mutate<Self::AccountId>;
		type PalletId: Get<PalletId>;
		type MaxLiquidityProviders: Get<u32>;
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
	pub(super) type LiquidityProviders<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, BoundedVec<T::AccountId, T::MaxLiquidityProviders>, ValueQuery>;
	
	// #[pallet::storage]
	// pub(super) type LpMinted<T: Config> = StorageMap<_, Blake2_128Concat, TokenIdOf<T>, BalanceOf<T>>;

	// EVENTS
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		// LiquidityWithdrawn
		LiquidityWithdrawn {
			from: T::AccountId,
			to: T::AccountId,
			token_a: TokenIdOf<T>,
			token_b: TokenIdOf<T>,
			quantity_token_a: BalanceOf<T>,
			quantity_token_b: BalanceOf<T>,
		},
		// LiquidityDeposited
		LiquidityDeposited {
			from: T::AccountId,
			to: T::AccountId,
			token_a: TokenIdOf<T>,
			token_b: TokenIdOf<T>,
			quantity_token_a: BalanceOf<T>,
			quantity_token_b: BalanceOf<T>,
		},
		// SwapAccured
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
		/// An overflow in amount of LP tokens minted has occured.
		Overflow,
		/// To many liquidity providers which shouldn't be possible.
		TooManyLiqProviders,
		/// Can't provide liquidity with this token. 
		InvalidToken,
		/// Defensive error.
		DefensiveError,
		/// Wallet has not provided liquidity to this pool
		NoLiquidityProvided,

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
			// Check for other tokens than the allowed tokens to provide liquidity with
			Self::check_if_valid_tokens(token_a, token_b)?;
			// Check if wallet has enough funds
			Self::check_balances(&wallet, token_a, token_b, quantity_token_a, quantity_token_b)?;
			// Create token struct where token are ordered, the amounts are ordered to the tokens.
			// In addition, a tokenpair ID is created.
			let deposit = Self::create_deposit(token_a, token_b, quantity_token_a, quantity_token_b);
			// Check if pool already exists
			if let Ok(pool) = AllPools::<T>::try_get(&deposit.tokenpair_id) {
				// Deposit to existing pool
				Self::make_deposit(&deposit, &wallet, &pool, false);
			} else {
				// Create and deposit to new pool
				let pool_id: T::AccountId = T::PalletId::get().into_sub_account_truncating(&deposit.tokenpair_id);
				AllPools::<T>::insert(&deposit.tokenpair_id, pool_id.clone());
				Self::make_deposit(&deposit, &wallet, &pool_id, true);
			}
            // Self::deposit_event(Event::LiquidityDeposited {
			// 	from: *wallet,
			// 	to: *pool_id,
			// 	token_a: deposit.tokenpair[0],
			// 	token_b: deposit.tokenpair[1],
			// 	quantity_token_a: deposit.quantity_token_a,
			// 	quantity_token_b: deposit.quantity_token_b,
			// });
			Ok(())
		}

		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn withdraw_liquidity(
			origin: OriginFor<T>,
			token_a: TokenIdOf<T>,
			token_b: TokenIdOf<T>,
			lp_token: TokenIdOf<T>,
		) -> DispatchResult {

			// Check if extrinsic was signed
			let wallet = ensure_signed(origin)?;
			// Check if tokens are not the same
			ensure!(token_a != token_b, Error::<T>::IdenticalTokens);
			// Check for other tokens than the allowed tokens to provide liquidity with
			Self::check_if_valid_tokens(token_a, token_b)?;
			// Create token struct where token are ordered, the amounts are ordered to the tokens.
			// In addition, a tokenpair ID is created.
			let withdrawal = Self::create_withdrawal(token_a, token_b, lp_token);
			// Check if pool already exists
			if let Ok(pool) = AllPools::<T>::try_get(&withdrawal.tokenpair_id) {
				// Deposit to existing pool
				Self::check_if_liq_is_provided(&wallet, &pool)?;
				Self::make_withdrawal(&withdrawal, &wallet, &pool)?;
			} else {
				ensure!(false, Error::<T>::PoolNotFound);
			}

			Ok(())
		}
	}


	// FUNCTIONS
    impl<T: Config> Pallet<T>
		where TokenIdOf<T>: Ord + PartialOrd + AtLeast32Bit {
		fn check_balances(
			wallet: &T::AccountId,
			token_a: TokenIdOf<T>,
			token_b: TokenIdOf<T>,
			quantity_token_a: BalanceOf<T>,
			quantity_token_b: BalanceOf<T>,
		) -> DispatchResult {
			// Check if wallet has balance of token_a and token_b
			let valid_token_a = Self::check_balance(wallet, token_a, quantity_token_a);
			let valid_token_b = Self::check_balance(wallet, token_b, quantity_token_b);
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
			let balance = T::Tokens::balance(token, wallet);
			// Check for enough balance
			balance >= quantity_token
		}

		fn check_if_valid_tokens(token_a: TokenIdOf<T>, token_b: TokenIdOf<T>) -> DispatchResult {
			let dot: TokenIdOf<T> = 1u32.into();
			let eth: TokenIdOf<T> = 2u32.into();
			let ada: TokenIdOf<T> = 3u32.into();
			let btc: TokenIdOf<T> = 4u32.into();

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
			let mut tokenpair = vec![token_a, token_b];
			let cloned_tokenpair = tokenpair.clone();
			tokenpair.sort();
			let tokenpair_id = Self::create_token_pair_id(tokenpair[0], tokenpair[1]);
			let mut quantity_token_a = unsorted_quant_token_a;
			let mut quantity_token_b = unsorted_quant_token_b;
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
			let mut hash1 = token_a.blake2_128_concat();
			let mut hash2 = token_b.blake2_128_concat();
			hash1.append(&mut hash2);
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

		fn make_deposit(
			deposit: &Deposit<T>,
			wallet: &T::AccountId,
			pool_id: &T::AccountId,
			new_pool_bool: bool,
		) -> DispatchResult {
			if new_pool_bool {
				Self::deposit_to_new_pool(deposit, wallet, pool_id)?;	
			} else {
				Self::deposit_to_existing_pool(deposit, wallet, pool_id)?;
			}
			T::Tokens::transfer(
				deposit.tokenpair[0], 
				wallet,
				pool_id,
				deposit.quantity_token_a,
				true
			)?;
			T::Tokens::transfer(
				deposit.tokenpair[1], 
				wallet,
				pool_id,
				deposit.quantity_token_b,
				true
			)?;
			Ok(())
		}

		fn deposit_to_new_pool(deposit: &Deposit<T>, wallet: &T::AccountId, pool_id: &T::AccountId) -> DispatchResult {
			let lp_reward = DexPricer::new_pool_function(deposit.quantity_token_a, deposit.quantity_token_b);
			let maybe_value = u32::decode(&mut &*deposit.tokenpair_id.to_vec());
			if maybe_value.is_err() {
				return Err(sp_runtime::DispatchError::BadOrigin);
			}
			let value = maybe_value.expect("value checked to be 'Some'; eqd");
			let lp_token_id: TokenIdOf<T> = value.into();
			T::Tokens::mint_into(lp_token_id, wallet, lp_reward)?;
			T::Tokens::mint_into(lp_token_id, pool_id, lp_reward)?;
			LiquidityProviders::<T>::try_append(pool_id, wallet).map_err(|_| Error::<T>::TooManyLiqProviders)?;
			Ok(())
		}

		fn deposit_to_existing_pool(deposit: &Deposit<T>, wallet: &T::AccountId, pool_id: &T::AccountId) -> DispatchResult {
			let liq_providers = LiquidityProviders::<T>::get(pool_id);
			if let Some(_) = liq_providers.iter().position(|id| id == wallet) {
				();
			} else {
				LiquidityProviders::<T>::try_append(pool_id, wallet).map_err(|_| Error::<T>::TooManyLiqProviders)?;
			}
			let pool_amount = T::Tokens::balance(deposit.tokenpair[0], pool_id);
			let share = DexPricer::share_to(deposit.quantity_token_a, pool_amount);
			let maybe_value = u32::decode(&mut &*deposit.tokenpair_id.to_vec());
			if maybe_value.is_err() {
				return Err(sp_runtime::DispatchError::BadOrigin);
			}
			let value = maybe_value.expect("value checked to be 'Some'; eqd");
			let lp_token_id: TokenIdOf<T> = value.into();
			let lp_minted = T::Tokens::balance(lp_token_id, pool_id);
			let lp_reward = DexPricer::multiply_to(share, lp_minted);
			T::Tokens::mint_into(lp_token_id, wallet, lp_reward)?;
			T::Tokens::mint_into(lp_token_id, pool_id, lp_reward)?;
			Ok(())
		}

		fn check_if_liq_is_provided(wallet: &T::AccountId, pool_id: &T::AccountId) -> DispatchResult {
			let liq_providers = LiquidityProviders::<T>::get(pool_id);
			if let Some(_) = liq_providers.iter().position(|id| id == wallet) {
				return Ok(());
			} 
			ensure!(false, Error::<T>::NoLiquidityProvided);
			Ok(())
		}

		fn make_withdrawal(
			withdrawal: &Withdrawal<T>,
			wallet: &T::AccountId,
			pool_id: &T::AccountId,
		) -> DispatchResult {

			let quantity_token_a = T::Tokens::balance(withdrawal.tokenpair[0], pool_id);
			let quantity_token_b = T::Tokens::balance(withdrawal.tokenpair[1], pool_id);
			let maybe_value = u32::decode(&mut &*withdrawal.tokenpair_id.to_vec());
			if maybe_value.is_err() {
				return Err(sp_runtime::DispatchError::BadOrigin);
			}
			let value = maybe_value.expect("value checked to be 'Some'; eqd");
			let lp_token_id: TokenIdOf<T> = value.into();
			let lp_tokens = T::Tokens::balance(lp_token_id, wallet);
			let lp_minted = T::Tokens::balance(lp_token_id, pool_id);
			if lp_tokens > lp_minted {
				T::Tokens::burn_from(lp_token_id, pool_id, lp_tokens)?;
				Self::withdrawal_event(withdrawal, wallet, pool_id, quantity_token_a, quantity_token_b)?;
			} else {
				let share = DexPricer::share_to(lp_tokens, lp_minted);
				let token_a_reward = DexPricer::multiply_to(share, quantity_token_a);
				let token_b_reward = DexPricer::multiply_to(share, quantity_token_b);
				T::Tokens::burn_from(lp_token_id, pool_id, lp_tokens)?;
				Self::withdrawal_event(withdrawal, wallet, pool_id, token_a_reward, token_b_reward)?;
			}
			T::Tokens::burn_from(lp_token_id, wallet, lp_tokens)?;
			
			Ok(())
		}

		fn withdrawal_event(
			withdrawal: &Withdrawal<T>,
			wallet: &T::AccountId,
			pool_id: &T::AccountId,
			token_a_reward: BalanceOf<T>,
			token_b_reward: BalanceOf<T>,
		) -> DispatchResult {

			T::Tokens::transfer(
				withdrawal.tokenpair[0], 
				pool_id,
				wallet,
				token_a_reward,
				true
			);
			T::Tokens::transfer(
				withdrawal.tokenpair[1], 
				pool_id,
				wallet,
				token_b_reward,
				true
			);
			// Self::deposit_event(Event::LiquidityWithdrawn {
			// 	from: *pool_id,
			// 	to: *wallet,
			// 	token_a: withdrawal.tokenpair[0],
			// 	token_b: withdrawal.tokenpair[1],
			// 	quantity_token_a: token_a_reward,
			// 	quantity_token_b: token_b_reward,
			// });
			Ok(())
		}
	}
}