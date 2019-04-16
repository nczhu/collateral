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
use parity_codec::{Encode, Decode};
use runtime_primitives::traits::{Hash, Zero, As, CheckedSub};

use support::traits::Currency;

type BalanceOf<T> = <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::Balance;

#[cfg(test)]
mod test;

/// The module's configuration trait.
pub trait Trait: timestamp::Trait + erc721::Trait {
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
	type Currency: Currency<Self::AccountId>;
}

// Asset owners can create a DebtRequest to ask for a traunche of Balance
#[derive(Encode, Decode, Default, Clone, PartialEq)] //these are custom traits required by all structs (some traits forenums)
#[cfg_attr(feature = "std", derive(Debug))] // attr provided by rust compiler. uses derive(debug) trait when in std mode
pub struct Debt<AccountId, Balance, Moment> {   //Needs the blake2 Hash trait
	request_expiry: Moment,	// debt_request 

	requestor: AccountId,		// TODO: Use Option<T::AccountId>?
	beneficiary: AccountId,	// Recipient of the loan
	creditor: AccountId,
	
	//TODO: refactor out debt-terms attributes
	term_start: Moment,				// when the debt was fulfilled & loanded
	term_length: Moment, 			// total time *interval* to repay, in seconds. not a date.

	principal: Balance,				// principal remaining
	interest: Balance,				// interest remaining
	interest_rate: u64,				// interest: 100 is 1% , significance to 0.00%
	interest_period: Moment,	// monthly, daily, in seconds
	n_periods: u64, 					// n periods of interest already calculated in interest
}

type DebtIndex = u64;

decl_storage! {
	trait Store for Module<T: Trait> as Debt {		
		Debts get(get_debt): map T::Hash => Debt<T::AccountId, BalanceOf<T>, T::Moment>;
		DebtIndexToId get(get_debt_id): map DebtIndex => T::Hash;
		DebtCount get(get_total_debts): DebtIndex;
	}
}

decl_module! {

	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event<T>() = default;

		pub fn borrow(
				origin, 
				beneficiary: T::AccountId, // why do let beneficiary = T::Lookup::lookup(beneficiary)?;
				request_expiry: T::Moment,
				principal: BalanceOf<T>, //make compact?
				interest_rate: u64,
				interest_period: T::Moment,
				term_length: T::Moment
		) {
			let requestor = ensure_signed(origin)?;	
			let now = <timestamp::Module<T>>::get();

			let debt_id = (<system::Module<T>>::random_seed(), &requestor, now).using_encoded(<T as system::Trait>::Hashing::hash);
	
			ensure!(!<Debts<T>>::exists(debt_id), "Error: Debt request already exists");
			ensure!(! interest_period.is_zero(), "Error: interest period cannot be zero");
			ensure!(! term_length.is_zero(), "Error: term length cannot be zero");
			ensure!( term_length > interest_period, "Error: interest period cannot be longer than term length");

			let i = Self::get_total_debts();
			<DebtCount<T>>::put(i+1);

			<DebtIndexToId<T>>::insert(i, debt_id);

			<Debts<T>>::insert(debt_id, Debt { requestor: requestor.clone(), beneficiary, request_expiry, 
																				principal,interest_rate, interest_period, term_length, ..Default::default() }
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

		// Debtors can repay on a debt
		pub fn repay(origin, debt_id: T::Hash, value: BalanceOf<T>) {
			Self::update_balance(debt_id);

			let sender = ensure_signed(origin)?;
			let now = <timestamp::Module<T>>::get();
			
			ensure!(<Debts<T>>::exists(debt_id), "This debt does not exist");
			
			let mut debt = <Debts<T>>::get(debt_id);
			ensure!(debt.creditor != <T as system::Trait>::AccountId::default(), "This debt was never fulfilled");
			
			let term_end = debt.term_start.clone() + debt.term_length.clone(); // TODO figure out safer way to add Moments 
			ensure!(now <= term_end, "This debt is past due");
		
			let balance = debt.principal + debt.interest; 
			let payment = cmp::min(value, balance); 			// make sure debtor doesn't overpay
			
			T::Currency::transfer(&sender, &debt.creditor, payment)?;
			
			// 1. Substrate from interest first
			let interest_payment = cmp::min(debt.interest, payment);
			debt.interest = debt.interest.checked_sub(&interest_payment)
				.ok_or("Underflow substracting interest payment")?;

			// 2. If money left, substract from the principal
			if payment > interest_payment {
				let principal_payment = payment.checked_sub(&interest_payment)
					.ok_or("Underflow substracting principal payment")?;
				debt.principal = debt.principal.checked_sub(&principal_payment)
					.ok_or("Underflow substracting from principal")?;				
			}

			<Debts<T>>::insert(debt_id, debt.clone());

			if debt.principal.is_zero() && debt.interest.is_zero() {
				<erc721::Module<T>>::uncollateralize_token(debt.requestor, debt_id)?;	
			}

			Self::deposit_event(RawEvent::DebtRepaid(sender, debt_id));
		}

		// Creditors can seize expired loans
		pub fn seize(origin, debt_id: T::Hash) {	
			Self::update_balance(debt_id);

			let sender = ensure_signed(origin)?;
			let now = <timestamp::Module<T>>::get();

			ensure!(<Debts<T>>::exists(debt_id), "This debt does not exist");
			let mut debt = <Debts<T>>::get(debt_id);

			ensure!(debt.creditor != <T as system::Trait>::AccountId::default(), "This debt request was never fulfilled");
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
	pub fn update_balance(debt_id: T::Hash) -> Result {
		let now = <timestamp::Module<T>>::get();

		ensure!(<Debts<T>>::exists(debt_id), "This debt does not exist");
		let mut debt = <Debts<T>>::get(debt_id);

		let time_passed = now - (T::Moment::sa(debt.n_periods) * debt.interest_period.clone()); //TODO safer math

		// additional periods to calculate interest for
		let t:u64 = (time_passed / debt.interest_period.clone()).as_();
		let principal:u64 = debt.principal.as_();
		let prev_interest:u64 = debt.interest.as_();
		
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
		DebtBorrowed(AccountId, Hash),
		DebtFulfilled(AccountId, Hash), 
		DebtRepaid(AccountId, Hash),
		DebtSeized(AccountId, Hash),
	}
);
