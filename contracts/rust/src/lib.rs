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

              /*  println!("current_blockheight = {}", &current_blockheight.to_string());
                println!("start_block_height = {}", &self.start_block_height.to_string());
                println!("auction_period = {}", &self.auction_period.to_string());
                */

                // calculate number of weeks since auction started
                let weeks = (current_blockheight - self.start_block_height) / self.auction_period;

                // calculate account_id hash
                let mut account_hasher = DefaultHasher::new();
                account_hasher.write(account_id.as_bytes());
                let account_hash = account_hasher.finish();

              /*  println!("account_hash = {}", &account_hash.to_string());
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
                // return if the auction is in progress 
                let current_blockheight = env::block_index();
                if current_blockheight - auction.start_block_height <= self.auction_period {
                    return false;
                }

                // return if reveal is in progress and not all bidders revealed themselves
                if current_blockheight - auction.start_block_height <= self.auction_period + self.reveal_period {
                    if auction.bids.len() != auction.reveals.len() {
                        return false;
                    }
                }

                // withdraw funds for loosing bid
                match auction.bids.get_mut(&withdrawer_account_id) {
                    Some(_bid) => {
                        // transfer back the bid.amount
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

                // if second_highest_bid = 0, use the heighest_bid 
                if second_highest_bid == 0 {
                    // of highest_bid = 0 return false
                    if highest_bid == 0 {
                        return false;
                    } else {
                        // TODO: uncomment if needed
                        // second_highest_bid = highest_bid;
                    }
                }
            }
            None => {
                return false;
            }
        }

        let claimer_account_id: AccountId = env::predecessor_account_id();

        if winning_account_id == claimer_account_id {
            // TODO: burn the locked amount

            // creates the new name with given public key for the winer
            let key = Base58PublicKey::from(public_key);
            let p1 = Promise::new(account_id.to_string()).create_account();
            let p2 = Promise::new(account_id.to_string()).add_full_access_key(key.0);
            p1.then(p2);

            //TODO: withdraw all other bids automatically.
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






/*
/// Contains balance and allowances information for one account.
#[derive(BorshDeserialize, BorshSerialize)]
pub struct Account {
    /// Current account balance.
    pub balance: Balance,
    /// Escrow Account ID hash to the allowance amount.
    /// Allowance is the amount of tokens the Escrow Account ID can spent on behalf of the account
    /// owner.
    pub allowances: UnorderedMap<Vec<u8>, Balance>,
}

impl Account {
    /// Initializes a new Account with 0 balance and no allowances for a given `account_hash`.
    pub fn new(account_hash: Vec<u8>) -> Self {
        Self {
            balance: 0,
            allowances: UnorderedMap::new(account_hash),
        }
    }

    /// Sets allowance for account `escrow_account_id` to `allowance`.
    pub fn set_allowance(&mut self, escrow_account_id: &AccountId, allowance: Balance) {
        let escrow_hash = env::sha256(escrow_account_id.as_bytes());
        if allowance > 0 {
            self.allowances.insert(&escrow_hash, &allowance);
        } else {
            self.allowances.remove(&escrow_hash);
        }
    }

    /// Returns the allowance of account `escrow_account_id`.
    pub fn get_allowance(&self, escrow_account_id: &AccountId) -> Balance {
        let escrow_hash = env::sha256(escrow_account_id.as_bytes());
        self.allowances.get(&escrow_hash).unwrap_or(0)
    }
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct FungibleToken {
    /// sha256(AccountID) -> Account details.
    pub accounts: UnorderedMap<Vec<u8>, Account>,

    /// Total supply of the all token.
    pub total_supply: Balance,
}

impl Default for FungibleToken {
    fn default() -> Self {
        panic!("Fun token should be initialized before usage")
    }
}

#[near_bindgen]
impl FungibleToken {
    /// Initializes the contract with the given total supply owned by the given `owner_id`.
    #[init]
    pub fn new(owner_id: AccountId, total_supply: U128) -> Self {
        let total_supply = total_supply.into();
        assert!(!env::state_exists(), "Already initialized");
        let mut ft = Self {
            accounts: UnorderedMap::new(b"a".to_vec()),
            total_supply,
        };
        let mut account = ft.get_account(&owner_id);
        account.balance = total_supply;
        ft.set_account(&owner_id, &account);
        ft
    }

    /// Increments the `allowance` for `escrow_account_id` by `amount` on the account of the caller of this contract
    /// (`predecessor_id`) who is the balance owner.
    /// Requirements:
    /// * Caller of the method has to attach deposit enough to cover storage difference at the
    ///   fixed storage price defined in the contract.
    #[payable]
    pub fn inc_allowance(&mut self, escrow_account_id: AccountId, amount: U128) {
        let initial_storage = env::storage_usage();
        assert!(
            env::is_valid_account_id(escrow_account_id.as_bytes()),
            "Escrow account ID is invalid"
        );
        let owner_id = env::predecessor_account_id();
        if escrow_account_id == owner_id {
            env::panic(b"Can not increment allowance for yourself");
        }
        let mut account = self.get_account(&owner_id);
        let current_allowance = account.get_allowance(&escrow_account_id);
        account.set_allowance(
            &escrow_account_id,
            current_allowance.saturating_add(amount.0),
        );
        self.set_account(&owner_id, &account);
        self.refund_storage(initial_storage);
    }

    /// Decrements the `allowance` for `escrow_account_id` by `amount` on the account of the caller of this contract
    /// (`predecessor_id`) who is the balance owner.
    /// Requirements:
    /// * Caller of the method has to attach deposit enough to cover storage difference at the
    ///   fixed storage price defined in the contract.
    #[payable]
    pub fn dec_allowance(&mut self, escrow_account_id: AccountId, amount: U128) {
        let initial_storage = env::storage_usage();
        assert!(
            env::is_valid_account_id(escrow_account_id.as_bytes()),
            "Escrow account ID is invalid"
        );
        let owner_id = env::predecessor_account_id();
        if escrow_account_id == owner_id {
            env::panic(b"Can not decrement allowance for yourself");
        }
        let mut account = self.get_account(&owner_id);
        let current_allowance = account.get_allowance(&escrow_account_id);
        account.set_allowance(
            &escrow_account_id,
            current_allowance.saturating_sub(amount.0),
        );
        self.set_account(&owner_id, &account);
        self.refund_storage(initial_storage);
    }

    /// Transfers the `amount` of tokens from `owner_id` to the `new_owner_id`.
    /// Requirements:
    /// * `amount` should be a positive integer.
    /// * `owner_id` should have balance on the account greater or equal than the transfer `amount`.
    /// * If this function is called by an escrow account (`owner_id != predecessor_account_id`),
    ///   then the allowance of the caller of the function (`predecessor_account_id`) on
    ///   the account of `owner_id` should be greater or equal than the transfer `amount`.
    /// * Caller of the method has to attach deposit enough to cover storage difference at the
    ///   fixed storage price defined in the contract.
    #[payable]
    pub fn transfer_from(&mut self, owner_id: AccountId, new_owner_id: AccountId, amount: U128) {
        let initial_storage = env::storage_usage();
        assert!(
            env::is_valid_account_id(new_owner_id.as_bytes()),
            "New owner's account ID is invalid"
        );
        let amount = amount.into();
        if amount == 0 {
            env::panic(b"Can't transfer 0 tokens");
        }
        assert_ne!(
            owner_id, new_owner_id,
            "The new owner should be different from the current owner"
        );
        // Retrieving the account from the state.
        let mut account = self.get_account(&owner_id);

        // Checking and updating unlocked balance
        if account.balance < amount {
            env::panic(b"Not enough balance");
        }
        account.balance -= amount;

        // If transferring by escrow, need to check and update allowance.
        let escrow_account_id = env::predecessor_account_id();
        if escrow_account_id != owner_id {
            let allowance = account.get_allowance(&escrow_account_id);
            if allowance < amount {
                env::panic(b"Not enough allowance");
            }
            account.set_allowance(&escrow_account_id, allowance - amount);
        }

        // Saving the account back to the state.
        self.set_account(&owner_id, &account);

        // Deposit amount to the new owner and save the new account to the state.
        let mut new_account = self.get_account(&new_owner_id);
        new_account.balance += amount;
        self.set_account(&new_owner_id, &new_account);
        self.refund_storage(initial_storage);
    }

    /// Transfer `amount` of tokens from the caller of the contract (`predecessor_id`) to
    /// `new_owner_id`.
    /// Act the same was as `transfer_from` with `owner_id` equal to the caller of the contract
    /// (`predecessor_id`).
    /// Requirements:
    /// * Caller of the method has to attach deposit enough to cover storage difference at the
    ///   fixed storage price defined in the contract.
    #[payable]
    pub fn transfer(&mut self, new_owner_id: AccountId, amount: U128) {
        // NOTE: New owner's Account ID checked in transfer_from.
        // Storage fees are also refunded in transfer_from.
        self.transfer_from(env::predecessor_account_id(), new_owner_id, amount);
    }

    /// Returns total supply of tokens.
    pub fn get_total_supply(&self) -> U128 {
        self.total_supply.into()
    }

    /// Returns balance of the `owner_id` account.
    pub fn get_balance(&self, owner_id: AccountId) -> U128 {
        self.get_account(&owner_id).balance.into()
    }

    /// Returns current allowance of `escrow_account_id` for the account of `owner_id`.
    ///
    /// NOTE: Other contracts should not rely on this information, because by the moment a contract
    /// receives this information, the allowance may already be changed by the owner.
    /// So this method should only be used on the front-end to see the current allowance.
    pub fn get_allowance(&self, owner_id: AccountId, escrow_account_id: AccountId) -> U128 {
        assert!(
            env::is_valid_account_id(escrow_account_id.as_bytes()),
            "Escrow account ID is invalid"
        );
        self.get_account(&owner_id)
            .get_allowance(&escrow_account_id)
            .into()
    }
}

impl FungibleToken {
    /// Helper method to get the account details for `owner_id`.
    fn get_account(&self, owner_id: &AccountId) -> Account {
        assert!(
            env::is_valid_account_id(owner_id.as_bytes()),
            "Owner's account ID is invalid"
        );
        let account_hash = env::sha256(owner_id.as_bytes());
        self.accounts
            .get(&account_hash)
            .unwrap_or_else(|| Account::new(account_hash))
    }

    /// Helper method to set the account details for `owner_id` to the state.
    fn set_account(&mut self, owner_id: &AccountId, account: &Account) {
        let account_hash = env::sha256(owner_id.as_bytes());
        if account.balance > 0 || !account.allowances.is_empty() {
            self.accounts.insert(&account_hash, &account);
        } else {
            self.accounts.remove(&account_hash);
        }
    }

    fn refund_storage(&self, initial_storage: StorageUsage) {
        let current_storage = env::storage_usage();
        let attached_deposit = env::attached_deposit();
        let refund_amount = if current_storage > initial_storage {
            let required_deposit =
                Balance::from(current_storage - initial_storage) * STORAGE_PRICE_PER_BYTE;
            assert!(
                required_deposit <= attached_deposit,
                "The required attached deposit is {}, but the given attached deposit is is {}",
                required_deposit,
                attached_deposit,
            );
            attached_deposit - required_deposit
        } else {
            attached_deposit
                + Balance::from(initial_storage - current_storage) * STORAGE_PRICE_PER_BYTE
        };
        if refund_amount > 0 {
            env::log(format!("Refunding {} tokens for storage", refund_amount).as_bytes());
            Promise::new(env::predecessor_account_id()).transfer(refund_amount);
        }
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

    fn get_context(predecessor_account_id: AccountId) -> VMContext {
        VMContext {
            current_account_id: alice(),
            signer_account_id: bob(),
            signer_account_pk: vec![0, 1, 2],
            predecessor_account_id,
            input: vec![],
            block_index: 0,
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
    fn test_initialize_new_token() {
        let context = get_context(carol());
        testing_env!(context);
        let total_supply = 1_000_000_000_000_000u128;
        let contract = FungibleToken::new(bob(), total_supply.into());
        assert_eq!(contract.get_total_supply().0, total_supply);
        assert_eq!(contract.get_balance(bob()).0, total_supply);
    }

    #[test]
    #[should_panic]
    fn test_initialize_new_token_twice_fails() {
        let context = get_context(carol());
        testing_env!(context);
        let total_supply = 1_000_000_000_000_000u128;
        {
            let _contract = FungibleToken::new(bob(), total_supply.into());
        }
        FungibleToken::new(bob(), total_supply.into());
    }

    #[test]
    fn test_transfer_to_a_different_account_works() {
        let mut context = get_context(carol());
        testing_env!(context.clone());
        let total_supply = 1_000_000_000_000_000u128;
        let mut contract = FungibleToken::new(carol(), total_supply.into());
        context.storage_usage = env::storage_usage();

        context.attached_deposit = 1000 * STORAGE_PRICE_PER_BYTE;
        testing_env!(context.clone());
        let transfer_amount = total_supply / 3;
        contract.transfer(bob(), transfer_amount.into());
        context.storage_usage = env::storage_usage();
        context.account_balance = env::account_balance();

        context.is_view = true;
        context.attached_deposit = 0;
        testing_env!(context.clone());
        assert_eq!(
            contract.get_balance(carol()).0,
            (total_supply - transfer_amount)
        );
        assert_eq!(contract.get_balance(bob()).0, transfer_amount);
    }

    #[test]
    #[should_panic(expected = "The new owner should be different from the current owner")]
    fn test_transfer_to_self_fails() {
        let mut context = get_context(carol());
        testing_env!(context.clone());
        let total_supply = 1_000_000_000_000_000u128;
        let mut contract = FungibleToken::new(carol(), total_supply.into());
        context.storage_usage = env::storage_usage();

        context.attached_deposit = 1000 * STORAGE_PRICE_PER_BYTE;
        testing_env!(context.clone());
        let transfer_amount = total_supply / 3;
        contract.transfer(carol(), transfer_amount.into());
    }

    #[test]
    #[should_panic(expected = "Can not increment allowance for yourself")]
    fn test_increment_allowance_to_self_fails() {
        let mut context = get_context(carol());
        testing_env!(context.clone());
        let total_supply = 1_000_000_000_000_000u128;
        let mut contract = FungibleToken::new(carol(), total_supply.into());
        context.attached_deposit = STORAGE_PRICE_PER_BYTE * 1000;
        testing_env!(context.clone());
        contract.inc_allowance(carol(), (total_supply / 2).into());
    }

    #[test]
    #[should_panic(expected = "Can not decrement allowance for yourself")]
    fn test_decrement_allowance_to_self_fails() {
        let mut context = get_context(carol());
        testing_env!(context.clone());
        let total_supply = 1_000_000_000_000_000u128;
        let mut contract = FungibleToken::new(carol(), total_supply.into());
        context.attached_deposit = STORAGE_PRICE_PER_BYTE * 1000;
        testing_env!(context.clone());
        contract.dec_allowance(carol(), (total_supply / 2).into());
    }

    #[test]
    fn test_decrement_allowance_after_allowance_was_saturated() {
        let mut context = get_context(carol());
        testing_env!(context.clone());
        let total_supply = 1_000_000_000_000_000u128;
        let mut contract = FungibleToken::new(carol(), total_supply.into());
        context.attached_deposit = STORAGE_PRICE_PER_BYTE * 1000;
        testing_env!(context.clone());
        contract.dec_allowance(bob(), (total_supply / 2).into());
        assert_eq!(contract.get_allowance(carol(), bob()), 0.into())
    }

    #[test]
    fn test_increment_allowance_does_not_overflow() {
        let mut context = get_context(carol());
        testing_env!(context.clone());
        let total_supply = std::u128::MAX;
        let mut contract = FungibleToken::new(carol(), total_supply.into());
        context.attached_deposit = STORAGE_PRICE_PER_BYTE * 1000;
        testing_env!(context.clone());
        contract.inc_allowance(bob(), total_supply.into());
        contract.inc_allowance(bob(), total_supply.into());
        assert_eq!(
            contract.get_allowance(carol(), bob()),
            std::u128::MAX.into()
        )
    }

    #[test]
    #[should_panic(
        expected = "The required attached deposit is 33100000000000000000000, but the given attached deposit is is 0"
    )]
    fn test_increment_allowance_with_insufficient_attached_deposit() {
        let mut context = get_context(carol());
        testing_env!(context.clone());
        let total_supply = 1_000_000_000_000_000u128;
        let mut contract = FungibleToken::new(carol(), total_supply.into());
        context.attached_deposit = 0;
        testing_env!(context.clone());
        contract.inc_allowance(bob(), (total_supply / 2).into());
    }

    #[test]
    fn test_carol_escrows_to_bob_transfers_to_alice() {
        // Acting as carol
        let mut context = get_context(carol());
        testing_env!(context.clone());
        let total_supply = 1_000_000_000_000_000u128;
        let mut contract = FungibleToken::new(carol(), total_supply.into());
        context.storage_usage = env::storage_usage();

        context.is_view = true;
        testing_env!(context.clone());
        assert_eq!(contract.get_total_supply().0, total_supply);

        let allowance = total_supply / 3;
        let transfer_amount = allowance / 3;
        context.is_view = false;
        context.attached_deposit = STORAGE_PRICE_PER_BYTE * 1000;
        testing_env!(context.clone());
        contract.inc_allowance(bob(), allowance.into());
        context.storage_usage = env::storage_usage();
        context.account_balance = env::account_balance();

        context.is_view = true;
        context.attached_deposit = 0;
        testing_env!(context.clone());
        assert_eq!(contract.get_allowance(carol(), bob()).0, allowance);

        // Acting as bob now
        context.is_view = false;
        context.attached_deposit = STORAGE_PRICE_PER_BYTE * 1000;
        context.predecessor_account_id = bob();
        testing_env!(context.clone());
        contract.transfer_from(carol(), alice(), transfer_amount.into());
        context.storage_usage = env::storage_usage();
        context.account_balance = env::account_balance();

        context.is_view = true;
        context.attached_deposit = 0;
        testing_env!(context.clone());
        assert_eq!(
            contract.get_balance(carol()).0,
            total_supply - transfer_amount
        );
        assert_eq!(contract.get_balance(alice()).0, transfer_amount);
        assert_eq!(
            contract.get_allowance(carol(), bob()).0,
            allowance - transfer_amount
        );
    }

    #[test]
    fn test_carol_escrows_to_bob_locks_and_transfers_to_alice() {
        // Acting as carol
        let mut context = get_context(carol());
        testing_env!(context.clone());
        let total_supply = 1_000_000_000_000_000u128;
        let mut contract = FungibleToken::new(carol(), total_supply.into());
        context.storage_usage = env::storage_usage();

        context.is_view = true;
        testing_env!(context.clone());
        assert_eq!(contract.get_total_supply().0, total_supply);

        let allowance = total_supply / 3;
        let transfer_amount = allowance / 3;
        context.is_view = false;
        context.attached_deposit = STORAGE_PRICE_PER_BYTE * 1000;
        testing_env!(context.clone());
        contract.inc_allowance(bob(), allowance.into());
        context.storage_usage = env::storage_usage();
        context.account_balance = env::account_balance();

        context.is_view = true;
        context.attached_deposit = 0;
        testing_env!(context.clone());
        assert_eq!(contract.get_allowance(carol(), bob()).0, allowance);
        assert_eq!(contract.get_balance(carol()).0, total_supply);

        // Acting as bob now
        context.is_view = false;
        context.attached_deposit = STORAGE_PRICE_PER_BYTE * 1000;
        context.predecessor_account_id = bob();
        testing_env!(context.clone());
        contract.transfer_from(carol(), alice(), transfer_amount.into());
        context.storage_usage = env::storage_usage();
        context.account_balance = env::account_balance();

        context.is_view = true;
        context.attached_deposit = 0;
        testing_env!(context.clone());
        assert_eq!(
            contract.get_balance(carol()).0,
            (total_supply - transfer_amount)
        );
        assert_eq!(contract.get_balance(alice()).0, transfer_amount);
        assert_eq!(
            contract.get_allowance(carol(), bob()).0,
            allowance - transfer_amount
        );
    }

    #[test]
    fn test_self_allowance_set_for_refund() {
        let mut context = get_context(carol());
        testing_env!(context.clone());
        let total_supply = 1_000_000_000_000_000u128;
        let mut contract = FungibleToken::new(carol(), total_supply.into());
        context.storage_usage = env::storage_usage();

        let initial_balance = context.account_balance;
        let initial_storage = context.storage_usage;
        context.attached_deposit = STORAGE_PRICE_PER_BYTE * 1000;
        testing_env!(context.clone());
        contract.inc_allowance(bob(), (total_supply / 2).into());
        context.storage_usage = env::storage_usage();
        context.account_balance = env::account_balance();
        assert_eq!(
            context.account_balance,
            initial_balance
                + Balance::from(context.storage_usage - initial_storage) * STORAGE_PRICE_PER_BYTE
        );

        let initial_balance = context.account_balance;
        let initial_storage = context.storage_usage;
        testing_env!(context.clone());
        context.attached_deposit = 0;
        testing_env!(context.clone());
        contract.dec_allowance(bob(), (total_supply / 2).into());
        context.storage_usage = env::storage_usage();
        context.account_balance = env::account_balance();
        assert!(context.storage_usage < initial_storage);
        assert!(context.account_balance < initial_balance);
        assert_eq!(
            context.account_balance,
            initial_balance
                - Balance::from(initial_storage - context.storage_usage) * STORAGE_PRICE_PER_BYTE
        );
    }
}
*/