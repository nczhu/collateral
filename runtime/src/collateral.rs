/* Used / Learned: 
	- Currency trait
	- Moment trait
*/

use crate::erc721;		// our ERC 721 implementation
use support::{decl_module, decl_storage, decl_event, 
	StorageValue, StorageMap,
	//dispatch::Result, 
	ensure //ensure is a macro from support/src/lib
	}; 
use system::ensure_signed;
use parity_codec::{Encode, Decode}; //enables #[derive(Decode)] Why? what is it
use runtime_primitives::traits::{Hash, StaticLookup}; // Zero, As //static look up is for beneficiary address

// import currency trait, to get access to "ensure_can_withdraw", everything for balance. 
// use support::traits::{Currency}; // Other avail traits lockablecurrency, onfreebalancezero, etc.

// Currency trait, needs this internal type (in order to input things into fn signatures inputs: e.g. #[compact] value: BalanceOf<T>
// type BalanceOf<T> = <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::Balance;

// This module's traits
// things used by fns in dclr modules need to be included in here.
// dont be redudant , i.e. timestamp includes system, and erc721 includes balances, so can omit here
pub trait Trait: timestamp::Trait + erc721::Trait { //+ erc721::Trait 
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

/// Status of the collateralized debt
#[derive(Encode, Decode, Clone, Copy, PartialEq)] //Encode, Deco req for enums, #[cfg_attr(feature = "std", derive(Debug))]
#[cfg_attr(feature = "std", derive(Debug))]
pub enum OrderStatus {
	/// loan is never filled, expired
	Expired,		/// loan is never filled, expired
	Open, 			// looking for issuance
	Active, 		// loan issued
	Repaid, 		// closed, repaid
	Defaulted,		// unpaid, collat seized
}

// Otherwise Rustc will Complain that i dont have default for orderstatus
impl Default for OrderStatus {
	fn default() -> OrderStatus { OrderStatus::Open }
}

// Created upon successful collateralization
#[derive(Encode, Decode, Default, Clone, PartialEq)] //Default is only for structs
#[cfg_attr(feature = "std", derive(Debug))]
pub struct DebtOrder<Hash, AccountId, Moment> {
	id: Hash, 
	request_id: Hash,				// corresponding DebtRequestId
	status: OrderStatus,		// status of this order
	creditor: AccountId,
	// Input by debtor
	// TODO: expiry should be Option<Moment>. so it defaults to None
	expiry: Moment,					// unless moment already defaults to None?
	// TODO collateral of tokens...  // a fixed length array of tokens collateralized in system escrow
}

// decode?
decl_storage! {
	trait Store for Module<T: Trait> as CollateralStorage {
		
		// TODO later abstrate T::Hash into generic vars, so its not so long?
		// doesn't get deleted
		DebtRequests get(get_debt_order): map T::Hash => DebtRequest<T::Hash, T::AccountId, T::Balance, T::Moment>; //DebtRequest ID to the RequestItself
		// [0, 0x...] [1, 0x...]
		DebtRequestIndexToId get(get_debt_request_id): map DebtRequestIndex => T::Hash;
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
		pub fn create_debt_request(
				origin, 
				amount: T::Balance, 
				beneficiary: <T::Lookup as StaticLookup>::Source, 
				expiry: T::Moment
		) { //TODO, change expiry
			let requestor = ensure_signed(origin)?;		//macro, returns sender address

			// TODO initial check
			// TODO check expiry

			let now = <timestamp::Module<T>>::get();

			// Q: whats the diff btw this and just doing <t as system:: trait> .. etc.
			let id = (<system::Module<T>>::random_seed(), &requestor, now).using_encoded(<T as system::Trait>::Hashing::hash); // use runtime_primitives::hash, its a constnat!
			let collateralized = false;

			let beneficiary = T::Lookup::lookup(beneficiary)?;		//looks up the accountId.

			// TODO make sure debtrequest doesn't exist already, in case they try to overwrite debt..
			ensure!(!<DebtRequests<T>>::exists(&id), "Error: Debt already exists");
			let new_debt_request = DebtRequest {
				id,
				requestor, 
				beneficiary: beneficiary.clone(), 	// can i do this here?!
				amount, 
				expiry, 
				collateralized
			};

			// Add new debt request to DebtRequests map
			let i = Self::get_total_debt_requests();
			<DebtRequestCount<T>>::put(i+1); //increment total count by 1
			<DebtRequestIndexToId<T>>::insert(i, &id);
			<DebtRequests<T>>::insert(id, new_debt_request);
			
			// emit the event TODO: figure out how to emit debt details later
			Self::deposit_event(RawEvent::DebtRequestCreated(9, beneficiary));
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
	pub enum Event<T> where AccountId = <T as system::Trait>::AccountId {
		// Just a dummy event.
		// Event `Something` is declared with a parameter of the type `u32` and `AccountId`
		// To emit this event, we call the deposit funtion, from our runtime funtions
		DebtRequestCreated(u64, AccountId),
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
		assert_ok, // assert_noop, assert_eq_uvec
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
		// We are just aliasing the types with the type, or some easier abstration!!
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

	// code above inherits but still have to declare it in test
	impl balances::Trait for Test {
		type Balance = u64;			// aliasing u64 as "balance" to mock the balance
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

	impl erc721::Trait for Test{
		type Event = ();
	}

	// this module, implements the traits.
	impl Trait for Test {
		type Event = ();
		// any custom traits from this module?
	}

	// Alias the module names
	type Collateral = Module<Test>;
	type Balance = balances::Module<Test>; // what does this alias?

	// This function basically just builds a genesis storage key/value store according to
	// our desired mockup.
	fn new_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
		system::GenesisConfig::<Test>::default().build_storage().unwrap().0.into()
	}


// beneficiary: <T::Lookup as StaticLookup>::Source
// 		) {
// 			let proposer = ensure_signed(origin)?;
// 			let beneficiary = T::Lookup::lookup(beneficiary)?; //returns the Target acct, or error

	#[test]
	fn should_create_debt_request() {
		with_externalities(&mut new_test_ext(), || {
			//       uses the Alias
			assert_ok!(Collateral::create_debt_request(
				Origin::signed(0),
				5,			// amount: T::Balance, 
				1,			// beneficiary, some u64, AccountId
				12345			// expiry: Moment
			));

		});
	}

}
