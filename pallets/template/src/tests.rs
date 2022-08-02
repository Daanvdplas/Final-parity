use crate::{mock::*, Error};
use frame_support::traits::Currency;
use frame_support::traits::fungibles::Mutate;
use frame_support::{assert_noop, assert_ok};
use frame_support::Hashable;
use scale_info::prelude::vec;
use frame_support::traits::tokens::fungibles::Create;
const USER: AccountId = 1;
const DOT: u32 = 1;
const ETH: u32 = 2;
const ADA: u32 = 3;
const BTC: u32 = 4;
const LP: u32 = 100;
const NOASSET1: u32 = 0;
const NOASSET2: u32 = 5;
const ENOUGH: u128 = 51;
const PLEDGE: u128 = 50;
const NOT_ENOUGH: u128 = 49;
const MINT: u128 = 1_000_000_000_000_000_000_000u128;
// const NEGATIVE: u128 = -1;
// const TOO_LARGE: u128 = 340_282_366_920_938_463_463_374_607_431_768_211_456;

#[test]
fn identical_tokens() {
    new_test_ext().execute_with(|| {
		let origin = Origin::signed(USER);

		assert_noop!(TemplateModule::deposit_liquidity(Origin::signed(USER), DOT, DOT, PLEDGE, PLEDGE), Error::<Test>::IdenticalTokens);
		assert_noop!(TemplateModule::deposit_liquidity(Origin::signed(USER), BTC, BTC, PLEDGE, PLEDGE), Error::<Test>::IdenticalTokens);
		assert_noop!(TemplateModule::deposit_liquidity(Origin::signed(USER), NOASSET1, NOASSET1, PLEDGE, PLEDGE), Error::<Test>::IdenticalTokens);
		
		assert_noop!(TemplateModule::withdraw_liquidity(Origin::signed(USER), ETH, ETH, LP), Error::<Test>::IdenticalTokens);
		assert_noop!(TemplateModule::withdraw_liquidity(Origin::signed(USER), ADA, ADA, LP), Error::<Test>::IdenticalTokens);
		assert_noop!(TemplateModule::withdraw_liquidity(Origin::signed(USER), NOASSET2, NOASSET2, LP), Error::<Test>::IdenticalTokens);
	});
}

#[test]
fn invalid_tokens() {
    new_test_ext().execute_with(|| {

		assert_noop!(TemplateModule::deposit_liquidity(Origin::signed(USER), NOASSET1, ETH, PLEDGE, PLEDGE), Error::<Test>::InvalidToken);
		assert_noop!(TemplateModule::deposit_liquidity(Origin::signed(USER), DOT, NOASSET2, PLEDGE, PLEDGE), Error::<Test>::InvalidToken);
		assert_noop!(TemplateModule::deposit_liquidity(Origin::signed(USER), NOASSET1, NOASSET2, PLEDGE, PLEDGE), Error::<Test>::InvalidToken);

		assert_noop!(TemplateModule::withdraw_liquidity(Origin::signed(USER), NOASSET1, ETH, LP), Error::<Test>::InvalidToken);
		assert_noop!(TemplateModule::withdraw_liquidity(Origin::signed(USER), DOT, NOASSET2, LP), Error::<Test>::InvalidToken);
		assert_noop!(TemplateModule::withdraw_liquidity(Origin::signed(USER), NOASSET1, NOASSET2, LP), Error::<Test>::InvalidToken);
	});
}

#[test]
fn invalid_funds() {
    new_test_ext().execute_with(|| {
		assert_noop!(TemplateModule::deposit_liquidity(Origin::signed(USER), DOT, ETH, PLEDGE, PLEDGE), Error::<Test>::NotEnoughFunds);

		let origin = Origin::signed(USER);
		Balances::make_free_balance_be(&USER, MINT);
        Assets::create(origin, DOT, USER, 1);
		Assets::mint_into(DOT, &USER, NOT_ENOUGH);

		let origin = Origin::signed(USER);
		Balances::make_free_balance_be(&USER, MINT);
        Assets::create(origin, ETH, USER, 1);
		Assets::mint_into(ETH, &USER, NOT_ENOUGH);
        assert_noop!(TemplateModule::deposit_liquidity(Origin::signed(USER), DOT, ETH, PLEDGE, PLEDGE), Error::<Test>::NotEnoughFunds);
    });
}

#[test]
fn provide_liquidity_with_too_little_of_tokens_a() {
    new_test_ext().execute_with(|| {

		let origin = Origin::signed(USER);
		Balances::make_free_balance_be(&USER, MINT);
        Assets::create(origin, DOT, USER, 1);
		Assets::mint_into(DOT, &USER, NOT_ENOUGH);

		let origin = Origin::signed(USER);
		Balances::make_free_balance_be(&USER, MINT);
        Assets::create(origin, ETH, USER, 1);
		Assets::mint_into(ETH, &USER, ENOUGH);
		assert_noop!(TemplateModule::deposit_liquidity(Origin::signed(USER), DOT, ETH, PLEDGE, PLEDGE), Error::<Test>::NotEnoughFundsTokenA);
	});
}

#[test]
fn provide_liquidity_with_too_little_of_tokens_b() {
    new_test_ext().execute_with(|| {
		let origin = Origin::signed(USER);
		Balances::make_free_balance_be(&USER, MINT);
        Assets::create(origin, DOT, USER, 1);
		Assets::mint_into(DOT, &USER, PLEDGE);

		let origin = Origin::signed(USER);
		Balances::make_free_balance_be(&USER, MINT);
        Assets::create(origin, ETH, USER, 1);
		Assets::mint_into(ETH, &USER, NOT_ENOUGH - 1);
		assert_noop!(TemplateModule::deposit_liquidity(Origin::signed(USER), DOT, ETH, PLEDGE, PLEDGE), Error::<Test>::NotEnoughFundsTokenB);
	});
}

#[test]
fn provide_liquidity_with_enough_funds() {
    new_test_ext().execute_with(|| {
		let origin = Origin::signed(USER);
		Balances::make_free_balance_be(&USER, MINT);
        Assets::create(origin, DOT, USER, 1);
		Assets::mint_into(DOT, &USER, ENOUGH);

		let origin = Origin::signed(USER);
		Balances::make_free_balance_be(&USER, MINT);
        Assets::create(origin, ETH, USER, 1);
		Assets::mint_into(ETH, &USER, PLEDGE);
        assert_ok!(TemplateModule::deposit_liquidity(Origin::signed(USER), DOT, ETH, PLEDGE, PLEDGE));
    });
}

