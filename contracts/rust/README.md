# Top Level Account Registrar

This is an implementation in rust for Top Level Account Registrar.

Top level account names (TLAs) are very valuable as they provide root of trust and discoverability for 
companies, applications and users. To allow for fair access to them, the top level account names that 
are shorter than MIN_ALLOWED_TOP_LEVEL_ACCOUNT_LENGTH characters (32 at time of writing) will be auctioned off.

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
