/* Used / Learned: 
	- Currency trait
	- Moment trait
	
*/

use support::{decl_module, decl_storage, decl_event, StorageValue, dispatch::Result};
use system::ensure_signed;

// import currency trait, to get access to "ensure can withdraw", everything for balance. 
use support::traits::{Currency}; // Other avail traits lockablecurrency, onfreebalancezero, etc.


pub trait Trait: system::Trait {
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}


// Asset owners can create a DebtRequest to ask for a traunche of Balance
pub struct DebtRequest<Hash, AccountId, Currency, Moment> {   //Needs the blake2 Hash trait
	id: Hash,								// DebtRequestId
	requestor: AccountId,		// Account that will go in debt
	beneficiary: AccountId,	// Recipient of Balance
	amount: Currency,				// Amount of loan
	expiry: Moment,					// Duration of debtRequest
	collateralized: bool,		// Defaults to false, true upon collaterlization
}

decl_storage! {
	trait Store for Module<T: Trait> as Collateral {
		
		DebtRequests get(get_debt_order): map T::Hash => DebtRequest<T::Hash>;


	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		
		fn deposit_event<T>() = default;

		// DEBTOR:
		// pub fn create_debt_request

		// pub fn collateralize_debt_request


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
