/// A runtime module template with necessary imports

/// Feel free to remove or edit this file as needed.
/// If you change the name of this file, make sure to update its references in runtime/src/lib.rs
/// If you remove this file, you can remove those references


/// For more guidance on Substrate modules, see the example module
/// https://github.com/paritytech/substrate/blob/master/srml/example/src/lib.rs

use support::{decl_module, decl_storage, decl_event, StorageValue, StorageMap, dispatch::Result, ensure};
use system::ensure_signed;
use super::erc721;
use rstd::cmp;
use parity_codec::{Encode, Decode}; //enables #[derive(Decode)] Why? what is it
use runtime_primitives::traits::{Hash, Zero, As, CheckedAdd, CheckedSub, CheckedMul}; // StaticLookup, As //static look up is for beneficiary address

use support::traits::Currency;
// replaces Balance type.
type BalanceOf<T> = <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::Balance;

#[cfg(test)]
mod test;

/// The module's configuration trait.
pub trait Trait: timestamp::Trait + erc721::Trait {
	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
	type Currency: Currency<Self::AccountId>;
}

// TODO refactor out the interest calculation parts
// Asset owners can create a DebtRequest to ask for a traunche of Balance
#[derive(Encode, Decode, Default, Clone, PartialEq)] //these are custom traits required by all structs (some traits forenums)
#[cfg_attr(feature = "std", derive(Debug))] // attr provided by rust compiler. uses derive(debug) trait when in std mode
pub struct Debt<AccountId, Balance, Moment> {   //Needs the blake2 Hash trait
	request_expiry: Moment,	// debt_request 

	requestor: AccountId,		// TODO: Use Option<T::AccountId>?
	beneficiary: AccountId,	// Recipient of Balance
	creditor: AccountId,  	// null as default
	
	//TODO to refactor out into debt_terms (interval, interest rate, deadline)
	term_start: Moment,				// when the debt started 
	term_length: Moment, 			// total time *interval* to repay, in seconds. not a date.

	principal: Balance,				// principal remaining
	interest: Balance,				// interest remaining
	interest_rate: u64,				// interest: 100 is 1% , significance to 0.00%
	interest_period: Moment,		// monthly, daily, in seconds
	n_periods: u64, 					// n periods of interest already calculated in interest
}

type DebtIndex = u64; //like proposalindex in treasury
type ActiveIndex = u64; //like proposalindex in treasury

/// This module's storage items.
decl_storage! {
	trait Store for Module<T: Trait> as Debt {		
		Debts get(get_debt): map T::Hash => Debt<T::AccountId, BalanceOf<T>, T::Moment>;
		DebtIndexToId get(get_debt_id): map DebtIndex => T::Hash;
		DebtCount get(get_total_debts): DebtIndex;
	}
}

decl_module! {
	/// The module declaration.
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event<T>() = default;

		pub fn borrow(
				origin, 
				beneficiary: T::AccountId, 
				request_expiry: T::Moment,
				principal: BalanceOf<T>, //make compact?
				interest_rate: u64,
				interest_period: T::Moment,
				term_length: T::Moment
		) { //TODO, change expiry
			let requestor = ensure_signed(origin)?;		//macro, returns sender address
			let now = <timestamp::Module<T>>::get();

			let debt_id = (<system::Module<T>>::random_seed(), &requestor, now).using_encoded(<T as system::Trait>::Hashing::hash); // use runtime_primitives::hash, its a constnat!
			// let beneficiary = T::Lookup::lookup(beneficiary)?;		//looks up the accountId.
	
			ensure!(!<Debts<T>>::exists(debt_id), "Error: Debt request already exists");
			ensure!(! interest_period.is_zero(), "Error: interest period cannot be zero");
			ensure!(! term_length.is_zero(), "Error: term length cannot be zero");
			ensure!( term_length > interest_period, "Error: interest period cannot be longer than term length");

			// Add new debt request to DebtRequests map
			let i = Self::get_total_debts();
			<DebtCount<T>>::put(i+1); //increment total count by 1

			<DebtIndexToId<T>>::insert(i, debt_id);
			

			<Debts<T>>::insert(debt_id, Debt {
													requestor: requestor.clone(),
													beneficiary,
													request_expiry,
													principal,
													interest_rate,
													interest_period,
													term_length,
													..Default::default() //suspected culprit
												}
			);

			Self::deposit_event(RawEvent::DebtBorrowed(requestor, debt_id));
		}

		// Creditor sends money into this function to fulfill loan
		pub fn fulfill(origin, debt_id: T::Hash) {
			let sender = ensure_signed(origin)?;
			ensure!(<Debts<T>>::exists(debt_id), "This debt does not exist");
			let mut debt = <Debts<T>>::get(debt_id);

			let now = <timestamp::Module<T>>::get();
			ensure!(debt.request_expiry >= now, "This debt request has expired");
			ensure!(debt.creditor == <T as system::Trait>::AccountId::default(), "This debt request is fulfilled");
			
			let collateral = <erc721::Module<T>>::get_escrow(debt_id);
			ensure!(collateral != <T as system::Trait>::Hash::default(), "This debt is not collateralized");
			
			T::Currency::transfer(&sender, &debt.beneficiary, debt.principal)?;
			debt.creditor = sender.clone();
			debt.term_start = now;
			<Debts<T>>::insert(debt_id, debt);
			
			Self::deposit_event(RawEvent::DebtFulfilled(sender, debt_id));
		}

		pub fn repay(origin, debt_id: T::Hash, value: BalanceOf<T>) {
			Self::update_balance(debt_id); // calculates interest

			let sender = ensure_signed(origin)?;
			let now = <timestamp::Module<T>>::get();
			
			ensure!(<Debts<T>>::exists(debt_id), "This debt does not exist");
			let mut debt = <Debts<T>>::get(debt_id);

			ensure!(debt.creditor != <T as system::Trait>::AccountId::default(), "This debt was never fulfilled");
			
			let term_end = debt.term_start.clone() + debt.term_length.clone(); // TODO figure out safer way to add Moments 
			ensure!(now <= term_end, "This debt is past due");
		
			// TODO grab the min btw value, and whats owed
			T::Currency::transfer(&sender, &debt.creditor, value)?;
			
			let interest_payment = cmp::min(debt.interest, value);
			// TODO check if this is redundant, since checked_sub substracts in place
			debt.interest = debt.interest.checked_sub(&interest_payment)
				.ok_or("Underflow substracting interest payment")?;

			// 2. if remainder, pay off principal
			if value > interest_payment {
				let principal_payment = value.checked_sub(&interest_payment)
					.ok_or("Underflow substracting principal payment")?;
				debt.principal = debt.principal.checked_sub(&principal_payment)
					.ok_or("Underflow substracting from principal")?;				
			}

			<Debts<T>>::insert(debt_id, debt.clone());

			// TODO: If debt is fully repaid
			if debt.principal.is_zero() && debt.interest.is_zero() {
				<erc721::Module<T>>::uncollateralize_token(debt.requestor, debt_id)?;	
			}

			Self::deposit_event(RawEvent::DebtRepaid(sender, debt_id));
		}

		// The tracking of debt defaults, etc is on the debtor
		// Called by creditor when 
		pub fn seize(origin, debt_id: T::Hash) {	
			Self::update_balance(debt_id); // updates interest calculations

			let sender = ensure_signed(origin)?;
			let now = <timestamp::Module<T>>::get();

			ensure!(<Debts<T>>::exists(debt_id), "This debt does not exist");
			let mut debt = <Debts<T>>::get(debt_id);
			ensure!(debt.creditor != <T as system::Trait>::AccountId::default(), "This debt was never fulfilled");
			let term_end = debt.term_start.clone() + debt.term_length.clone(); // TODO figure out safer way to add Moments 
			ensure!(now >= term_end, "This debt has not defaulted yet");

			let owed = debt.principal + debt.interest;
			ensure!(! owed.is_zero(), "This debt has been paid off");

			<erc721::Module<T>>::uncollateralize_token(debt.creditor, debt_id)?;

			Self::deposit_event(RawEvent::DebtSeized(sender, debt_id));
		}
	}
}

impl <T: Trait> Module<T> {
																	// balance or currency?
	// TODO check if this works for seize
	pub fn update_balance(debt_id: T::Hash) -> Result {
		let now = <timestamp::Module<T>>::get();
		
		// check for activity, not expired, end of term.
		ensure!(<Debts<T>>::exists(debt_id), "This debt does not exist");
		let mut debt = <Debts<T>>::get(debt_id);

		let time_passed = now - (T::Moment::sa(debt.n_periods) * debt.interest_period.clone()); //TODO safer math

		// additional periods to calculate interest for
		let t:u64 = (time_passed / debt.interest_period.clone()).as_();

		// convert to just u64
		let principal:u64 = debt.principal.as_();
		let prev_interest:u64 = debt.interest.as_();
		let prev_balance:u64 = principal + prev_interest;
		
		// simple interest calculation: A=P(1+rt)
		let new_balance = principal * (10000 + debt.interest_rate * t) / 10000; 
		let new_interest = prev_interest + new_balance - principal;

		debt.interest = <BalanceOf<T> as As<u64>>::sa(new_interest as u64); //todo get rid of extraneous things
		debt.n_periods = debt.n_periods + t;

		<Debts<T>>::insert(debt_id, debt.clone());

		Ok(())
	}
}

decl_event!(
	pub enum Event<T> where 
		<T as system::Trait>::AccountId,
		<T as system::Trait>::Hash,
	{
		// 						trx	sender, debt_id
		DebtBorrowed(AccountId, Hash),
		DebtFulfilled(AccountId, Hash), 
		DebtRepaid(AccountId, Hash),
		DebtSeized(AccountId, Hash),
	}
);
