Top level accounts (TLAs)
==============================

This is an implementation in rust for Top Level Account Registrar.

This repository includes TLAs implementation in Rust (https://github.com/near/core-contracts/issues/25)

Top level account names (TLAs) are very valuable as they provide root of trust and discoverability for 
companies, applications and users. To allow for fair access to them, the top level account names that 
are shorter than MIN_ALLOWED_TOP_LEVEL_ACCOUNT_LENGTH characters (32 at time of writing) will be auctioned off.

# Rust

_Using Gitpod? You can skip these setup steps!_

To run this project locally:

1. Prerequisites: Make sure you have Node.js ≥ 12 installed (https://nodejs.org), then use it to install [yarn]: `npm install --global yarn` (or just `npm i -g yarn`)
2. Install dependencies: `yarn install` (or just `yarn`)
3. Follow instructions for installing [rust] here https://docs.near.org/docs/roles/developer/contracts/near-sdk-rs#pre-requisites

Now you can run all the [rust]-related scripts listed in `package.json`! Scripts you might want to start with:

- `yarn test`: Runs all Rust tests in the project
- `yarn build`: Compiles the Rust contracts to [Wasm] binaries

## Data collection

By using Gitpod in this project, you agree to opt-in to basic, anonymous analytics. No personal information is transmitted. Instead, these usage statistics aid in discovering potential bugs and user flow information.

  [rust]: https://www.rust-lang.org/
  [yarn]: https://yarnpkg.com/
  [Wasm]: https://webassembly.org/

# Reference-level explanation

The full implementation in Rust can be found here: https://github.com/CrossChainLabs/near-registrar/blob/master/contracts/rust/src/lib.rs

**NOTES**
  - Each week’s account names—such that hash(account_id) % 52 is equal to the week since the launch of the 
    auction—will open for bidding. 
  - Auctions will run for seven days after the first bid, and anyone can bid for a given name. 
  - A bid consists of a bid and mask, allowing the bidder to hide the amount that they are bidding. 
  - After the seven days run out, participants must reveal their bid and mask within the next seven days.
  - The winner of the auction pays the second-largest price.
  - Proceeds of the auctions then get burned by the naming contract, benefiting all the token holders.
  - Done: account was claimed and created, the auction is done and all state will be cleared except that 
    this name is in done collection. On claim also withdraws all other bids automatically.

