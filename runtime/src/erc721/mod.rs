/// A runtime module template with necessary imports

/// Feel free to remove or edit this file as needed.
/// If you change the name of this file, make sure to update its references in runtime/src/lib.rs
/// If you remove this file, you can remove those references

/// For more guidance on Substrate modules, see the example module
/// https://github.com/paritytech/substrate/blob/master/srml/example/src/lib.rs

/// Collateral: functions for dealing with a collateralizable nonfungible token

use support::{
    decl_module, decl_storage, decl_event, 
    ensure, 
    StorageValue, StorageMap,
    dispatch::Result};
use system::ensure_signed;

// @nczhu: added
use parity_codec::Encode; // serialization and deserialization codec for simple marshalling.
use runtime_primitives::traits::{Hash, Zero};
use rstd::prelude::*;

#[cfg(test)] //tells compiler to compile based on "test" flag. i.e. its a test.
mod test;

/// The module's configuration trait.
pub trait Trait: balances::Trait {

	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_event!(
    pub enum Event<T>
    where
        <T as system::Trait>::AccountId,
        <T as system::Trait>::Hash
    {
        Transfer(Option<AccountId>, Option<AccountId>, Hash),
        Approval(AccountId, AccountId, Hash),
        ApprovalForAll(AccountId, AccountId, bool),
    }
);

decl_storage! {
    trait Store for Module<T: Trait> as ERC721Storage {
        // Start ERC721 : Storage & Getters //
        OwnedTokensCount get(balance_of): map T::AccountId => u64;
        TokenOwner get(owner_of): map T::Hash => Option<T::AccountId>;
        TokenApprovals get(get_approved): map T::Hash => Option<T::AccountId>;
        OperatorApprovals get(is_approved_for_all): map (T::AccountId, T::AccountId) => bool;
        // End ERC721 : Storage & Getters //

        // Start ERC721 : Enumerable : Storage & Getters //
        TotalSupply get(total_supply): u64;
        AllTokens get(token_by_index): map u64 => T::Hash;
        AllTokensIndex: map T::Hash => u64;
        OwnedTokens get(token_of_owner_by_index): map (T::AccountId, u64) => T::Hash;
        OwnedTokensIndex: map T::Hash => u64;
        // Start ERC721 : Enumerable : Storage & Getters //

        // @nczhu: Mapping of a token_id to whats its collateralized for
        Escrow get(is_escrowed): map T::Hash => T::Hash;
        // TODO, make escrows enumerable? or associated with teh people?
        
        // Not a part of the ERC721 specification, but used in random token generation
        Nonce: u64;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {

        fn deposit_event<T>() = default;

        // Start ERC721 : Public Functions //
        pub fn approve(origin, to: T::AccountId, token_id: T::Hash) -> Result {
            let sender = ensure_signed(origin)?;
            let owner = match Self::owner_of(token_id) {
                Some(c) => c,
                None => return Err("No owner for this token"),
            };

            ensure!(to != owner, "Owner is implicitly approved");
            ensure!(sender == owner || Self::is_approved_for_all((owner.clone(), sender.clone())), "You are not allowed to approve for this token");

            <TokenApprovals<T>>::insert(&token_id, &to);

            Self::deposit_event(RawEvent::Approval(owner, to, token_id));

            Ok(())
        }

        pub fn set_approval_for_all(origin, to: T::AccountId, approved: bool) -> Result {
            let sender = ensure_signed(origin)?;
            ensure!(to != sender, "You are already implicity approved for your own actions");
            <OperatorApprovals<T>>::insert((sender.clone(), to.clone()), approved);

            Self::deposit_event(RawEvent::ApprovalForAll(sender, to, approved));

            Ok(())
        }

        // transfer_from will transfer to addresses even without a balance
        pub fn transfer_from(origin, from: T::AccountId, to: T::AccountId, token_id: T::Hash) -> Result {
            let sender = ensure_signed(origin)?;
            ensure!(Self::_is_approved_or_owner(sender, token_id), "You do not own this token");

            Self::_transfer_from(from, to, token_id)?;

            Ok(())
        }

        // safe_transfer_from checks that the recieving address has enough balance to satisfy the ExistentialDeposit
        // This is not quite what it does on Ethereum, but in the same spirit...
        pub fn safe_transfer_from(origin, from: T::AccountId, to: T::AccountId, token_id: T::Hash) -> Result {
            let to_balance = <balances::Module<T>>::free_balance(&to);
            ensure!(!to_balance.is_zero(), "'to' account does not satisfy the `ExistentialDeposit` requirement");

            Self::transfer_from(origin, from, to, token_id)?;

            Ok(())
        }
        // End ERC721 : Public Functions //

        // Not part of ERC721, but allows you to play with the runtime
        pub fn create_token(origin) -> Result {
            let sender = ensure_signed(origin)?;
            let nonce = <Nonce<T>>::get();
            let random_hash = (<system::Module<T>>::random_seed(), &sender, nonce).using_encoded(<T as system::Trait>::Hashing::hash);
            
            Self::_mint(sender, random_hash)?;
            <Nonce<T>>::mutate(|n| *n += 1);

            Ok(())
        }

        // User can collateralize n token for any reason (referenced by a hash ptr)
        // After that, the token is no longer "owned" by the user
        // Later: assume you can collateralize by a specific token ID
        pub fn collateralize_token(origin, n: u64, reason: T::Hash) {
            // "Locks" token from leaving 
            let sender = ensure_signed(origin)?;

            Self::_put_in_escrow(sender, n, reason)?;

            // TODO: emit some event here
        }

        // Only callable by the system
        // Gives collateralized token to an account
        // Can be debtor, or creditor
        // Only collable by the system
        pub fn uncollateralize_token(to: T::AccountId, token_id: T::Hash) {

        }

    }
}

impl<T: Trait> Module<T> {

    fn _put_in_escrow(sender: T::AccountId, n: u64, reason: T::Hash) -> Result {
        let token_balance = Self::balance_of(&sender);
        
        // check n isn't some insane number
        ensure!(token_balance >= n, "Not enough tokens to collateralize");

        for i in 0..n {                
            let token_id = Self::token_of_owner_by_index((sender.clone(), i));
            ensure!(Self::_is_approved_or_owner(sender.clone(), token_id), "You do not own this token");
        
            let new_balance = match token_balance.checked_sub(1) {
                Some (c) => c,
                None => return Err("Collateralizing causes underflow of token balance"),
            };

            Self::_remove_token_from_owner_enumeration(sender.clone(), token_id)?;
            Self::_clear_approval(token_id)?;
            <OwnedTokensCount<T>>::insert(&sender, new_balance);

            //Add to escrow
            <Escrow<T>>::insert(token_id, reason);
        }

        Ok(())
    }

    // Start ERC721 : Internal Functions //
    fn _exists(token_id: T::Hash) -> bool {
        return <TokenOwner<T>>::exists(token_id);
    }

    fn _is_approved_or_owner(spender: T::AccountId, token_id: T::Hash) -> bool {
        let owner = Self::owner_of(token_id);
        let approved_user = Self::get_approved(token_id);

        let approved_as_owner = match owner {
            Some(ref o) => o == &spender,
            None => false,
        };

        let approved_as_delegate = match owner {
            Some(d) => Self::is_approved_for_all((d, spender.clone())),
            None => false,
        };

        let approved_as_user = match approved_user {
            Some(u) => u == spender,
            None => false,
        };

        return approved_as_owner || approved_as_user || approved_as_delegate
    }

    fn _mint(to: T::AccountId, token_id: T::Hash) -> Result {
        ensure!(!Self::_exists(token_id), "Token already exists");

        let balance_of = Self::balance_of(&to);

        let new_balance_of = match balance_of.checked_add(1) {
            Some(c) => c,
            None => return Err("Overflow adding a new token to account balance"),
        };

        // Writing to storage begins here
        Self::_add_token_to_all_tokens_enumeration(token_id)?;
        Self::_add_token_to_owner_enumeration(to.clone(), token_id)?;

        <TokenOwner<T>>::insert(token_id, &to);
        <OwnedTokensCount<T>>::insert(&to, new_balance_of);

        Self::deposit_event(RawEvent::Transfer(None, Some(to), token_id));

        Ok(())
    }

    fn _burn(token_id: T::Hash) -> Result {
        let owner = match Self::owner_of(token_id) {
            Some(c) => c,
            None => return Err("No owner for this token"),
        };

        let balance_of = Self::balance_of(&owner);

        let new_balance_of = match balance_of.checked_sub(1) {
            Some(c) => c,
            None => return Err("Underflow subtracting a token to account balance"),
        };

        // Writing to storage begins here
        Self::_remove_token_from_all_tokens_enumeration(token_id)?;
        Self::_remove_token_from_owner_enumeration(owner.clone(), token_id)?;
        <OwnedTokensIndex<T>>::remove(token_id);

        Self::_clear_approval(token_id)?;

        <OwnedTokensCount<T>>::insert(&owner, new_balance_of);
        <TokenOwner<T>>::remove(token_id);

        Self::deposit_event(RawEvent::Transfer(Some(owner), None, token_id));

        Ok(())
    }

    fn _transfer_from(from: T::AccountId, to: T::AccountId, token_id: T::Hash) -> Result {
        let owner = match Self::owner_of(token_id) {
            Some(c) => c,
            None => return Err("No owner for this token"),
        };

        ensure!(owner == from, "'from' account does not own this token");

        let balance_of_from = Self::balance_of(&from);
        let balance_of_to = Self::balance_of(&to);

        let new_balance_of_from = match balance_of_from.checked_sub(1) {
            Some (c) => c,
            None => return Err("Transfer causes underflow of 'from' token balance"),
        };

        let new_balance_of_to = match balance_of_to.checked_add(1) {
            Some(c) => c,
            None => return Err("Transfer causes overflow of 'to' token balance"),
        };

        // Writing to storage begins here
        Self::_remove_token_from_owner_enumeration(from.clone(), token_id)?;
        Self::_add_token_to_owner_enumeration(to.clone(), token_id)?;
        
        Self::_clear_approval(token_id)?;
        <OwnedTokensCount<T>>::insert(&from, new_balance_of_from);
        <OwnedTokensCount<T>>::insert(&to, new_balance_of_to);
        <TokenOwner<T>>::insert(&token_id, &to);

        Self::deposit_event(RawEvent::Transfer(Some(from), Some(to), token_id));
        
        Ok(())
    }

    fn _clear_approval(token_id: T::Hash) -> Result{
        <TokenApprovals<T>>::remove(token_id);

        Ok(())
    }
    // End ERC721 : Internal Functions //

    // Start ERC721 : Enumerable : Internal Functions //
    fn _add_token_to_owner_enumeration(to: T::AccountId, token_id: T::Hash) -> Result {
        let new_token_index = Self::balance_of(&to);

        <OwnedTokensIndex<T>>::insert(token_id, new_token_index);
        <OwnedTokens<T>>::insert((to, new_token_index), token_id);

        Ok(())
    }

    fn _add_token_to_all_tokens_enumeration(token_id: T::Hash) -> Result {
        let total_supply = Self::total_supply();

        // Should never fail since overflow on user balance is checked before this
        let new_total_supply = match total_supply.checked_add(1) {
            Some (c) => c,
            None => return Err("Overflow when adding new token to total supply"),
        };

        let new_token_index = total_supply;

        <AllTokensIndex<T>>::insert(token_id, new_token_index);
        <AllTokens<T>>::insert(new_token_index, token_id);
        <TotalSupply<T>>::put(new_total_supply);

        Ok(())
    }

    fn _remove_token_from_owner_enumeration(from: T::AccountId, token_id: T::Hash) -> Result {
        let balance_of_from = Self::balance_of(&from);

        // Should never fail because same check happens before this call is made
        let last_token_index = match balance_of_from.checked_sub(1) {
            Some (c) => c,
            None => return Err("Transfer causes underflow of 'from' token balance"),
        };
        
        let token_index = <OwnedTokensIndex<T>>::get(token_id);

        if token_index != last_token_index {
            let last_token_id = <OwnedTokens<T>>::get((from.clone(), last_token_index));
            <OwnedTokens<T>>::insert((from.clone(), token_index), last_token_id);
            <OwnedTokensIndex<T>>::insert(last_token_id, token_index);
        }

        <OwnedTokens<T>>::remove((from, last_token_index));
        // OpenZeppelin does not do this... should I?
        <OwnedTokensIndex<T>>::remove(token_id);

        Ok(())
    }

    fn _remove_token_from_all_tokens_enumeration(token_id: T::Hash) -> Result {
        let total_supply = Self::total_supply();

        // Should never fail because balance of underflow is checked before this
        let new_total_supply = match total_supply.checked_sub(1) {
            Some(c) => c,
            None => return Err("Underflow removing token from total supply"),
        };

        let last_token_index = new_total_supply;

        let token_index = <AllTokensIndex<T>>::get(token_id);

        let last_token_id = <AllTokens<T>>::get(last_token_index);

        <AllTokens<T>>::insert(token_index, last_token_id);
        <AllTokensIndex<T>>::insert(last_token_id, token_index);

        <AllTokens<T>>::remove(last_token_index);
        <AllTokensIndex<T>>::remove(token_id);

        <TotalSupply<T>>::put(new_total_supply);

        Ok(())
    }
    // End ERC721 : Enumerable : Internal Functions //
}

// TESTING

// #[cfg(test)]
// mod tests {
//     use super::*; // just rust test layout

//     use primitives::{H256, Blake2Hasher}; //called substrate_primitives as primitives
//     use runtime_primitives::{
//         BuildStorage,
//         traits::{IdentityLookup, BlakeTwo256}, // Test wrapper for this specific type/ looks up the identity; returns Result
//         testing::{Digest, DigestItem, Header}
//     };
//     use support::{impl_outer_origin};
//     use runtime_io::{TestExternalities}; //to set up the block for tests

//     // impl outer origin
//     impl_outer_origin! {
//         pub enum Origin for ERCTest {}
//     }

//     // 2. Set up mock runtime
//     #[derive(Clone, PartialEq, Eq, Debug)]
//     pub struct ERCTest; // can call thsi anything Runtime or Test...

//     impl system::Trait for ERCTest {
//         type Origin = Origin;  // these types are declared in the module traits, so they must be ste
//         type Index = u64;       //hack it to just be a u64 int (later: double check the actual type?)
//         type BlockNumber = u64;
//         type Hash = H256;
//         type Hashing = BlakeTwo256; //from runtime_primitives::traits::blaketwo256
//         type Digest = Digest;
//         type AccountId = u64;
//         type Lookup = IdentityLookup<Self::AccountId>;
//         type Header = Header;
//         type Event = ();
//         type Log = DigestItem;
//     }

//     impl balances::Trait for ERCTest {
//         type Balance = u64; //hack it to be a u64 figure
//         type OnFreeBalanceZero = (); //overrides. () is to use the default. 
//         type OnNewAccount = ();
//         type TransactionPayment = ();
//         type TransferPayment = ();
//         type DustRemoval = ();
//         type Event = ();
//     }

//     // impl the types for this particular trait!
//     impl Trait for ERCTest{
//         type Event = ();
//     }
    
//     type Erc721 = Module<ERCTest>;

//     fn new_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
//         system::GenesisConfig::<ERCTest>::default().build_storage().unwrap().0.into()
//     }

//     // TODO: write the first test
//     #[test]
//     fn alice_can_create_token() {
//         // let mut ext = TestExternalities::<Blake2Hasher>::default();
//         assert!(true);
//     }
// }







