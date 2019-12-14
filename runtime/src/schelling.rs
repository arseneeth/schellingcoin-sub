use support::{decl_module, decl_storage, decl_event, ensure, StorageMap, StorageValue, dispatch::Result};
use parity_codec::{Decode, Encode};
use runtime_primitives::traits::Hash;
use system::ensure_signed;

use crate::token;

/// The module's configuration trait.
pub trait Trait: system::Trait + token::Trait {
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

#[cfg_attr(feature = "std", derive(Debug))]
#[derive(Encode, Decode, Default, Clone, PartialEq)]

pub struct Message<Hash, TokenBalance> {
	mtype: u32, // message type
	mhash: Hash,
	mvalue: u64,
	mdeposit: TokenBalance,
}


/// This module's storage items.
decl_storage! {
	trait Store for Module<T: Trait> as SchellingStorage {

		// BlockNumber of a new epoch being started 
        pub EpochStartBlock get(epoch_start_block): T::BlockNumber;
        // All the messages being submitted in the following epoch
        pub Messages get(messages): map T::AccountId => Message<T::Hash, T::TokenBalance>;
	
        // pub CorrectMessages get(correct_messages): Vec<Message<T::Hash, T::TokenBalance>>;
	}
}

decl_module! {
	/// The module declaration.
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		// Initializing events
		// this is needed only if you are using events in your module
		fn deposit_event<T>() = default;

		fn submit_hash(origin, hash: T::Hash, #[compact] deposit: T::TokenBalance) -> Result{
			let sender = ensure_signed(origin)?;
			// TODO: add more checks

			let message = Message{
				mtype: 1, // todo change variable names
				mhash: hash,
				mvalue: 0,
				mdeposit: deposit,
			};
			<Messages<T>>::insert(sender, message);

			Ok(())
		}

		fn submit_value(origin, #[compact] value: u64) -> Result{
			let sender = ensure_signed(origin)?;
			ensure!(<Messages<T>>::exists(&sender), "Message hash was not submitted");
			// TODO: add more checks

			let mut message = Self::messages(&sender);
			let tuple = (sender.clone(), message.mvalue);
			let random_hash = tuple.using_encoded(<T as system::Trait>::Hashing::hash);
			ensure!(random_hash == message.mhash, "Hashes do not mattch");

			message.mvalue = value;
			message.mtype = 2;
			<Messages<T>>::insert(sender, message);

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
	type schelling = Module<Test>;

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
			assert_ok!(schelling::do_something(Origin::signed(1), 42));
			// asserting that the stored value is equal to what we stored
			assert_eq!(schelling::something(), Some(42));
		});
	}
}
