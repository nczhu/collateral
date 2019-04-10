/* Used / Learned: 
	- Currency trait
	- Moment trait
*/

use crate::erc721;		// our ERC 721 implementation

use support::{decl_module, decl_storage, decl_event, StorageValue, dispatch::Result};
use system::ensure_signed;

// import currency trait, to get access to "ensure_can_withdraw", everything for balance. 
// use support::traits::{Currency}; // Other avail traits lockablecurrency, onfreebalancezero, etc.

// Currency trait, needs this internal type (in order to input things into fn signatures inputs: e.g. #[compact] value: BalanceOf<T>
// type BalanceOf<T> = <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::Balance;

// This module's traits
pub trait Trait: balances::Trait { // need to add timstam and things here?
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

// Status of the debtors request
enum RequestStatus {
	Draft,
	Collateralized,
}

// Asset owners can create a DebtRequest to ask for a traunche of Balance
pub struct DebtRequest<Hash, AccountId, Balance, Moment> {   //Needs the blake2 Hash trait
	id: Hash,								// DebtRequestId
	requestor: AccountId,		// Account that will go in debt
	beneficiary: AccountId,	// Recipient of Balance
	amount: Balance,				// Amount of loan
	expiry: Moment,					// Duration of debtRequest
	collateralized: bool,		// Defaults to false, true upon collaterlization
	status: RequestStatus,	// status of this request
}

// Status of the collateralized debt
enum OrderStatus {
	Expired,		// loan is never filled, expired
	Open, 			// looking for issuance
	Active, 		// loan issued
	Repaid, 		// closed, repaid
	Default,		// unpaid, collat seized
}

// Created upon successful collateralization
pub struct DebtOrder<Hash, AccountId, Moment> {
	id: Hash, 
	requestId: Hash,				// corresponding DebtRequestId
	status: OrderStatus,		// status of this order
	creditor: AccountId,
	// Input by debtor
	expiry: Moment,					// Due date for all payment
	// TODO collateral of tokens...  // a fixed length array of tokens collateralized in system escrow
}

decl_storage! {
	trait Store for Module<T: Trait> as CollateralStorage {
		
		DebtRequests get(get_debt_order): map T::Hash => DebtRequest<T::Hash>; //DebtRequest ID to the RequestItself

		// Escrow get(escrow): //hash of tokenID under management
	}
}

// writes functions, make sure to declares all traits where using here in: 
// pub trait Trait
decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		
		fn deposit_event<T>() = default;

		// DEBTOR FUNCTIONS: 
		pub fn create_debt_request(origin, amount: T::Balance, beneficiary: T::AccountId, expiry: u64) { //TODO, change expiry
			use RequestStatus::Draft;		// have to import to this the enum

			let sender = ensure_signed(origin)?;		//macro, returns sender address

			// TODO do some checks here
			// let id = (<system::Module<T>>::random_seed(), &sender, nonce).using_encoded(<T as system::Trait>::Hashing::hash);
			let now = <timestamp::Module<T>>::get();
			let id = (<system::Module<T>>::random_seed(), &sender, now).using_encoded(<T as system::Trait>::Hashing::hash);

			// TODO make sure debtrequest doesn't exist already, in case they try to overwrite debt..

			let requestor = beneficiary;
			let collateralized = false;
			let status = Draft;

			let new_debt_request = DebtRequest { id, requestor, beneficiary, amount, expiry, collateralized, status};

			// make a hash for this dbt request


		}

		// pub fn collateralize_debt_request (stake n tokens?)

		// pub fn pay_back_debt() // has to be a one time payment...

		// LOANER:
		// pub fn fill_debt_order
		
		// SYSTEM:     		// Removes the need for a trusted contract, etc. system maintains
		// fn return_collateral
		// fn seize_collateral

		// on_intialize().. // 

	}
}

decl_event!(
	pub enum Event<T> where AccountId = <T as system::Trait>::AccountId {
		
		// SomethingStored(u32, AccountId),
	}
);

/// tests for this module
#[cfg(test)]
mod tests {
	use super::*;

	use runtime_io::with_externalities;
	use primitives::{H256, Blake2Hasher};
	use support::{impl_outer_origin, assert_ok};
	use runtime_primitives::{
		BuildStorage,
		traits::{BlakeTwo256, IdentityLookup},
		testing::{Digest, DigestItem, Header}
	};

	impl_outer_origin! {
		pub enum Origin for Test {}
	}


// TESTS

	// For testing the module, we construct most of a mock runtime. This means
	// first constructing a configuration type (`Test`) which `impl`s each of the
	// configuration traits of modules we want to use.
	#[derive(Clone, Eq, PartialEq)]
	pub struct Test;
	impl system::Trait for Test {
		type Origin = Origin;
		type Index = u64;
		type BlockNumber = u64;
		type Hash = H256;
		type Hashing = BlakeTwo256;
		type Digest = Digest;
		type AccountId = u64;
		type Lookup = IdentityLookup<Self::AccountId>;
		type Header = Header;
		type Event = ();
		type Log = DigestItem;
	}
	impl Trait for Test {
		type Event = ();
	}
	type TemplateModule = Module<Test>;

	// This function basically just builds a genesis storage key/value store according to
	// our desired mockup.
	fn new_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
		system::GenesisConfig::<Test>::default().build_storage().unwrap().0.into()
	}

	#[test]
	fn it_works_for_default_value() {
		with_externalities(&mut new_test_ext(), || {
			// Just a dummy test for the dummy funtion `do_something`
			// calling the `do_something` function with a value 42
			assert_ok!(TemplateModule::do_something(Origin::signed(1), 42));
			// asserting that the stored value is equal to what we stored
			assert_eq!(TemplateModule::something(), Some(42));
		});
	}
}
