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
use runtime_primitives::traits::{Hash, Zero, As, CheckedAdd, CheckedSub, CheckedMul}; // StaticLookup, As //static look up is for beneficiary address

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
	interest_rate: u32,				// % charged on principal, 10 for 10 percent
	interest_period: Moment,		// monthly, daily, in seconds
	n_periods: Moment, 					// n periods of interest already calculated in interest
}

type DebtIndex = u64; //like proposalindex in treasury
type ActiveIndex = u64; //like proposalindex in treasury

/// This module's storage items.
decl_storage! {
	trait Store for Module<T: Trait> as Debt {
		// TODO later abstrate T::Hash into generic vars, so its not so long?
		
		// Queue of active debts. inactive debts are cleared. TODO: track the statuses, credit history
		Debts get(get_debt): map T::Hash => Debt<T::AccountId, BalanceOf<T>, T::Moment>;
		// [0, 0x...] [1, 0x...]
		DebtIndexToId get(get_debt_id): map DebtIndex => T::Hash;
		DebtCount get(get_total_debts): DebtIndex;  //Alias for u64
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
				beneficiary: T::AccountId, 
				request_expiry: T::Moment,
				principal: BalanceOf<T>, //make compact?
				interest_rate: u32,
				interest_period: T::Moment,
				term_length: T::Moment
		) { //TODO, change expiry
			let requestor = ensure_signed(origin)?;		//macro, returns sender address
			let now = <timestamp::Module<T>>::get();

			// Q: whats the diff btw this and just doing <t as system:: trait> .. etc.
			let debt_id = (<system::Module<T>>::random_seed(), &requestor, now).using_encoded(<T as system::Trait>::Hashing::hash); // use runtime_primitives::hash, its a constnat!
			// let beneficiary = T::Lookup::lookup(beneficiary)?;		//looks up the accountId.
	
			ensure!(!<Debts<T>>::exists(debt_id), "Error: Debt request already exists");
			
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

		// helper fn: get Active & collateralized loans...

		// Creditor sends money into this function to fulfill loan
		pub fn fulfill(origin, debt_id: T::Hash) {
			let sender = ensure_signed(origin)?;
			ensure!(<Debts<T>>::exists(debt_id), "This debt does not exist");
			let mut debt = <Debts<T>>::get(debt_id);

			let now = <timestamp::Module<T>>::get();
			ensure!(debt.request_expiry >= now, "This debt request has expired");
			// ensure!(debt.status == Status::Active, "This debt request is no longer available");
			ensure!(debt.creditor == <T as system::Trait>::AccountId::default(), "This debt request is fulfilled");
			
			let collateral = <erc721::Module<T>>::get_escrow(debt_id);
			ensure!(collateral != <T as system::Trait>::Hash::default(), "This debt is not collateralized");
			
			// With the currency trait from balances<module<T>>
			T::Currency::transfer(&sender, &debt.beneficiary, debt.principal)?;
			debt.creditor = sender.clone();
			debt.term_start = now;
			// debt.last_payment = now; //todo: fix this?
			// write to the state...
			<Debts<T>>::insert(debt_id, debt);
			
			// TODO emit event
			Self::deposit_event(RawEvent::DebtFulfilled(sender, debt_id));
		}

		// TODO remove this fn
		pub fn update_balance(debt_id: T::Hash) {
			Self::_update_balance(debt_id);
		}
		// Debtor pays back "value" 
		// right now, has to be entire value... 
		pub fn repay(origin, debt_id: T::Hash, value: BalanceOf<T>) {

			// todo: call update balance

			let sender = ensure_signed(origin)?;
			let now = <timestamp::Module<T>>::get();
			
			ensure!(<Debts<T>>::exists(debt_id), "This debt does not exist");
			let mut debt = <Debts<T>>::get(debt_id);

			ensure!(debt.creditor != <T as system::Trait>::AccountId::default(), "This debt was never fulfilled");
			
			let term_end = debt.term_start.clone() + debt.term_length.clone(); // TODO figure out safer way to add Moments 
			ensure!(now <= term_end, "This debt is past due");
			ensure!(value >= debt.principal, "You have to repay the debt in full");

			// TODO grab the min btw value, and whats owed
			T::Currency::transfer(&sender, &debt.creditor, value)?;
			debt.principal = debt.principal.checked_sub(&value)
				.ok_or("Underflow substracting debt")?;

			<Debts<T>>::insert(debt_id, debt.clone());

			// TODO: If debt is fully repaid
			<erc721::Module<T>>::uncollateralize_token(debt.requestor, debt_id)?;

			Self::deposit_event(RawEvent::DebtRepaid(sender, debt_id));

		}

		// The tracking of debt defaults, etc is on the debtor
		// Called by creditor when 
		pub fn seize(origin, debt_id: T::Hash) {	
			// TODO call _update_balance
			let sender = ensure_signed(origin)?;
			let now = <timestamp::Module<T>>::get();

			ensure!(<Debts<T>>::exists(debt_id), "This debt does not exist");
			let mut debt = <Debts<T>>::get(debt_id);
			ensure!(debt.creditor != <T as system::Trait>::AccountId::default(), "This debt was never fulfilled");
			let term_end = debt.term_start.clone() + debt.term_length.clone(); // TODO figure out safer way to add Moments 
			ensure!(now >= term_end, "This debt has not defaulted yet");
			ensure!(! debt.principal.is_zero(), "This debt has been paid off");
			
			<erc721::Module<T>>::uncollateralize_token(debt.creditor, debt_id)?;
			// Later: if not the creditor, give account a small "hunting" fee

			Self::deposit_event(RawEvent::DebtSeized(sender, debt_id));
		}

		// Later: incentivise people to hunt for defaulted
	}
}

impl <T: Trait> Module<T> {
																	// balance or currency?
	fn _update_balance(debt_id: T::Hash) -> Result {
		let now = <timestamp::Module<T>>::get();
		
		// check for activity, not expired, end of term.
		ensure!(<Debts<T>>::exists(debt_id), "This debt does not exist");
		// TODO check debt hasn't expired or defaulted?

		let mut debt = <Debts<T>>::get(debt_id);

		// time passed since last calculated interest
												// todo: check if n_periods is 0/nil?
		let time_passed = now - (debt.n_periods * debt.interest_period.clone()); //TODO safer math
		println!("Seconds passed since last calc {:?}", time_passed);

		// additional periods to calculate interest for
		let n:u64 = (time_passed / debt.interest_period).as_();			//convert this into u32 somehow
		println!("Periods passed since last calc {:?}", n);

		// convert to just u64
		let prev_balance = debt.principal + debt.interest;
		println!("Prev balance {:?}", prev_balance);
		
		// simple interest calculation: balance = (prev_balance)(1 + interest) ^ periods passed
		let i:f64 = f64::from(debt.interest_rate) / 100.0 + 1.0;
		let x = i.powi(n as i32) as u64;

						
		let new_balance = prev_balance + <BalanceOf<T> as As<u64>>::sa(x as u64);
		// println!("New balance:{:?}", new_balance);

		// let new_balance = prev_balance.checked_add(&y);
		// let new_balance = prev_balance * y;
		// let x = (100 + debt.interest_rate).pow(3) ;

		// convert into T::balance

		// debt.n_periods = debt.n_periods + n; // add new periods compounded
		// debt.interest = new_balance - debt.principal; // update interest with the new compounded interest
		// <Debts<T>>::insert(debt_id, debt.clone());

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
