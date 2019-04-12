/// A runtime module template with necessary imports

/// Feel free to remove or edit this file as needed.
/// If you change the name of this file, make sure to update its references in runtime/src/lib.rs
/// If you remove this file, you can remove those references


/// For more guidance on Substrate modules, see the example module
/// https://github.com/paritytech/substrate/blob/master/srml/example/src/lib.rs

use support::{decl_module, decl_storage, decl_event, StorageValue, dispatch::Result, ensure};
use system::ensure_signed;
use super::erc721;
use parity_codec::{Encode, Decode}; //enables #[derive(Decode)] Why? what is it
use runtime_primitives::traits::{Hash, StaticLookup}; // Zero, As //static look up is for beneficiary address

/// The module's configuration trait.
pub trait Trait: timestamp::Trait + erc721::Trait {
	// TODO: Add other types and constants required configure this module.

	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

// Asset owners can create a DebtRequest to ask for a traunche of Balance
#[derive(Encode, Decode, Default, Clone, PartialEq)] //these are custom traits required by all structs (some traits forenums)
#[cfg_attr(feature = "std", derive(Debug))] // attr provided by rust compiler. uses derive(debug) trait when in std mode
pub struct DebtRequest<Hash, AccountId, Balance, Moment> {   //Needs the blake2 Hash trait
	requestor: AccountId,		// Account that will go in debt
	beneficiary: AccountId,	// Recipient of Balance
	amount: Balance,				// Amount of loan
	expiry: Moment,					// Duration of debtRequest
	collateralized: bool,		// Defaults to false, true upon collaterlization
}

type DebtRequestIndex = u64; //like proposalindex in treasury

/// This module's storage items.
decl_storage! {
	trait Store for Module<T: Trait> as Debt {
				// TODO later abstrate T::Hash into generic vars, so its not so long?
		// doesn't get deleted
		DebtRequests get(get_debt_order): map T::Hash => DebtRequest<T::Hash, T::AccountId, T::Balance, T::Moment>; //DebtRequest ID to the RequestItself
		// [0, 0x...] [1, 0x...]
		DebtRequestIndexToId get(get_debt_request_id): map DebtRequestIndex => T::Hash;
		DebtRequestCount get(get_total_debt_requests): DebtRequestIndex;  //Alias for u64

		// Just a dummy storage item. 
		// Here we are declaring a StorageValue, `Something` as a Option<u32>
		// `get(something)` is the default getter which returns either the stored `u32` or `None` if nothing stored
		Something get(something): Option<u32>;
	}
}

decl_module! {
	/// The module declaration.
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		// Initializing events
		// this is needed only if you are using events in your module
		fn deposit_event<T>() = default;

		pub fn create_debt_request(
				origin, 
				amount: T::Balance, 
				beneficiary: <T::Lookup as StaticLookup>::Source, 
				expiry: T::Moment
		) { //TODO, change expiry
			let requestor = ensure_signed(origin)?;		//macro, returns sender address
			let now = <timestamp::Module<T>>::get();

			// Q: whats the diff btw this and just doing <t as system:: trait> .. etc.
			let id = (<system::Module<T>>::random_seed(), &requestor, now).using_encoded(<T as system::Trait>::Hashing::hash); // use runtime_primitives::hash, its a constnat!
			let collateralized = false; //TODO search how to default, soi dont have to set it.
			let beneficiary = T::Lookup::lookup(beneficiary)?;		//looks up the accountId.

			// TODO make sure debtrequest doesn't exist already, in case they try to overwrite debt..
			ensure!(!<DebtRequests<T>>::exists(id), "Error: Debt request already exists");
			let new_debt_request = DebtRequest {
				requestor: requestor.clone(),
				beneficiary: beneficiary.clone(), 	// can i do this here?!
				amount,
				expiry,
				collateralized
			};

			// Add new debt request to DebtRequests map
			let i = Self::get_total_debt_requests();
			<DebtRequestCount<T>>::put(i+1); //increment total count by 1
			<DebtRequestIndexToId<T>>::insert(i, id);
			<DebtRequests<T>>::insert(id, new_debt_request);
			// Emit the event

			Self::deposit_event(RawEvent::DebtRequestCreated(requestor, id));
			// TODO remove later
		}

		// Just a dummy entry point.
		// function that can be called by the external world as an extrinsics call
		// takes a parameter of the type `AccountId`, stores it and emits an event
		pub fn do_something(origin, something: u32) -> Result {
			// TODO: You only need this if you want to check it was signed.
			let who = ensure_signed(origin)?;

			// TODO: Code to execute when something calls this.
			// For example: the following line stores the passed in u32 in the storage
			<Something<T>>::put(something);

			// here we are raising the Something event
			Self::deposit_event(RawEvent::SomethingStored(something, who));
			Ok(())
		}
	}
}

decl_event!(
	pub enum Event<T> where AccountId = <T as system::Trait>::AccountId {
		// Just a dummy event.
		// Event `Something` is declared with a parameter of the type `u32` and `AccountId`
		// To emit this event, we call the deposit funtion, from our runtime funtions
		SomethingStored(u32, AccountId),
	}
);
