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

#[derive(Debug, PartialEq)]
pub struct Deposit {
		tokenpair: Vec<u32>,
		tokenpair_id: [u8; 16],
		quantity_token_a: u128,
		quantity_token_b: u128,
	}

#[derive(Debug, PartialEq)]
pub struct Withdrawal {
	tokenpair: Vec<u32>,
	tokenpair_id: [u8; 16],
	lp_token: u32,
}

fn create_deposit(
	token_a: u32,
	token_b: u32,
	unsorted_quant_token_a: u128,
	unsorted_quant_token_b: u128,
) -> Deposit {

	let mut tokenpair = vec![token_a, token_b];
	let cloned_tokenpair = tokenpair.clone();
	tokenpair.sort();
	let tokenpair_id = create_token_pair_id(tokenpair[0], tokenpair[1]);
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
	token_a: u32,
	token_b: u32,
	lp_token: u32,
) -> Withdrawal {
	let mut tokenpair = vec![token_a, token_b];
	tokenpair.sort();
	let tokenpair_id = create_token_pair_id(tokenpair[0], tokenpair[1]);
	Withdrawal {
		tokenpair,
		tokenpair_id,
		lp_token,
	}
}

fn create_token_pair_id(token_a: u32, token_b: u32) -> [u8; 16] {
	let mut hash1 = token_a.blake2_128_concat();
	let mut hash2 = token_b.blake2_128_concat();
	hash1.append(&mut hash2);
	let pool_id = hash1.blake2_128();
	pool_id
}

#[test]
fn test_create_tokenpair_id() {
	let mut hash1 = DOT.blake2_128_concat();
	let mut hash2 = ETH.blake2_128_concat();
	hash1.append(&mut hash2);
	let pool_id = hash1.blake2_128();
	assert_eq!(pool_id, create_token_pair_id(DOT, ETH));
}

#[test]
fn test_create_deposit() {
	let deposit = create_deposit(ETH, DOT, ENOUGH, PLEDGE);
	let tokenpair_id = create_token_pair_id(DOT, ETH);
	let test = Deposit {
		tokenpair: vec![DOT, ETH],
		tokenpair_id: tokenpair_id,
		quantity_token_a: PLEDGE,
		quantity_token_b: ENOUGH,
	};
	assert_eq!(deposit, test);
}

#[test]
fn test_create_withdrawal() {
	let withdrawal = create_withdrawal(ETH, DOT, LP);
	let tokenpair_id = create_token_pair_id(DOT, ETH);
	let test = Withdrawal {
		tokenpair: vec![DOT, ETH],
		tokenpair_id: tokenpair_id,
		lp_token: LP,
	};
	assert_eq!(withdrawal, test);
}