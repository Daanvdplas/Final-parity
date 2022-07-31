#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/v3/runtime/frame>
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
	use codec::{HasCompact, EncodeLike, MaxEncodedLen};
	use frame_support::pallet_prelude::*;
	use sp_std::str::from_utf8;
	use frame_system::pallet_prelude::*;
	use frame_support::PalletId;
	use frame_support::Hashable;
	use scale_info::prelude::vec;
	use sp_runtime::traits::{AccountIdConversion, AtLeast32Bit};
	use frame_support::traits::tokens::fungibles::{Inspect, Transfer};

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

		type Tokens: Inspect<Self::AccountId, AssetId = Self::AssetId> + Transfer<Self::AccountId, AssetId = Self::AssetId>;
		type PalletId: Get<PalletId>;
	}

	// pub struct TokenPair<T: Config> {
	// 	token_a: TokenIdOf<T>,
	// 	token_b: TokenIdOf<T>,
	// }

	// impl PartialEq for TokenPair<T> {
	// 	fn eq(&self, other: &Self) -> bool {
	// 		if self.token_a == other.token_a && self.token_b == other.token_b {
	// 			return true;
	// 		} else if self.token_b == other.token_a && self.token_a == other.token_b {
	// 			return true;
	// 		} else if self.token_a == other.token_b && self.token_b == other.token_a {
	// 			return true;
	// 		} else {
	// 			return false;
	// 		}
	// 	}
	// }

	// pub struct Pool<T: Config> {
	// 	pool_id: sp_runtime::AccountId32,
	// 	token_pair: TokenPair<T>,
	// 	token_a: (TokenIdOf<T>, BalanceOf<T>),
	// 	token_b: (TokenIdOf<T>, BalanceOf<T>),
	// }

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);
	

	// Keeps track of all the dex pools and their connected token pair.
	#[pallet::storage]
	pub(super) type AllPools<T: Config> = StorageMap<_, Twox64Concat, (TokenIdOf<T>, TokenIdOf<T>), T::AccountId, ValueQuery>;

	// Keeps track of all the dex pools an individual has provided liquidity to.
	// #[pallet::storage]
	// pub(super) type AccountToPools<T: Config> = StorageMap<
		// _,
		// Twox64Concat,
		// <T as frame_system::Config>::AccountId,
		// (TokenIdOf<T>, TokenIdOf<T>,
		// ValueQuery)
		// >;


	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/v3/runtime/events-and-errors
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event documentation should end with an array that provides descriptive names for event
		/// parameters. [something, who]
		SomethingStored(u32, T::AccountId),
	}

	// All the Errors that can occur while trying to swap, add liquidity and withdraw liquidity
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

	}

	#[pallet::call]
	impl<T: Config> Pallet<T> 
		where <T::Tokens as Inspect<T::AccountId>>::AssetId: AtLeast32Bit + Encode {
		/// Funtion to provide liquidity.
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn add_liquidity(
			origin: OriginFor<T>,
			token_a: TokenIdOf<T>,
			token_b: TokenIdOf<T>,
			token_a_amount: BalanceOf<T>,
			token_b_amount: BalanceOf<T>,
		) -> DispatchResult {
			
			// Check if extrinsic was signed
			let wallet = ensure_signed(origin)?;
			// Check if tokens are not the same and order deterministically
			ensure!(token_a != token_b, Error::<T>::IdenticalTokensError);

			// Self::check_balance(wallet, token_a, token_b, token_a_amount, token_b_amount)?;
			// if let Some(poolid) = Self::check_if_pool_exists(token_a, token_b) {
			// 	todo!("stake to poolid");
			// }
			// todo!("create new pool and poolid");

			// new_pool = Self::new_pool(String::from("pool"));


			Ok(())
		}
	}

		// Function to withdraw liquidity.
		// #[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		// pub fn withdraw_liquidity(
		// 	origin: OriginFor<T>,
		// 	token_a: AssetIdOf<T>,
		// 	token_b: AssetIdOf<T>,
		// 	rate: u32,
		// 	token_a_amount: BalanceOf<T>,
		// 	token_b_amount: BalanceOf<T>,
		// ) -> DispatchResult {
			
		// 	// Check if extrinsic was signed
		// 	let wallet = ensure_signed(origin)?;

		// 	Ok(())
		// }
	// To avoid cases of division by zero a minimum number of tokens have to exist in a pool.
	//pub const MINIMUM_LIQUIDITY = 1000u64;

	// Pallet internal functions
    impl<T: Config> Pallet<T> 
		where <T::Tokens as Inspect<T::AccountId>>::AssetId: Ord + PartialOrd {
		fn check_balance(
			wallet: <T as frame_system::Config>::AccountId,
			token_a: TokenIdOf<T>,
			token_b: TokenIdOf<T>,
			token_a_amount: BalanceOf<T>,
			token_b_amount: BalanceOf<T>,
		) -> DispatchResult {
			// Check if wallet has balance of token_a and token_b
			let balance_a = T::Tokens::balance(token_a, &wallet);
			let balance_b = T::Tokens::balance(token_b, &wallet);

			// Check if wallet has sufficient funds for balance transfer
			ensure!(balance_a >= token_a_amount || balance_b >= token_b_amount, Error::<T>::NotEnoughFunds);
			ensure!(balance_a >= token_a_amount, Error::<T>::NotEnoughFundsTokenA);
			ensure!(balance_b >= token_b_amount, Error::<T>::NotEnoughFundsTokenB);
			Ok(())
		}

		fn check_if_pool_exists(token_a: TokenIdOf<T>, token_b: TokenIdOf<T>) -> T::AccountId {
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

		// fn concatenate_hash_values(left: <T as pallet::Config>::Hashing, right: <T as pallet::Config>::Hashing) -> <T as Config>::Hashing {
		// 	let hash1: String = left.to_string();
		// 	let hash2: String = right.to_string();
		
		// 	let result = hash1 + &hash2;
		// 	<T as Config>::Hashing::hash(&result)
		// }
		//fn mint_lptoken();
		//fn burn_lptoken(); 
	}
}