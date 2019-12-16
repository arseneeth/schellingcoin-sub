use support::{decl_module, decl_storage, decl_event, ensure, StorageMap, StorageValue, dispatch::Result};
use parity_codec::{Decode, Encode};
use runtime_primitives::traits::Hash;
use system::{ensure_signed, ensure_root};
use runtime_primitives::traits::{CheckedAdd, CheckedSub, CheckedMul, CheckedDiv, As};

use crate::token;

/// The module's configuration trait.
pub trait Trait: system::Trait + token::Trait {
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

#[cfg_attr(feature = "std", derive(Debug))]
#[derive(Encode, Decode, Default, Clone, PartialEq)]

pub struct Message<AccountId, Hash, TokenBalance> {
	owner: AccountId,
	status: u32, // TODO: implement status checks and updates
	hash: Hash, 
	value: u64,
	deposit: TokenBalance,
}


decl_storage! {
	trait Store for Module<T: Trait> as SchellingStorage {

        pub TokenBase get(token_base): T::AccountId;

		// BlockNumber of a new epoch being started 
        pub EpochStart get(epoch_start): T::BlockNumber;
        
        // All the messages being submitted in the following epoch
        pub Messages get(messages): map T::AccountId => Message<T::AccountId, T::Hash, T::TokenBalance>;
		
        pub ValidMessages get(valid_messages): Vec<Message<T::AccountId, T::Hash, T::TokenBalance>>;
	
        pub Value get(value): u64;
	}
}

decl_module! {
	/// The module declaration.
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		// Initializing events
		// this is needed only if you are using events in your module
		fn deposit_event<T>() = default;

		fn set_token_base(origin, token_base: T::AccountId) -> Result{
			let _root = ensure_root(origin)?;

			<TokenBase<T>>::put(token_base);

			Ok(())			
		}

		fn new_epoch(origin) -> Result{
			let _root = ensure_root(origin)?;

			let block_number = <system::Module<T>>::block_number();
			<EpochStart<T>>::put(block_number);		
			Ok(())	
		}

		fn submit_hash(origin, hash: T::Hash, #[compact] deposit: T::TokenBalance) -> Result{
			let sender = ensure_signed(origin)?;
			ensure!(!<Messages<T>>::exists(&sender), "There is a submission made by the message sender");		
			
			let epoch_start = Self::epoch_start();
			let block_number = <system::Module<T>>::block_number();
			let deadline = epoch_start.checked_add(&T::BlockNumber::sa(50)).ok_or("Overflow")?;

			ensure!(block_number < deadline, "The deadline for hash submission is passed, try next epoch");
			
			// TODO: add more checks
			<token::Module<T>>::lock(sender.clone(), deposit.clone(), hash.clone())?;
			
			let message = Message{
				owner: sender.clone(),
				status: 1, 
				hash: hash, 
				value: 0,
				deposit: deposit,
			};
			<Messages<T>>::insert(sender, message);

			Ok(())
		}

		fn submit_value(origin, #[compact] value: u64) -> Result{
			let sender = ensure_signed(origin)?;
			ensure!(<Messages<T>>::exists(&sender), "Message hash was not submitted");
			
			let epoch_start = Self::epoch_start();
			let block_number = <system::Module<T>>::block_number();
			let round_one_end = epoch_start.checked_add(&T::BlockNumber::sa(50)).ok_or("Overflow")?;
			let deadline = epoch_start.checked_add(&T::BlockNumber::sa(100)).ok_or("Overflow")?;

			ensure!(block_number > round_one_end, "Hash submission round did not end yet");
			ensure!(block_number < deadline, "The deadline for value submission is passed, please withdraw deposit");
			
			// TODO: add more checks
			let mut message = Self::messages(&sender);
			ensure!(message.status == 1, "Message status should be 1");

			let tuple = (sender.clone(), message.value);
			let random_hash = tuple.using_encoded(<T as system::Trait>::Hashing::hash);
			ensure!(random_hash == message.hash, "Hashes do not match");

			message.value = value;
			message.status = 2;

			let mut valid_messages = Self::valid_messages();
			valid_messages.push(message);

			<ValidMessages<T>>::put(valid_messages);
			
			// delete message from the map
			<Messages<T>>::remove(sender);

			Ok(())
		}

		//  function for deposit withdrawal the case when message was not validated
		fn withdraw(origin) -> Result{
			let sender = ensure_signed(origin)?;
			ensure!(<Messages<T>>::exists(&sender), "Message hash was not submitted");

			let message = Self::messages(&sender);
			let mut message_clone = message.clone();
			ensure!(message.status == 1, "Message status should be 1");
			<token::Module<T>>::unlock(message.owner, message.deposit, message.hash)?;

			message_clone.status = 3;
			<Messages<T>>::insert(sender, message_clone);

			Ok(())
		}


		fn send_rewards(origin) -> Result{
			let _root = ensure_root(origin)?;
			// TODO: add auto triggerring onFinalize
			// TODO: move sorting to this function

			let mut valid_messages = Self::valid_messages();

			// implement quick sort over valid_messages
			valid_messages.sort_by_key(|k| k.value);

			let messages_length = valid_messages.len();
			let lower_border = messages_length.checked_div(4).ok_or("overflow")?;
			let step = messages_length.checked_mul(3).ok_or("overflow")?;
			let upper_border = step.checked_div(4).ok_or("overflow")?;

			let median_index =  messages_length.checked_div(2).ok_or("overflow")?;
			<Value<T>>::put(valid_messages[median_index].value);

			let mut i = 0;

			for message in valid_messages.iter(){
				if i > lower_border && i < upper_border{
					// unlock deposits
					let message_clone = message.clone();
					let owner = message_clone.owner.clone();

					<token::Module<T>>::unlock(message_clone.owner, message_clone.deposit, message_clone.hash)?;

					// send rewards from token_base
					let token_base = Self::token_base();
					let origin_clone = system::RawOrigin::Root.into(); // todo: check out how to solve it the other way
					<token::Module<T>>::transfer_from(origin_clone, token_base, owner, T::TokenBalance::sa(100))?;					
				} else {
					let message_clone = message.clone();
					let deposit = message_clone.deposit;
					let step = deposit.clone().checked_mul(&T::TokenBalance::sa(99)).ok_or("overflow")?;
					let refund = step.checked_div(&T::TokenBalance::sa(100)).ok_or("overflow")?;
					let penalty = deposit.checked_sub(&refund).ok_or("overflow")?;

					// send back deposits with penalties
					<token::Module<T>>::unlock(message_clone.owner, refund, message_clone.hash)?;
					
					// send penalties to token_base
					let token_base = Self::token_base();
					<token::Module<T>>::unlock(token_base, penalty, message_clone.hash)?;					
					
				}
				i = i.checked_add(1).ok_or("overflow")?;
			}
			let token_base = Self::token_base();
			let origin_clone = system::RawOrigin::Signed(token_base.clone()).into();// todo: check out how to solve it the other way

			// TODO: update array with an empty one
			Self::new_epoch(origin_clone)			
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
