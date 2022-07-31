use crate::{mock::*, Error};
use frame_support::traits::Currency;
use frame_support::traits::fungibles::Mutate;
use frame_support::{assert_noop, assert_ok};
// use pallet_assets::pallet::Pallet as AssetsPallet;
use frame_support::traits::tokens::fungibles::Create;
const USER: AccountId = 1;
const ASSET1: u32 = 1;
const ASSET2: u32 = 2;
const ASSET1_AMOUNT: u128 = 1;
const ASSET2_AMOUNT: u128 = 1;
const MINT: u128 = 1_000_000_000_000_000_000_000u128;

#[test]
fn provide_liquidity_without_tokens() {
    new_test_ext().execute_with(|| {
		assert_noop!(TemplateModule::add_liquidity(Origin::signed(USER), ASSET1, ASSET2, ASSET1_AMOUNT, ASSET2_AMOUNT), Error::<Test>::NotEnoughFunds);
    });
}

#[test]
fn provide_liquidity_with_enough_funds() {
    new_test_ext().execute_with(|| {
		let origin = Origin::signed(USER);
		// Create balance for user
		Balances::make_free_balance_be(&USER, MINT);
		// Create token_a
        Assets::create(origin, ASSET1, USER, 1);
		// Mint token_a
		Assets::mint_into(ASSET1, &USER, MINT);

		let origin = Origin::signed(USER);
		// Create balance for user
		Balances::make_free_balance_be(&USER, MINT);
		// Create token_a
        Assets::create(origin, ASSET2, USER, 1);
		// Mint token_a
		Assets::mint_into(ASSET2, &USER, MINT);
        assert_ok!(TemplateModule::add_liquidity(Origin::signed(USER), ASSET1, ASSET2, ASSET1_AMOUNT, ASSET2_AMOUNT));
    });
}

#[test]
fn provide_liquidity_with_too_little_of_tokens_b() {
    new_test_ext().execute_with(|| {
		let origin = Origin::signed(USER);
		Balances::make_free_balance_be(&USER, MINT);
        Assets::create(origin, ASSET1, USER, 1);
		Assets::mint_into(ASSET1, &USER, MINT);

		assert_noop!(TemplateModule::add_liquidity(Origin::signed(USER), ASSET1, ASSET2, ASSET1_AMOUNT, ASSET2_AMOUNT), Error::<Test>::NotEnoughFundsTokenB);
	});
}

#[test]
fn provide_liquidity_with_too_little_of_tokens_a() {
    new_test_ext().execute_with(|| {
		let origin = Origin::signed(USER);
		Balances::make_free_balance_be(&USER, MINT);
        Assets::create(origin, ASSET2, USER, 1);
		Assets::mint_into(ASSET2, &USER, MINT);

		assert_noop!(TemplateModule::add_liquidity(Origin::signed(USER), ASSET1, ASSET2, ASSET1_AMOUNT, ASSET2_AMOUNT), Error::<Test>::NotEnoughFundsTokenA);
	});
}