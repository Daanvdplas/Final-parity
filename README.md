# Daan's Substrate Node Template with a DEX pallet

# Goal:
Make a DEX pallet. Create 4 accounts with a certain amount of DOT, ETH, ADA and BTC.
Using the following extrinsics they could interact with my blockchain/pallet:

## Functionality - extrinsics
- Deposit liquidity:
    To deposit liquidity, the extrinsic needs two tokens (ID's) the user wants to provide liquidity
    with, and the amount of each token. If the pool does not exist, it is created during runtime
    as well as the lp token for that pool. If the pool already exists liquidity is added. If all
    checks are passed, in both cases, the user is rewarded in lp tokens.

- Withdraw liquidity:
    To withdraw liquidity, the extrinsic needs the two tokens (ID's) the user provided liquidity
    with as well as the lp token (ID's). If all checks are passed, liquidity is withdrawn and 
    user is rewarded in both tokens.

- Swap:
    To swap, the extrinsic needs the swap pair (token ID's) as well as the amount of tokens the 
    user wants to swap. If all checks are passed, the swap is executed and the user is rewarded
    in the other token.

### What could have been done better
Due to the time stress I've been more careless about a few things what I would have done differently
if I had more time:

First, I haven't implemented the fees. In addition, I haven't looked carefully how to work 
with the decimals.

Secondly, I think I tested all my errors, but tested only with scenarios specific to that error.

Thirdly, some funtions are messy and the level of clean code could be a lot higher.

In general, I had to rush a lot, really want to go over everything a let it sink.

#### On the whole
Wow what a cool project! Coming from C was a hard road but I'm proud of my project. I learned a 
lot the last couple of days and I can't wait to improve in substrate/frame! 


