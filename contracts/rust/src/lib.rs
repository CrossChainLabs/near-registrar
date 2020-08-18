/**
* Top level account names (TLAs) are very valuable as they provide root of trust and discoverability for 
* companies, applications and users. To allow for fair access to them, the top level account names that 
* are shorter than MIN_ALLOWED_TOP_LEVEL_ACCOUNT_LENGTH characters (32 at time of writing) will be auctioned off.
* NOTES:
*  - Each week’s account names—such that hash(account_id) % 52 is equal to the week since the launch of the 
*    auction—will open for bidding. 
*  - Auctions will run for seven days after the first bid, and anyone can bid for a given name. 
*  - A bid consists of a bid and mask, allowing the bidder to hide the amount that they are bidding. 
*  - After the seven days run out, participants must reveal their bid and mask within the next seven days.
*  - The winner of the auction pays the second-largest price.
*  - Proceeds of the auctions then get burned by the naming contract, benefiting all the token holders.
*  - Done: account was claimed and created, the auction is done and all state will be cleared except that 
*    this name is in done collection. On claim also withdraws all other bids automatically.
*/

use near_sdk::json_types::Base58PublicKey;
use near_sdk::{env, wee_alloc, AccountId, Balance, Promise, BlockHeight};
use std::collections::HashMap;
use std::str;

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hasher}; 

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

pub struct Bid {
    amount: Balance,
    commitment: Vec<u8>
}

// AccountId of the bidder and AccountId of the revealer
pub struct Auction {
    start_block_height: BlockHeight,
    bids: HashMap<AccountId, Bid>,
    reveals: HashMap<AccountId, Balance>,
}

// AccountId that is auctioned
pub struct Registrar {
    start_block_height: BlockHeight,
    auction_period: BlockHeight,
    reveal_period: BlockHeight,
    auctions: HashMap<AccountId, Auction>
}

impl Registrar {  
    /// Construct this contract and record starting block height.
    /// auction_period represents the number of blocks an auction can take, aproximately 7 days
    /// reveal_period represents the number of blocks the reveal period can take, aproximately 7 days
    pub fn new(auction_period: BlockHeight, reveal_period: BlockHeight) -> Self {
        Self {
            start_block_height: env::block_index(),
            auction_period: auction_period,
            reveal_period: reveal_period,
            auctions: HashMap::new(),
        }
    }

    /// Attached deposit serves as locking funds for given account name.
    /// Commitment is `hash(masked amount + salt)` in base58 encoding.
    /// bid fails if `account_id` is not yet on the market based on `hash(account_id) % 52 > weeks from start_blockhegiht`
    /// bid records a new auction if auction for this name doesn't exist yet.
    /// bid fails if auction period expired.
    pub fn bid(&mut self, account_id: AccountId, commitment: Vec<u8>) -> bool {
        let new_bid = Bid {
            amount: 0,
            commitment: commitment
        };

        let bidder_account_id: AccountId = env::predecessor_account_id();
        println!("bidder_account_id = {}", &bidder_account_id.to_string());

        match self.auctions.get_mut(&account_id) {
            Some(auction) => {
                // check if auction expired
                let current_blockheight = env::block_index();
                if current_blockheight - auction.start_block_height > self.auction_period {
                    return false;
                }
            
                // insert into bids map
                auction.bids.insert(bidder_account_id, new_bid);
            },
            None => {        
                let current_blockheight = env::block_index();

                /* println!("current_blockheight = {}", &current_blockheight.to_string());
                println!("start_block_height = {}", &self.start_block_height.to_string());
                println!("auction_period = {}", &self.auction_period.to_string());
                */

                // calculate number of weeks since auction started
                let weeks = (current_blockheight - self.start_block_height) / self.auction_period;

                // calculate account_id hash
                let mut account_hasher = DefaultHasher::new();
                account_hasher.write(account_id.as_bytes());
                let account_hash = account_hasher.finish();

                /* println!("account_hash = {}", &account_hash.to_string());
                println!("account_hash % 52 = {}", (account_hash % 52).to_string());
                println!("weeks = {}", &weeks.to_string());
                */

                // check if account_id is open for auction
                if account_hash % 52 > weeks {
                    return false;
                }

                // insert this new auction to auction list
                let mut new_auction = Auction {
                                    start_block_height: env::block_index(),
                                    bids:  HashMap::new(),
                                    reveals:  HashMap::new(),
                                };
                new_auction.bids.insert(bidder_account_id, new_bid);
                self.auctions.insert(account_id, new_auction);
            }
        }

        return true;
    }

    /// Reveal shows the masked amount and salt. Invalid reveals are declined.
    /// Reveal fails if auction is still going.
    /// Reveal fails if `hash(masked_amount + salt)` != `commitment` by env::predeccessor_account_id()`
    pub fn reveal(&mut self, account_id: AccountId, masked_amount: Balance, salt: String) -> bool {
        let revealer_account_id: AccountId = env::predecessor_account_id();

        // check if masked amount was deposited
        if masked_amount != env::attached_deposit() {
            return false;
        }

        // proceed to reveal operation
        match self.auctions.get_mut(&account_id) {
            Some(auction) => {
                // check if auction is in progress
                let current_blockheight = env::block_index();
                if current_blockheight - auction.start_block_height <= self.auction_period {
                    return false;
                }

                // check if reveal period expired
                if current_blockheight - auction.start_block_height > self.auction_period + self.reveal_period {
                    return false;
                }

                // check if `hash(masked_amount + salt)` != `commitment` by env::predeccessor_account_id()`
                match auction.bids.get_mut(&revealer_account_id) {
                    Some(bid) => {
                        // calculate hash(masked_amount + salt)
                        let commitment_hash = masked_amount.to_string() + &salt;
                        let revealer_commitment = &bs58::encode(&commitment_hash).into_string();
                        if str::from_utf8(&bid.commitment).unwrap() != revealer_commitment {
                            return false;
                        }

                        // set the missing bid amount info
                        bid.amount = masked_amount;
                    }
                    None => {
                        return false;
                    }
                }
                
                // insert into reveal's map
                auction.reveals.insert(revealer_account_id, masked_amount);
            },
            None => {
                return false;
            }
        }
        return true;
    }

    /// Withdraw funds for loosing bids.
    /// Withdraw fails if account_id doesn't exist, if `env::predeccessor_account_id()` didn't bid or if auction is still in progress or not all bids were revealed yet.
    /// If not all bids were revealed but required reveal period passed, can withdraw.
    pub fn withdraw(&mut self, account_id: AccountId) -> bool {
        let withdrawer_account_id: AccountId = env::predecessor_account_id();
        match self.auctions.get_mut(&account_id) {
            Some(auction) => {
                // return false if the auction is in progress 
                let current_blockheight = env::block_index();
                if current_blockheight - auction.start_block_height <= self.auction_period {
                    return false;
                }

                // return false if reveal is in progress and not all bidders revealed themselves
                if current_blockheight - auction.start_block_height <= self.auction_period + self.reveal_period {
                    if auction.bids.len() != auction.reveals.len() {
                        return false;
                    }
                }

                // withdraw funds for loosing bider
                match auction.bids.get_mut(&withdrawer_account_id) {
                    Some(bid) => {
                        // transfer back the bid.amount
                        Promise::new(withdrawer_account_id.to_string()).transfer(bid.amount);
                        bid.amount = 0;
                    }
                    None => {
                        return false;
                    }
                }
            }
            None => {
                return false;
            }
        }
        return true;

    }

    /// Creates the new name with given public key for the winer.
    pub fn claim(&mut self, account_id: AccountId, public_key: Base58PublicKey) -> bool {
        let mut winning_account_id: AccountId = "".to_string();
        let mut second_highest_bid: Balance = 0;
        match self.auctions.get_mut(&account_id) {
            Some(auction) => {
                // check if auction is in progress
                let current_blockheight = env::block_index();
                if current_blockheight - auction.start_block_height <= self.auction_period {
                    return false;
                }

                // check if reaveal is in progress 
                if current_blockheight - auction.start_block_height <= self.auction_period + self.reveal_period {
                    // check if all bidders revealed themselves
                    if auction.bids.len() != auction.reveals.len() {
                        return false;
                    }
                }

                // get the second highest bid
                let mut highest_bid: Balance = 0;
                let mut is_first_check: bool = true;
                for (revealer_account_id, revealer_balance) in &auction.reveals {

                    // set the highest_bid as the first map entry
                    if is_first_check {
                        highest_bid = *revealer_balance;
                        is_first_check = false;
                        winning_account_id = revealer_account_id.to_string();
                        continue;
                    }

                    if *revealer_balance > second_highest_bid {
                        second_highest_bid = *revealer_balance;

                        if highest_bid < second_highest_bid {
                            let temp = highest_bid;
                            highest_bid = second_highest_bid;
                            second_highest_bid = temp;
                            winning_account_id = revealer_account_id.to_string();
                        }                     
                    }
                }

                
                // if second_highest_bid = 0, nobody wins
                if second_highest_bid == 0 {
                    return false;

                    // TODO: uncomment if the second_highest_bid should take the value of the highest_bid in case the second_highest_bid is 0
                    /*if highest_bid == 0 {
                        return false;
                    } else {     
                        second_highest_bid = highest_bid;
                    }*/
                }

                // check if the claimer is also the winner
                let claimer_account_id: AccountId = env::predecessor_account_id();
                if winning_account_id != claimer_account_id {
                    return false;
                }

                // TODO: burn the locked amount, which is

                // creates the new name with given public key for the winer
                let key = Base58PublicKey::from(public_key);
                let p1 = Promise::new(account_id.to_string()).create_account();
                let p2 = Promise::new(account_id.to_string()).add_full_access_key(key.0);
                p1.then(p2);

                // withdraw all other bids automatically
                for (bidder_account_id, bid) in auction.bids.iter_mut() {
                    if &claimer_account_id != bidder_account_id {
                        // transfer back the bid.amount
                        Promise::new(bidder_account_id.to_string()).transfer(bid.amount);
                        bid.amount = 0;
                    }
                }
            }
            None => {
                return false;
            }
        }      
        return true;
    }
}


#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use near_sdk::MockedBlockchain;
    use near_sdk::{testing_env, VMContext};

    use super::*;

    fn alice() -> AccountId {
        "alice.near".to_string()
    }
    fn bob() -> AccountId {
        "bob.near".to_string()
    }
    fn carol() -> AccountId {
        "carol.near".to_string()
    }
    fn auctioned_id() -> AccountId {
        "auctioned_id1.near".to_string()
    }

    fn get_context(predecessor_account_id: AccountId) -> VMContext {
        VMContext {
            current_account_id: alice(),
            signer_account_id: bob(),
            signer_account_pk: vec![0, 1, 2],
            predecessor_account_id,
            input: vec![],
            block_index: 2,
            block_timestamp: 0,
            account_balance: 1_000_000_000_000_000_000_000_000_000u128,
            account_locked_balance: 0,
            storage_usage: 10u64.pow(6),
            attached_deposit: 0,
            prepaid_gas: 10u64.pow(18),
            random_seed: vec![0, 1, 2],
            is_view: false,
            output_data_receivers: vec![],
            epoch_height: 0,
        }
    }

    fn get_context2(predecessor_account_id: AccountId) -> VMContext {
        VMContext {
            current_account_id: alice(),
            signer_account_id: bob(),
            signer_account_pk: vec![0, 1, 2],
            predecessor_account_id,
            input: vec![],
            block_index: 1321,
            block_timestamp: 0,
            account_balance: 1_000_000_000_000_000_000_000_000_000u128,
            account_locked_balance: 0,
            storage_usage: 10u64.pow(6),
            attached_deposit: 0,
            prepaid_gas: 10u64.pow(18),
            random_seed: vec![0, 1, 2],
            is_view: false,
            output_data_receivers: vec![],
            epoch_height: 0,
        }
    }


    #[test]

    fn test_initialize_new_registrar_and_bid() {
        let context = get_context(carol());
        testing_env!(context);
        let mut contract = Registrar::new(30, 35);

        let context2 = get_context2(carol());
        testing_env!(context2);
        let commitment = "test1test2test3hashCommitment";
        assert_eq!(contract.bid(auctioned_id(), commitment.as_bytes().to_vec()), true);
    }

    #[test]

    fn test_another_bid() {
        let context = get_context(bob());
        testing_env!(context);
        let mut contract = Registrar::new(30, 35);

        let context2 = get_context2(bob());
        testing_env!(context2);
        let commitment = "test1test2test3hashCommitment";
        assert_eq!(contract.bid(auctioned_id(), commitment.as_bytes().to_vec()), true);
    }

}

