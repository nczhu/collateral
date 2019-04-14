#[cfg(test)] // allows us to compile code, based on the "test" flag.

use super::*;
use support::{impl_outer_origin};
use runtime_io::with_externalities;
use primitives::{H256, Blake2Hasher}; //called substrate_primitives as primitives
use support::{assert_ok, assert_noop};
use runtime_primitives::{
    BuildStorage,
    traits::{IdentityLookup, BlakeTwo256}, // Test wrapper for this specific type/ looks up the identity; returns Result
    testing::{Digest, DigestItem, Header}
};

// impl outer origin
impl_outer_origin! {
    pub enum Origin for Test {}
}

// 2. Set up mock runtime
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Test; // can call thsi anything Runtime or Test...

impl system::Trait for Test {
    type Origin = Origin;  // these types are declared in the module traits, so they must be ste
    type Index = u64;       //hack it to just be a u64 int (later: double check the actual type?)
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256; //from runtime_primitives::traits::blaketwo256
    type Digest = Digest;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = ();
    type Log = DigestItem;
}

impl balances::Trait for Test {
    type Balance = u64; //hack it to be a u64 figure
    type OnFreeBalanceZero = (); //overrides. () is to use the default. 
    type OnNewAccount = ();
    type TransactionPayment = ();
    type TransferPayment = ();
    type DustRemoval = ();
    type Event = ();
}

// impl the types for this particular trait!
impl Trait for Test{
    type Event = ();
    // type Currency = balances::Module<Test>;
}

type ERC = Module<Test>;

fn new_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
    system::GenesisConfig::<Test>::default().build_storage().unwrap().0.into()
}

#[test]
#[ignore]
fn can_create_token() {
    // let mut ext = TestExternalities::<Blake2Hasher>::default();
    with_externalities(&mut new_test_ext(), || {
        assert_ok!(ERC::create_token(Origin::signed(0)));
    });
}

#[test]
#[ignore]
fn can_collateralize_token() {
    // let mut ext = TestExternalities::<Blake2Hasher>::default();
    with_externalities(&mut new_test_ext(), || {
        assert_ok!(ERC::create_token(Origin::signed(0)));
        let token_id = ERC::token_by_index(0);
        assert_ok!(ERC::collateralize_token(Origin::signed(0), token_id, H256::zero()));
        // owner shouldn't have token
        assert_eq!(ERC::balance_of(0), 0);
        // token shouldn't have owner
        assert!(ERC::owner_of(ERC::token_by_index(0)).is_none());
        // ERC::owner_of(ERC::token_by_index(0))
        // owner shouldn't have token
        assert_eq!(ERC::total_supply(), 1); //total supply shouldn't change
    });
}
