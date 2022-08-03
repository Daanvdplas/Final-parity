use crate::{mock::*, Error};
use frame_support::traits::Currency;
use frame_support::traits::fungibles::Mutate;
use frame_support::pallet_prelude::*;
use frame_support::{assert_noop, assert_ok};
use frame_support::Hashable;

const USER: AccountId = 1;
const USER2: AccountId = 2;
const DOT: u32 = 1;
const ETH: u32 = 2;
const ADA: u32 = 3;
const BTC: u32 = 4;
const LP: u32 = 100;
const NOASSET1: u32 = 0;
const NOASSET2: u32 = 5;
const A_LOT: u128 = 1_000_000_000_000;
const TOO_MUCH: u128 = 1_000_000_000_000_000_000_000_000_000_000u128;
const PLEDGE: u128 = 50_000_000;
const NOT_ENOUGH: u128 = 49_000_000;

#[derive(Debug, PartialEq)]
pub struct Withdrawal {
	tokenpair: Vec<u32>,
	tokenpair_id: [u8; 16],
	lp_token: u32,
}

fn create_user_with_one_asset(user: AccountId, asset: u32, balance: u128) -> AccountId {
	let origin = Origin::signed(user);
	Balances::make_free_balance_be(&user, balance);
	Assets::create(origin, asset, user, 1);
	Assets::mint_into(asset, &user, balance);
	user
}

fn create_user_with_two_assets(user: AccountId, asset1: u32, asset2: u32, balance: u128) -> AccountId {
	let origin = Origin::signed(user);
	Balances::make_free_balance_be(&user, balance);
	Assets::create(origin, asset1, user, 1);
	Assets::mint_into(asset1, &user, balance);

	let origin = Origin::signed(user);
	Balances::make_free_balance_be(&user, balance);
	Assets::create(origin, asset2, user, 1);
	Assets::mint_into(asset2, &user, balance);
	user
}

fn create_token_pair_id(token_a: u32, token_b: u32) -> [u8; 16] {
	let mut hash1 = token_a.blake2_128_concat();
	let mut hash2 = token_b.blake2_128_concat();
	hash1.append(&mut hash2);
	let pool_id = hash1.blake2_128();
	pool_id
}

#[test]
fn test_identicaltokens_error() {
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
fn test_invalidtoken_error() {
    new_test_ext().execute_with(|| {

		assert_noop!(TemplateModule::deposit_liquidity(Origin::signed(USER), NOASSET1, ETH, PLEDGE, PLEDGE), Error::<Test>::InvalidToken);
		assert_noop!(TemplateModule::deposit_liquidity(Origin::signed(USER), DOT, NOASSET2, PLEDGE, PLEDGE), Error::<Test>::InvalidToken);
		assert_noop!(TemplateModule::deposit_liquidity(Origin::signed(USER), NOASSET1, NOASSET2, PLEDGE, PLEDGE), Error::<Test>::InvalidToken);

		assert_noop!(TemplateModule::withdraw_liquidity(Origin::signed(USER), NOASSET1, ETH, LP), Error::<Test>::InvalidToken);
		assert_noop!(TemplateModule::withdraw_liquidity(Origin::signed(USER), DOT, NOASSET2, LP), Error::<Test>::InvalidToken);
		assert_noop!(TemplateModule::withdraw_liquidity(Origin::signed(USER), NOASSET1, NOASSET2, LP), Error::<Test>::InvalidToken);

		assert_noop!(TemplateModule::swap(Origin::signed(USER), NOASSET1, ETH, PLEDGE), Error::<Test>::InvalidToken);
		assert_noop!(TemplateModule::swap(Origin::signed(USER), DOT, NOASSET2, PLEDGE), Error::<Test>::InvalidToken);
		assert_noop!(TemplateModule::swap(Origin::signed(USER), NOASSET1, NOASSET2, PLEDGE), Error::<Test>::InvalidToken);
	});
}

#[test]
fn test_notenoughfunds_error() {
    new_test_ext().execute_with(|| {
		let user = create_user_with_two_assets(USER, DOT, ETH, NOT_ENOUGH);
        assert_noop!(TemplateModule::deposit_liquidity(Origin::signed(user), DOT, ETH, PLEDGE, PLEDGE), Error::<Test>::NotEnoughFunds);
        assert_noop!(TemplateModule::swap(Origin::signed(user), DOT, ETH, PLEDGE), Error::<Test>::NotEnoughFunds);
    });
}

#[test]
fn test_notenoughfundstokena_error() {
    new_test_ext().execute_with(|| {
		let user = create_user_with_two_assets(USER, DOT, ETH, PLEDGE);
        assert_noop!(TemplateModule::deposit_liquidity(Origin::signed(user), DOT, ETH, A_LOT, PLEDGE), Error::<Test>::NotEnoughFundsTokenA);
	});
}

#[test]
fn test_notenoughfundstokenb_error() {
    new_test_ext().execute_with(|| {
		let user = create_user_with_two_assets(USER, DOT, ETH, PLEDGE);
        assert_noop!(TemplateModule::deposit_liquidity(Origin::signed(user), DOT, ETH, PLEDGE, A_LOT), Error::<Test>::NotEnoughFundsTokenB);
	});
}

#[test]
fn test_deposit_ok() {
    new_test_ext().execute_with(|| {
		let user = create_user_with_two_assets(USER, DOT, ETH, A_LOT);
        assert_ok!(TemplateModule::deposit_liquidity(Origin::signed(user), DOT, ETH, PLEDGE, PLEDGE));
    });
}

#[test]
fn test_maxliqproviders_error() {
    new_test_ext().execute_with(|| {
		let user1 = create_user_with_two_assets(1, DOT, ETH, A_LOT);
        assert_ok!(TemplateModule::deposit_liquidity(Origin::signed(user1), DOT, ETH, PLEDGE, PLEDGE));

		let user2 = create_user_with_two_assets(2, DOT, ETH, A_LOT);
        assert_ok!(TemplateModule::deposit_liquidity(Origin::signed(user2), DOT, ETH, PLEDGE, PLEDGE));

		let user3 = create_user_with_two_assets(3, DOT, ETH, A_LOT);
        assert_ok!(TemplateModule::deposit_liquidity(Origin::signed(user3), DOT, ETH, PLEDGE, PLEDGE));

		let user4 = create_user_with_two_assets(4, DOT, ETH, A_LOT);
        assert_ok!(TemplateModule::deposit_liquidity(Origin::signed(user4), DOT, ETH, PLEDGE, PLEDGE));
		
		let user5 = create_user_with_two_assets(5, DOT, ETH, A_LOT);
        assert_noop!(TemplateModule::deposit_liquidity(Origin::signed(user5), DOT, ETH, PLEDGE, PLEDGE), Error::<Test>::LiqProvidersOverflow);
    });
}

#[test]
fn test_mathoverflow_error() {
    new_test_ext().execute_with(|| {
		// Some how this test does not pass, but it gives me the correct MathProblem error
		let user = create_user_with_two_assets(USER, DOT, ETH, TOO_MUCH);
		assert_noop!(TemplateModule::deposit_liquidity(Origin::signed(user), DOT, ETH, TOO_MUCH, TOO_MUCH), Error::<Test>::MathProblem);
    });
}

#[test]
fn test_nolptokens_error() {
    new_test_ext().execute_with(|| {
        assert_noop!(TemplateModule::withdraw_liquidity(Origin::signed(USER), DOT, ETH, LP), Error::<Test>::NoTokens);
		let user = create_user_with_one_asset(USER, LP, 0);
        assert_noop!(TemplateModule::withdraw_liquidity(Origin::signed(user), DOT, ETH, LP), Error::<Test>::NoTokens);
    });
}

#[test]
fn test_poolnotfound_error() {
    new_test_ext().execute_with(|| {
		// Hacky way of testing the check_if_valid_tokens function without depositing first and letting
		// the lp token exist
		let user = create_user_with_two_assets(USER, DOT, ETH, A_LOT);
        assert_noop!(TemplateModule::withdraw_liquidity(Origin::signed(user), DOT, ETH, ETH), Error::<Test>::PoolNotFound);
        assert_noop!(TemplateModule::swap(Origin::signed(user), DOT, ETH, PLEDGE), Error::<Test>::PoolNotFound);
    });
}

#[test]
fn test_noliquidityprovided_error() {
    new_test_ext().execute_with(|| {
		let user1 = create_user_with_two_assets(USER, DOT, ETH, A_LOT);
        assert_ok!(TemplateModule::deposit_liquidity(Origin::signed(user1), DOT, ETH, PLEDGE, PLEDGE));
		let user2 = create_user_with_one_asset(USER2, ETH, A_LOT);
        assert_noop!(TemplateModule::withdraw_liquidity(Origin::signed(user2), DOT, ETH, ETH), Error::<Test>::NoLiquidityProvided);
    });
}

#[test]
fn test_withdrawal_ok() {
    new_test_ext().execute_with(|| {
		let tokenpair_id = create_token_pair_id(DOT, ETH);
		let lp_token_id = u32::decode(&mut &*tokenpair_id.to_vec()).unwrap();
		let user = create_user_with_two_assets(USER, DOT, ETH, A_LOT);
        assert_ok!(TemplateModule::deposit_liquidity(Origin::signed(user), DOT, ETH, PLEDGE, PLEDGE));
		let user2 = create_user_with_two_assets(USER2, DOT, ETH, A_LOT);
        assert_ok!(TemplateModule::deposit_liquidity(Origin::signed(user2), DOT, ETH, PLEDGE, PLEDGE));
        assert_ok!(TemplateModule::withdraw_liquidity(Origin::signed(user), DOT, ETH, lp_token_id));
    });
}

#[test]
fn test_swap_ok() {
    new_test_ext().execute_with(|| {
		let user = create_user_with_two_assets(USER, DOT, ETH, A_LOT);
        assert_ok!(TemplateModule::deposit_liquidity(Origin::signed(user), DOT, ETH, PLEDGE, PLEDGE));
		assert_ok!(TemplateModule::swap(Origin::signed(user), DOT, ETH, NOT_ENOUGH));
    });
}