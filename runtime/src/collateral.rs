/* Used / Learned: 
	- Currency trait
	- Moment trait
*/

use crate::erc721;		// our ERC 721 implementation

use support::{decl_module, decl_storage, decl_event, 
	StorageValue, 
	dispatch::Result, 
	ensure //ensure is a macro from support/src/lib
	}; 
use system::ensure_signed;
use parity_codec::{Encode, Decode}; //enables #[derive(Decode)] Why? what is it

// import currency trait, to get access to "ensure_can_withdraw", everything for balance. 
// use support::traits::{Currency}; // Other avail traits lockablecurrency, onfreebalancezero, etc.

// Currency trait, needs this internal type (in order to input things into fn signatures inputs: e.g. #[compact] value: BalanceOf<T>
// type BalanceOf<T> = <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::Balance;

// This module's traits
// things used by fns in dclr modules need to be included in here.
// dont be redudant , i.e. timestamp includes system, and erc721 includes balances, so can omit here
pub trait Trait: timestamp::Trait + erc721::Trait {
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

// Asset owners can create a DebtRequest to ask for a traunche of Balance
#[derive(Encode, Decode, Default, Clone, PartialEq)] //these are custom traits required by all structs (some traits forenums)
#[cfg_attr(feature = "std", derive(Debug))] // attr provided by rust compiler. uses derive(debug) trait when in std mode
pub struct DebtRequest<Hash, AccountId, Balance, Moment> {   //Needs the blake2 Hash trait
	id: Hash,								// DebtRequestId
	requestor: AccountId,		// Account that will go in debt
	beneficiary: AccountId,	// Recipient of Balance
	amount: Balance,				// Amount of loan
	expiry: Moment,					// Duration of debtRequest
	collateralized: bool,		// Defaults to false, true upon collaterlization
}

// Status of the collateralized debt
#[derive(Encode)] //Encode, Deco req for enums
#[cfg_attr(feature = "std", derive(Debug))]
enum OrderStatus {
	Expired,		// loan is never filled, expired
	Open, 			// looking for issuance
	Active, 		// loan issued
	Repaid, 		// closed, repaid
	Default,		// unpaid, collat seized
}

// Created upon successful collateralization
#[derive(Encode, Decode, Default)] //Default is only for structs
#[cfg_attr(feature = "std", derive(Debug))]
pub struct DebtOrder<Hash, AccountId, Moment> {
	id: Hash, 
	requestId: Hash,				// corresponding DebtRequestId
	status: OrderStatus,		// status of this order
	creditor: AccountId,
	// Input by debtor
	expiry: Moment,					// Due date for all payment
	// TODO collateral of tokens...  // a fixed length array of tokens collateralized in system escrow
}

// decode?
decl_storage! {
	trait Store for Module<T: Trait> as CollateralStorage {
		
		// TODO later abstrate T::Hash into generic vars, so its not so long?
		// doesn't get deleted
		DebtRequests get(get_debt_order): map DebtRequestIndex => DebtRequest<T::Hash, T::AccountId, T::Balance, T::Moment>; //DebtRequest ID to the RequestItself
		DebtRequestCount get(get_total_debt_requests): DebtRequestIndex;  //Alias for u64
		// Escrow get(escrow): //hash of tokenID under management
	}
}

// TYPE ALIASING!!!!
type DebtRequestIndex = u64; //like proposalindex in treasury

// writes functions, make sure to declares all traits where using here in: 
// pub trait Trait
decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		
		fn deposit_event<T>() = default;

		// DEBTOR FUNCTIONS: 
		pub fn create_debt_request(origin, amount: T::Balance, beneficiary: T::AccountId, expiry: T::Moment) { //TODO, change expiry
			let requestor = ensure_signed(origin)?;		//macro, returns sender address

			// TODO initial check
			// TODO check expiry

			let now = <timestamp::Module<T>>::get();
			let id = (<system::Module<T>>::random_seed(), &requestor, now).using_encoded(<T as system::Trait>::Hashing::hash);
			let collateralized = false;

			// TODO make sure debtrequest doesn't exist already, in case they try to overwrite debt..
			ensure!(!<DebtRequests<T>>::exists(id), "Error: Debt already exists");
			let new_debt_request = DebtRequest {id, requestor, beneficiary, amount, expiry, collateralized};

			// Add new debt request to DebtRequests map
			let i = Self::get_total_debt_requests();
			<DebtRequestCount<T>>::put(i+1); //increment total count by 1
			<DebtRequests<T>>::insert(i, new_debt_request);
			
			// emit the event TODO: figure out how to emit debt details later
			Self::deposit_event(RawEvent::DebtRequestCreated(9, &requestor));
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

// impl<T: Trait> Module<T> {
// 	_create_debt_request()
// }

decl_event!(
	pub enum Event<T> where
		AccountId = <T as system::Trait>::AccountId 
	{	
		DebtRequestCreated(u32, AccountId),
	}
);

// ==================================================================
// TESTS

#[cfg(test)]
mod tests {
	use super::*;

	use runtime_io::with_externalities;
	use primitives::{H256, Blake2Hasher};
	use support::{impl_outer_origin, 
		assert_ok, //assert_noop, assert_eq_uvec
	};
	use runtime_primitives::{
		BuildStorage,
		traits::{BlakeTwo256, IdentityLookup},
		testing::{Digest, DigestItem, Header}
	};

	impl_outer_origin! {
		pub enum Origin for Test {}
	}

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

	impl balances::Trait for Test {
		type Balance = u64;			// using u64 to mock balance
		type OnFreeBalanceZero = ();
		type OnNewAccount = ();
		type Event = ();
		type TransactionPayment = ();
		type TransferPayment = ();
		type DustRemoval = ();
	}
	
	impl timestamp::Trait for Test {
		type Moment = u64;
		type OnTimestampSet = ();
	}

	// this module, implements the traits.
	impl Trait for Test {
		type Event = ();
		// any custom traits from this module?
	}

	// shorthand?
	type Collateral = Module<Test>;

	// This function basically just builds a genesis storage key/value store according to
	// our desired mockup.
	fn new_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
		system::GenesisConfig::<Test>::default().build_storage().unwrap().0.into()
	}

	// #[test]
	// fn asdf() {
	// 	with_externalities(&mut new_test_ext(), || {
	// 		assert_eq!(1,1);
	// 		assert_ok!(Collateral::create_debt_request(
	// 			Origin::signed(1),
	// 			// amount: T::Balance, beneficiary: T::AccountId, expiry: u64
	// 			));
	// 	});
	// }

	#[test]
	fn it_works_for_default_value() {
		with_externalities(&mut new_test_ext(), || {
			// Just a dummy test for the dummy funtion `do_something`
			// calling the `do_something` function with a value 42
			assert_ok!(Collateral::do_something(Origin::signed(1), 42));
			// asserting that the stored value is equal to what we stored
			assert_eq!(Collateral::something(), Some(42));
		});
	}
}
