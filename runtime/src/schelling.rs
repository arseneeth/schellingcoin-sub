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
	status: u32, 
	hash: Hash, 
	value: u64,
	deposit: TokenBalance,
}

decl_storage! {
	trait Store for Module<T: Trait> as SchellingStorage {

		// Address that we send rewards from
        pub TokenBase get(token_base): T::AccountId;

		// BlockNumber of a new epoch being started 
        pub EpochStart get(epoch_start): T::BlockNumber;
        
        // All the messages being submitted in the following epoch
        pub Messages get(messages): map T::AccountId => Message<T::AccountId, T::Hash, T::TokenBalance>;
		
		// Messages that passed our checks
        pub ValidMessages get(valid_messages): Vec<Message<T::AccountId, T::Hash, T::TokenBalance>>;
	
		// Output of our alrorithm, source of wisdom of the crowd 
        pub Value get(value): u64;

        // Minimal deposit
        pub MinDeposit get(min_deposit): T::TokenBalance;
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {

		fn deposit_event<T>() = default;

		fn new_epoch(origin) -> Result{
			let _root = ensure_root(origin)?;

			let block_number = <system::Module<T>>::block_number();
			<EpochStart<T>>::put(block_number.clone());		

			// emit event that new epoch has started
			Self::deposit_event(RawEvent::NewEpochStarted(block_number));

			Ok(())	
		}

		fn submit_hash(origin, hash: T::Hash, #[compact] deposit: T::TokenBalance) -> Result{
			let sender = ensure_signed(origin)?;
			ensure!(!<Messages<T>>::exists(&sender), "There is a submission made by the message sender");		
			ensure!(deposit >= Self::min_deposit(), "The deposit is not high enough");		
			
			let epoch_start = Self::epoch_start();
			let block_number = <system::Module<T>>::block_number();

			// deadline for hash submission 50 blocks after the epoch start
			let deadline = epoch_start.checked_add(&T::BlockNumber::sa(50)).ok_or("Overflow")?;

			ensure!(block_number < deadline, "The deadline for hash submission is passed, try next epoch");
			
			// lock the deposit of the sender
			<token::Module<T>>::lock(sender.clone(), deposit.clone(), hash.clone())?;
			
			// compose a message and add to the message list
			let message = Message{
				owner: sender.clone(),
				status: 1, 
				hash: hash, 
				value: 0,
				deposit: deposit.clone(),
			};
			<Messages<T>>::insert(sender.clone(), message);

			// emit event that the hash was submitted
			Self::deposit_event(RawEvent::HashSubmitted(sender, deposit));

			Ok(())
		}

		fn submit_value(origin, #[compact] value: u64) -> Result{
			let sender = ensure_signed(origin)?;
			ensure!(<Messages<T>>::exists(&sender), "Message hash was not submitted");
			
			let epoch_start = Self::epoch_start();
			let block_number = <system::Module<T>>::block_number();

			// the end of the value submission round
			let round_one_end = epoch_start.checked_add(&T::BlockNumber::sa(50)).ok_or("Overflow")?;
			
			// the period for value submission is between 50 and 100 blocks after the epoch start
			let deadline = epoch_start.checked_add(&T::BlockNumber::sa(100)).ok_or("Overflow")?;

			ensure!(block_number > round_one_end, "Hash submission round did not end yet");
			ensure!(block_number < deadline, "The deadline for value submission is passed, please withdraw deposit");
			
			let mut message = Self::messages(&sender);
			ensure!(message.status == 1, "Message status should be 1");

			// compare the hash of account id and value with the hash being submitted
			let tuple = (sender.clone(), message.value);
			let random_hash = tuple.using_encoded(<T as system::Trait>::Hashing::hash);
			ensure!(random_hash == message.hash, "Hashes do not match");

			// update message info and add to the list of valid messages
			message.value = value.clone();
			message.status = 2;

			let mut valid_messages = Self::valid_messages();
			valid_messages.push(message);

			<ValidMessages<T>>::put(valid_messages);

			// emit event that the value submission was accepted
			Self::deposit_event(RawEvent::ValueSubmissionAccepted(sender.clone(), value));

			// delete message from the map
			<Messages<T>>::remove(sender);

			Ok(())
		}

		//  function for deposit withdrawal the case when message was not validated
		fn withdraw(origin) -> Result{
			let sender = ensure_signed(origin)?;
			ensure!(<Messages<T>>::exists(&sender), "Message hash was not submitted");

			let message = Self::messages(&sender);
			ensure!(message.status == 1, "Message status should be 1");
			<token::Module<T>>::unlock(message.owner, message.deposit, message.hash)?;

			// delete message from the map
			<Messages<T>>::remove(sender.clone());

			// emit event that the deposit was withdrawn
			Self::deposit_event(RawEvent::DepositWithdrawn(sender, message.deposit));

			Ok(())
		}

		fn send_rewards(origin) -> Result{
			let _root = ensure_root(origin)?;
			// TODO: add auto triggerring onFinalize

			let epoch_start = Self::epoch_start();
			// Should be triggered automatically on the 101st block after 
			// the epoch start, in the current implementation it is called manually 
			let epoch_end = epoch_start.checked_add(&T::BlockNumber::sa(101)).ok_or("Overflow")?;
			let block_number = <system::Module<T>>::block_number();

			ensure!(block_number == epoch_end, "It's not the time to send out the rewards yet");

			let mut valid_messages = Self::valid_messages();

			// implement quick sort over valid_messages by value submitted
			valid_messages.sort_by_key(|k| k.value);

			let messages_length = valid_messages.len();

			// get 25th and 75th percentiles
			let lower_border = messages_length.checked_div(4).ok_or("overflow")?;
			let step = messages_length.checked_mul(3).ok_or("overflow")?;
			let upper_border = step.checked_div(4).ok_or("overflow")?;

			// get median 
			let median_index =  messages_length.checked_div(2).ok_or("overflow")?;
			<Value<T>>::put(valid_messages[median_index].value);

			// Emit event that new value is being set
			Self::deposit_event(RawEvent::NewValueSet(valid_messages[median_index].value));

			let mut i = 0;

			for message in valid_messages.iter(){
				// if inside 25 and and 75 percentile range 
				if i > lower_border && i < upper_border{
					// unlock deposits
					let message_clone = message.clone();
					let owner = message_clone.owner.clone();

					<token::Module<T>>::unlock(message_clone.owner, message_clone.deposit, message_clone.hash)?;

					// send rewards from token_base
					let token_base = Self::token_base();
					let origin_clone = system::RawOrigin::Root.into();
					<token::Module<T>>::transfer_from(origin_clone, token_base, owner, T::TokenBalance::sa(100))?;					
				// if out of the range
				} else {
					let message_clone = message.clone();
					let deposit = message_clone.deposit;

					// get the 99 percent of the deposit to refund 
					let step = deposit.clone().checked_mul(&T::TokenBalance::sa(99)).ok_or("overflow")?;
					let refund = step.checked_div(&T::TokenBalance::sa(100)).ok_or("overflow")?;
					let penalty = deposit.checked_sub(&refund).ok_or("overflow")?;

					// send back deposits after subtration of penalties
					<token::Module<T>>::unlock(message_clone.owner, refund, message_clone.hash)?;
					
					// send penalties to token_base
					let token_base = Self::token_base();
					<token::Module<T>>::unlock(token_base, penalty, message_clone.hash)?;					
					
				}
				i = i.checked_add(1).ok_or("overflow")?;
			}
			let origin_clone = system::RawOrigin::Root.into(); 
			
			//replace ValidMessages array with an empty one
			<ValidMessages<T>>::put(Vec::new());

			Self::new_epoch(origin_clone)			
		}
		
	}
}

decl_event!(
	pub enum Event<T> where AccountId = <T as system::Trait>::AccountId,
							Balance = <T as token::Trait>::TokenBalance,
							BlockNumber = <T as system::Trait>::BlockNumber,
	{

		NewEpochStarted(BlockNumber),
		HashSubmitted(AccountId, Balance),
		ValueSubmissionAccepted(AccountId, u64),
		DepositWithdrawn(AccountId, Balance),
		NewValueSet(u64),

	}
);

/// tests for this module
#[cfg(test)]
mod tests {
	use super::*;

	use primitives::{Blake2Hasher, H256};
	use runtime_io::with_externalities;
	use runtime_primitives::{
		testing::{Digest, DigestItem, Header, UintAuthorityId},
		traits::{BlakeTwo256, IdentityLookup},
		BuildStorage,
  	};
  	use support::{assert_noop, assert_ok, impl_outer_origin};

  	impl_outer_origin! {
    	pub enum Origin for Test {}
  	}

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
	    type Lookup = IdentityLookup<u64>;
	    type Header = Header;
	    type Event = ();
	    type Log = DigestItem;
	}

	impl consensus::Trait for Test {
	    type Log = DigestItem;
	    type SessionKey = UintAuthorityId;
	    type InherentOfflineReport = ();
	}

	impl token::Trait for Test {
	    type Event = ();
	    type TokenBalance = u64;
	}
	  
	impl Trait for Test {
	    type Event = ();
	}

	type schelling = Module<Test>;
	type Token = token::Module<Test>;

	fn new_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
		system::GenesisConfig::<Test>::default().build_storage().unwrap().0.into()
	}

	#[test]
	fn dummy_test() {
		with_externalities(&mut new_test_ext(), || {
			assert_eq!(1, 1);
		});
	}
}
