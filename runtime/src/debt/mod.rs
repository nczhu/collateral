/// A runtime module template with necessary imports

/// Feel free to remove or edit this file as needed.
/// If you change the name of this file, make sure to update its references in runtime/src/lib.rs
/// If you remove this file, you can remove those references


/// For more guidance on Substrate modules, see the example module
/// https://github.com/paritytech/substrate/blob/master/srml/example/src/lib.rs

use support::{decl_module, decl_storage, decl_event, StorageValue, StorageMap, dispatch::Result, ensure};
use system::ensure_signed;
use super::erc721;
use parity_codec::{Encode, Decode}; //enables #[derive(Decode)] Why? what is it
use runtime_primitives::traits::{Hash, StaticLookup}; // Zero, As //static look up is for beneficiary address

use support::traits::Currency;
// replaces Balance type.
type BalanceOf<T> = <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::Balance;

#[cfg(test)]
mod test;

/// The module's configuration trait.
pub trait Trait: timestamp::Trait + erc721::Trait {
	// TODO: Add other types and constants required configure this module.

	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
	type Currency: Currency<Self::AccountId>;
}

#[derive(Encode, Decode, Clone, Copy, Eq, PartialEq)] //Encode, Deco req for enums, #[cfg_attr(feature = "std", derive(Debug))]
#[cfg_attr(feature = "std", derive(Debug))]
pub enum Status {
	Open, 			// (in draft, just collateralized, repaying) i.e. not expired, repaid, or seized
	Repaid, 		// closed, repaid
	Seized,			// unpaid, collat seized
}

// Status is by default
impl Default for Status {
	fn default() -> Self { Status::Open }
}

// Asset owners can create a DebtRequest to ask for a traunche of Balance
#[derive(Encode, Decode, Default, Clone, PartialEq)] //these are custom traits required by all structs (some traits forenums)
#[cfg_attr(feature = "std", derive(Debug))] // attr provided by rust compiler. uses derive(debug) trait when in std mode
pub struct Debt<AccountId, Balance, Moment> {   //Needs the blake2 Hash trait
	status: Status,					// Default is open
	// todo, wrap this in Option<T::AccountId>
	requestor: AccountId,		// Account that will go in debt
	beneficiary: AccountId,	// Recipient of Balance
	request_expiry: Moment,	// debt_request 
	//TODO to refactor out into debt_terms (interval, interest rate, deadline)
	// principle total, interest total, deadline
	principal: Balance,			// Principal loan Q: why Balance inside struct, not balanceof
	interest_rate: u64,			// % charged on principal, for every interest period
	interest_period: u64,		// monthly, daily, in seconds
	term_length: u64, 			// repayment time, in seconds
	// Filled in after loan is fulfilled by someone
	creditor: AccountId,  	// null as default
}

type DebtIndex = u64; //like proposalindex in treasury
type OpenIndex = u64; //like proposalindex in treasury

/// This module's storage items.
decl_storage! {
	trait Store for Module<T: Trait> as Debt {
				// TODO later abstrate T::Hash into generic vars, so its not so long?
		// doesn't get deleted
		Debts get(get_debt): map T::Hash => Debt<T::AccountId, BalanceOf<T>, T::Moment>;
		// [0, 0x...] [1, 0x...]
		DebtIndexToId get(get_debt_id): map DebtIndex => T::Hash;
		DebtCount get(get_total_debts): DebtIndex;  //Alias for u64

		// A map of open debts that system must evaluate
		// TODO rename to active: ...
		OpenDebts get(get_open_debt): map OpenIndex => T::Hash;
		OpenDebtsCount get(get_total_open_debts): OpenIndex; 
	}
}

decl_module! {
	/// The module declaration.
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		// Initializing events
		// this is needed only if you are using events in your module
		fn deposit_event<T>() = default;

		pub fn borrow(
				origin, 
				beneficiary: <T::Lookup as StaticLookup>::Source, 
				request_expiry: T::Moment, 
				principal: BalanceOf<T>, //make compact?
				interest_rate: u64,
				interest_period: u64,
				term_length: u64
		) { //TODO, change expiry
			let requestor = ensure_signed(origin)?;		//macro, returns sender address
			let now = <timestamp::Module<T>>::get();

			// Q: whats the diff btw this and just doing <t as system:: trait> .. etc.
			let debt_id = (<system::Module<T>>::random_seed(), &requestor, now).using_encoded(<T as system::Trait>::Hashing::hash); // use runtime_primitives::hash, its a constnat!
			let beneficiary = T::Lookup::lookup(beneficiary)?;		//looks up the accountId.

			// TODO make sure debtrequest doesn't exist already, in case they try to overwrite debt..
			ensure!(!<Debts<T>>::exists(debt_id), "Error: Debt request already exists");
			let new_debt = Debt {
				requestor: requestor.clone(),
				beneficiary,
				request_expiry,
				principal,
				interest_rate,
				interest_period,
				term_length,
				..Default::default()
			};

			// Add new debt request to DebtRequests map
			let i = Self::get_total_debts();
			<DebtCount<T>>::put(i+1); //increment total count by 1
			<DebtIndexToId<T>>::insert(i, debt_id);
			<Debts<T>>::insert(debt_id, new_debt);
			// Emit the event

			Self::deposit_event(RawEvent::DebtCreated(requestor, debt_id));
		}

		// helper fn: get open & collateralized loans...

		// Creditor sends money into this function to fulfill loan
		pub fn fulfill(origin, debt_id: T::Hash) {
			let sender = ensure_signed(origin)?;
			ensure!(<Debts<T>>::exists(debt_id), "This debt does not exist");
			let mut debt = <Debts<T>>::get(debt_id);

			let now = <timestamp::Module<T>>::get();
			ensure!(debt.request_expiry >= now, "This debt request has expired");
			ensure!(debt.status == Status::Open, "This debt request is no longer available");
			ensure!(debt.creditor == <T as system::Trait>::AccountId::default(), "This debt request is fulfilled");
			
			let collateral = <erc721::Module<T>>::get_escrow(debt_id);
			ensure!(collateral != <T as system::Trait>::Hash::default(), "This debt is not collateralized");
			
			// Check sender has enough balance
			// Transfer the money

			// With the currency trait from balances<module<T>>
			T::Currency::transfer(&sender, &debt.beneficiary, debt.principal);
			// debt.creditor = sender;
			
			// Add to active loan
			// Sudo call transfer function...

		}
		

		// user sends money into this fn 
		// 
		pub fn repay(origin, debt_id: T::Hash) {
				// changes status from active to repaid

		}

		// Checks for passive situations
		// collateralized & never funded -> returns collateral
		// past payback date -> seizes collateral
		fn on_initialize() {

			// Check if open/collateralized && expired => EXPIRED
			
			// Check if open & collateralized => Open, add available for debter
				// => 

			// if active & repay date passed  => seized
			// if active & repaid 						=> repaid
			// if active & ddaste has passd => inactive 
		}

		fn on_finalize() {
			// TODO: clean up expired, clean debt requests
			// default/repaid should forever remain on chain... 

		}
		
	}
}


decl_event!(
	pub enum Event<T> where 
		<T as system::Trait>::AccountId,
		<T as system::Trait>::Hash,
	{
		// 								debtor, requestId
		DebtCreated(AccountId, Hash),
	}
);
