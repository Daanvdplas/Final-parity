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

	type TokenIdOf<T: Config> = <T::Tokens as Inspect<T::AccountId>>::AssetId;
	type BalanceOf<T: Config> = <T::Tokens as Inspect<T::AccountId>>::Balance;
	
	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type AssetId: Member
		+ Parameter
		+ Default
		+ Copy
		+ HasCompact
		+ MaybeSerializeDeserialize
		+ MaxEncodedLen
		+ TypeInfo 
		+ EncodeLike;

		type Tokens: Inspect<Self::AccountId, AssetId = Self::AssetId>
		 + Transfer<Self::AccountId, AssetId = Self::AssetId>
		 + Mutate<Self::AccountId, AssetId = Self::AssetId>;

		type PalletId: Get<PalletId>;
		type MaxLiquidityProviders: Get<u32>;
		type LpToken: Get<Self::Tokens>;
		type MaxBalance: Get<u128>;
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
	
	#[pallet::storage]
	pub(super) type LpMinted<T: Config> = StorageMap<_, Blake2_128Concat, TokenIdOf<T>, BalanceOf<T>>;

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
			quantity_lp_token: TokenIdOf<T>,
		},
		// SwapAccured
	}

	// ERROR
	#[pallet::error]
	pub enum Error<T> {
		/// When user wants to provide liquidity with identical tokens
		IdenticalTokensError,
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
		/// To many liquidity providers which shouldn't be possible
		TooManyLiqProviders,

	}

	// HOOKS
	#[pallet::call]
	impl<T: Config> Pallet<T> 
		where TokenIdOf<T>: AtLeast32Bit + Encode + MaxEncodedLen{
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
			ensure!(token_a != token_b, Error::<T>::IdenticalTokensError);
			// todo! check if lp token is not in tokenpair
			// Check if wallet has enough funds
			Self::check_balances(&wallet, token_a, token_b, quantity_token_a, quantity_token_b)?;
			// Create token struct where token are ordered, the amounts are ordered to the tokens.
			// In addition, a tokenpair ID is created.
			let deposit = Self::create_deposit(token_a, token_b, quantity_token_a, quantity_token_b);
			// Check if pool already exists
			if let Some(pool) = AllPools::<T>::get(&deposit.tokenpair_id) {
				// Deposit to existing pool
				Self::make_deposit(&deposit, &wallet, &pool, false);
			} else {
				// Create and deposit to new pool
				let pool_id: T::AccountId = T::PalletId::get().into_sub_account_truncating(&deposit.tokenpair_id);
				AllPools::<T>::insert(&deposit.tokenpair_id, pool_id.clone());
				Self::make_deposit(&deposit, &wallet, &pool_id, true);
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

			// Check if extrinsic was signed
			let wallet = ensure_signed(origin)?;
			// Check if tokens are not the same
			ensure!(token_a != token_b, Error::<T>::IdenticalTokensError);
			// todo! check if user has stake to this pool
			// check for lp token as tokenpair
			// Create token struct where token are ordered, the amounts are ordered to the tokens.
			// In addition, a tokenpair ID is created.
			let withdrawal = Self::create_withdrawal(token_a, token_b, lp_token);
			// Check if pool already exists
			if let Ok(pool) = AllPools::<T>::try_get(&withdrawal.tokenpair_id) {
				// Deposit to existing pool
				Self::make_withdrawal(&withdrawal, &wallet, &pool);
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
			let balance_a = T::Tokens::balance(token_a, wallet);
			let balance_b = T::Tokens::balance(token_b, wallet);

			// Check if wallet has sufficient funds for balance transfer
			ensure!(balance_a >= quantity_token_a || balance_b >= quantity_token_b, Error::<T>::NotEnoughFunds);
			ensure!(balance_a >= quantity_token_a, Error::<T>::NotEnoughFundsTokenA);
			ensure!(balance_b >= quantity_token_b, Error::<T>::NotEnoughFundsTokenB);
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
			let tokenpair_id = Self::create_token_pair_id(token_a, token_b);
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

		fn create_token_pair_id(token_a: TokenIdOf<T>, token_b: TokenIdOf<T>) -> [u8; 16] {
			let mut hash1 = token_a.blake2_128_concat();
			let mut hash2 = token_b.blake2_128_concat();
			hash1.append(&mut hash2);
			let pool_id = hash1.blake2_128();
			pool_id
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
			);
			T::Tokens::transfer(
				deposit.tokenpair[1], 
				wallet,
				pool_id,
				deposit.quantity_token_b,
				true
			);
            // Self::deposit_event(Event::LiquidityDeposited {
			// 	from: wallet,
			// 	to: pool_id,
			// 	token_a: deposit.tokenpair[0],
			// 	token_b: deposit.tokenpair[1],
			// 	quantity_token_a: deposit.quantity_token_a,
			// 	quantity_token_b: deposit.quantity_token_b,
			// });
			Ok(())
		}

		fn deposit_to_new_pool(deposit: &Deposit<T>, wallet: &T::AccountId, pool_id: &T::AccountId) -> DispatchResult {
			let lp_reward = DexPricer::new_pool_function(deposit.quantity_token_a, deposit.quantity_token_b);
			// todo! create unique lp_token
			let lp_token_id: TokenIdOf<T> = 1u32.into();
			T::Tokens::mint_into(lp_token_id, wallet, lp_reward);
			LpMinted::<T>::insert(lp_token_id, lp_reward);
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
			let lp_token_id: TokenIdOf<T> = 1u32.into();
			let lp_minted = LpMinted::<T>::get(lp_token_id).unwrap(); 
			let lp_reward = DexPricer::multiply_to(share, lp_minted);
			LpMinted::<T>::insert(lp_token_id, lp_minted + lp_reward);
			Ok(())
		}

		fn make_withdrawal(
			withdrawal: &Withdrawal<T>,
			wallet: &T::AccountId,
			pool_id: &T::AccountId,
		) -> DispatchResult {
			let quantity_token_a = T::Tokens::balance(withdrawal.tokenpair[0], pool_id);
			let quantity_token_b = T::Tokens::balance(withdrawal.tokenpair[1], pool_id);
			let lp_token_id: TokenIdOf<T> = 1u32.into();
			let lp_tokens = T::Tokens::balance(lp_token_id, wallet);
			let lp_minted = LpMinted::<T>::get(lp_token_id).unwrap(); 
			let share = DexPricer::share_to(lp_tokens, lp_minted);
			let token_a_reward = DexPricer::multiply_to(share, quantity_token_a);
			let token_b_reward = DexPricer::multiply_to(share, quantity_token_b);
			T::Tokens::burn_from(lp_token_id, wallet, lp_tokens);
			LpMinted::<T>::insert(lp_token_id, lp_minted - lp_tokens);
			
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
			// Self::withdrawn_event(Event::LiquidityDeposited {
			// 	from: pool_id,
			// 	to: wallet,
			// 	token_a: withdrawn.tokenpair[0],
			// 	token_b: withdrawn.tokenpair[1],
			// 	quantity_token_a: token_a_reward,
			// 	quantity_token_b: token_b_reward,
			// });
			Ok(())
		}
	}
}