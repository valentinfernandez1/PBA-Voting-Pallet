#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub type ProposalId = u32;

#[frame_support::pallet]
pub mod pallet {
	use core::cmp::Ordering;

use frame_support::{
		pallet_prelude::*,
		traits::{Currency, LockableCurrency, ReservableCurrency}, Blake2_128Concat, ensure, 
	};
	use frame_system::pallet_prelude::*;

	use crate::ProposalId;

	pub type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Type to access the Balances Pallet.
		type Currency: Currency<Self::AccountId>
			+ ReservableCurrency<Self::AccountId>
			+ LockableCurrency<Self::AccountId>;
	}

	#[pallet::storage]
	pub type RegisteredVoters<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, ()>;
	
	#[pallet::storage]
	pub type Proposals<T: Config> = StorageMap<_, Blake2_128Concat, ProposalId , Proposal<T>>;

	///Stores votes mapped to the account and proposal associated.
	#[pallet::storage]
	pub type Votes<T: Config> = StorageDoubleMap<_, Blake2_128Concat, T::AccountId, Blake2_128Concat, ProposalId, Vote>;

	#[pallet::storage]
	pub type ProposalCounter<T: Config> = StorageValue<_, ProposalId>;
	

	#[derive(Encode, Decode, TypeInfo, MaxEncodedLen, Clone)]
	#[scale_info(skip_type_params(T))]
	pub struct Proposal<T: Config> {
		pub id: ProposalId,
		pub proposer: T::AccountId,
		pub text: T::Hash,
		pub time_period: T::BlockNumber,
		pub status: ProposalStatus,
		pub ayes: u32,
		pub nays: u32,
	}

	impl<T: Config> Proposal<T> {
		fn new (
			id: ProposalId, 
			proposer: T::AccountId, 
			text: T::Hash, 
			time_period: T::BlockNumber
		) -> Self {
			Proposal { 
				id, 
				proposer, 
				text, 
				time_period, 
				status: ProposalStatus::InProgress, 
				ayes: 0, 
				nays: 0 
			}
		}
	}

	#[derive(Encode, Debug, Decode, Clone, TypeInfo, MaxEncodedLen, Eq, PartialEq)]
	pub struct Vote {
		pub in_favor: bool,
		pub amount: u32,
	}

	#[derive(Encode, Debug, Decode, TypeInfo, MaxEncodedLen, Clone, Eq, PartialEq)]
	#[scale_info(skip_type_params(T))]
	pub enum ProposalStatus {
		InProgress,
		Canceled,
		Passed,
		Rejected,
		Tied,
	}

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/main-docs/build/events-errors/
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		///New voter registered by root into the RegisteredVoters list 
		VoterRegistered {who: T::AccountId},
		///A user submitted a new proposal
		ProposalSubmitted {proposal_id: ProposalId, who: T::AccountId},
		///A registered voter casted a vote for a specific proposal
		VoteCasted {proposal_id: ProposalId, who: T::AccountId},
		///Registered voter updated his vote with new amount
		VoteUpdated {proposal_id: ProposalId, who: T::AccountId, previous: u32, new: u32},
		///Proposal ended and result is defined 
		ProposalEnded {proposal_id: ProposalId, status: ProposalStatus},
		///End time for the proposal has been updated
		ProposalUpdated {proposal_id: ProposalId, end_block: T::BlockNumber},
		///Proposal canceled by the proposer
		ProposalCanceled {proposal_id: ProposalId}
	}

	// Errors inform users that something went wrong.s
	#[pallet::error]
	pub enum Error<T> {
		///Voter already registered
		AlreadyRegistered,
		///Voter is not registered to cast vote
		VoterIsNotRegistered,
		///The vote of the voter for the proposal is already registered.
		VoteAlreadyCasted,
		///Vote not found for user and proposal
		VoteNotFound,
		///Reduction of votes not allowed after 
		InvalidVoteAmount,
		///The received amount of votes to update is invalid.
		InvalidUpdateAmount,
		///Block number received lower or equal to current block number
		TimePeriodToLow,
		///The proposal counter reached overflow limit
		ProposalIdToHigh,
		///The obtained proposal doesn't exist
		ProposalNotFound,
		///User not authorized to execute extrinsic
		Unauthorized,
		///The proposal is already ended, therefore it can not be modified.
		ProposalAlreadyEnded
		

	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {

		#[pallet::call_index(0)]
		#[pallet::weight(0)]
		pub fn register_voter(
			origin: OriginFor<T>,
			who: T::AccountId) 
		-> DispatchResult {
			ensure_root(origin)?;
			ensure!(!Self::is_registered(&who), Error::<T>::AlreadyRegistered);

			RegisteredVoters::<T>::insert(who.clone(), ());
			Self::deposit_event(Event::VoterRegistered { who });
			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(0)]
		pub fn make_proposal(
			origin: OriginFor<T>, 
			description: T::Hash, 
			time_period: T::BlockNumber
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let current_block_number = <frame_system::Pallet<T>>::block_number();
			ensure!(time_period > current_block_number, Error::<T>::TimePeriodToLow);

			let mut proposal_id: ProposalId = ProposalCounter::<T>::get().unwrap_or_default() ;
			ensure!(proposal_id.checked_add(1).is_some(), Error::<T>::ProposalIdToHigh);
			proposal_id = proposal_id + 1;

			let new_proposal = Proposal::<T>::new(
				proposal_id, 
				who.clone(), 
				description, 
				time_period
			);

			Proposals::<T>::insert(proposal_id, new_proposal);
			ProposalCounter::<T>::put(proposal_id);
			Self::deposit_event(Event::ProposalSubmitted { proposal_id, who });

			Ok(())
		}

	 	#[pallet::call_index(2)]
		#[pallet::weight(0)]
		pub fn increase_proposal_time(
			origin: OriginFor<T>, 
			proposal_id: ProposalId,
			new_time_period: T::BlockNumber
		) -> DispatchResult{
			let who = ensure_signed(origin)?;
			let current_block_number = <frame_system::Pallet<T>>::block_number();
			
			let proposal = Self::get_proposal(&proposal_id).ok_or(Error::<T>::ProposalNotFound)?;
			ensure!(proposal.proposer == who, Error::<T>::Unauthorized);
			ensure!(new_time_period > proposal.time_period,Error::<T>::TimePeriodToLow);
			ensure!(new_time_period > current_block_number,Error::<T>::TimePeriodToLow);

			<Proposals<T>>::mutate(proposal_id,|proposal|{
				if let Some(p) = proposal.as_mut() {
					p.time_period = new_time_period
				}
			});

			Self::deposit_event(Event::ProposalUpdated {proposal_id, end_block: new_time_period});

			Ok(())
		}

		#[pallet::call_index(3)]
		#[pallet::weight(0)]
		pub fn cancel_proposal(
			origin: OriginFor<T>, 
			proposal_id: ProposalId,
		) -> DispatchResult{
			let who = ensure_signed(origin)?;
			let current_block_number = <frame_system::Pallet<T>>::block_number();
			
			let proposal = Self::get_proposal(&proposal_id).ok_or(Error::<T>::ProposalNotFound)?;

			ensure!(proposal.proposer == who, Error::<T>::Unauthorized);
			ensure!(proposal.status == ProposalStatus::InProgress, Error::<T>::ProposalAlreadyEnded);
			ensure!(proposal.time_period > current_block_number,Error::<T>::TimePeriodToLow );

			<Proposals<T>>::mutate(proposal_id,|proposal|{
				if let Some(p) = proposal.as_mut() {
					p.status = ProposalStatus::Canceled
				}
			});
			Self::deposit_event(Event::ProposalCanceled {proposal_id});

			Ok(())
		}

		#[pallet::call_index(4)]
		#[pallet::weight(0)]
		pub fn vote(
			origin: OriginFor<T>, 
			proposal_id: ProposalId,
			vote: Vote 
		) -> DispatchResult {
			//Verify sender is part of register voters
			let who: T::AccountId = ensure_signed(origin)?;
			ensure!(Self::is_registered(&who), Error::<T>::VoterIsNotRegistered);
			//Verify if voter already casted vote
			ensure!(!Self::vote_casted(&who, &proposal_id), Error::<T>::VoteAlreadyCasted);
			
			ensure!(vote.amount>0, Error::<T>::InvalidVoteAmount);
			/* Add logic to verify and lock balance
			

			*/
			
			//Insert vote and update proposals
			<Votes<T>>::insert(who.clone(), proposal_id, vote.clone());
			<Proposals<T>>::mutate(proposal_id, |proposal|{
				if let Some(p) = proposal.as_mut() {
					if vote.in_favor {p.ayes += vote.amount;} else {p.nays += vote.amount;}
				}
			});
			Self::deposit_event(Event::VoteCasted {proposal_id, who});
			Ok(())
		}

/* 		#[pallet::call_index(5)]
		#[pallet::weight(0)]
		pub fn increase_vote(
			origin: OriginFor<T>, 
			proposal_id: ProposalId,
			amount: u32,
		) -> DispatchResult {
			//Verify sender is part of register voters and vote exists
			let who: T::AccountId = ensure_signed(origin)?;
			ensure!(Self::is_registered(&who), Error::<T>::VoterIsNotRegistered);

			//Get vote and verify if it exists
			let vote = <Votes<T>>::try_get(&who, &proposal_id);
			ensure!(vote.is_ok(), Error::<T>::VoteNotFound);
			let vote = vote.unwrap();

			ensure!(amount != 0, Error::<T>::InvalidUpdateAmount);
			/* 
				Add reserve currency logic
			*/

			<Votes<T>>::mutate(who.clone(), proposal_id, |vote|{
				if let Some(v) = vote.as_mut() {
					v.amount += amount;
				}
			});
			<Proposals<T>>::mutate(proposal_id, |proposal|{
				if let Some(p) = proposal.as_mut() {
					if vote.in_favor {p.ayes += vote.amount;} else {p.nays += vote.amount;}
				}
			});
			Self::deposit_event(Event::VoteCasted {proposal_id, who});

			Ok(())
		}

		
		#[pallet::call_index(6)]
		#[pallet::weight(0)]
		pub fn decrease_vote(
			origin: OriginFor<T>, 
			proposal_id: ProposalId,
			amount: i32,
		) -> DispatchResult {
			todo!()
		}
		
		#[pallet::call_index(9)]
		#[pallet::weight(0)]
		pub fn remove_vote(
			origin: OriginFor<T>, 
			proposal_id: ProposalId,
			amount: i32,
		) -> DispatchResult {
			todo!()
		} */

		
		#[pallet::call_index(7)]
		#[pallet::weight(0)]
		pub fn finish_proposal(
			origin: OriginFor<T>, 
			proposal_id: ProposalId,
		) -> DispatchResult {
			//Verify sender is part of register voters and vote exists
			let who: T::AccountId = ensure_signed(origin)?;
			ensure!(Self::is_registered(&who), Error::<T>::VoterIsNotRegistered);
			
			// works but ugly. Avoid and use implementation below this comment
			//let proposal = Self::get_proposal(&proposal_id);
			//ensure!(proposal.is_some(), Error::<T>::ProposalNotFound);
			//let mut proposal: Proposal<T> = proposal.unwrap();

			// same!
			let mut proposal = Self::get_proposal(&proposal_id).ok_or(Error::<T>::ProposalNotFound)?;

			let current_block_number = <frame_system::Pallet<T>>::block_number();
			ensure!(
				proposal.time_period<current_block_number && proposal.status == ProposalStatus::InProgress,
				Error::<T>::ProposalAlreadyEnded
			);

			let voting_result: ProposalStatus = match proposal.ayes.cmp(&proposal.nays){
				Ordering::Less => ProposalStatus::Rejected,
				Ordering::Greater => ProposalStatus::Passed,
				Ordering::Equal => ProposalStatus::Tied
			};

			proposal.status = voting_result.clone();

			<Proposals<T>>::insert(proposal_id, proposal);
			Self::deposit_event(Event::ProposalEnded { proposal_id, status: voting_result});
			Ok(())
		} 


		#[pallet::call_index(8)]
		#[pallet::weight(0)]
		pub fn unlock_balance(
			origin: OriginFor<T>, 
		) -> DispatchResult {
			todo!()
		} 
	}
	
	impl <T: Config> Pallet<T> {
		pub fn is_registered(who: &T::AccountId) -> bool {
			RegisteredVoters::<T>::contains_key(who)
		}

		pub fn proposal_exists(proposal_id: ProposalId) -> bool {
			Proposals::<T>::contains_key(proposal_id)
		}
		pub fn get_proposal_counter() -> ProposalId {
			ProposalCounter::<T>::get().unwrap_or_default()
		}
		pub fn get_proposal(proposal_id: &ProposalId) -> Option<Proposal<T>> {
			<Proposals<T>>::get(proposal_id)
		}
		pub fn vote_casted(who: &T::AccountId, proposal_id: &ProposalId) -> bool {
			if <Votes<T>>::try_get(who, proposal_id).is_err() {return false};
			true
		}
	}
}


//Anotations
// Desibil users:
//	Created a list of registered voters
// Proposal<T> structure: struct with lots of derives like decode encode typeinfo maxencodedlen
// 	data from struct, types hardcoded
// 	enum of proposalStatus
// 	Write on top of proposal [scale_info(skip_tyoe_params(T)))]
// Close strategies:
// 	Hooks(not recommended)
//	proposer closes the voting.
// 	if voter calls and past blocklimit voting is closed
//		search in collective pallet Pays::no so the last voter doesn't pay the fee for closing the voting. 


/* 		/// An example dispatchable that takes a singles value as a parameter, writes the value to
		/// storage and emits an event. This function must be dispatched by a signed extrinsic.
		#[pallet::call_index(0)]
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1).ref_time())]
		pub fn do_something(origin: OriginFor<T>, something: u32) -> DispatchResult {
			// Check that the extrinsic was signed and get the signer.
			// This function will return an error if the extrinsic is not signed.
			// https://docs.substrate.io/main-docs/build/origins/
			let who = ensure_signed(origin)?;

			// Update storage.
			<Something<T>>::put(something);

			// Emit an event.
			Self::deposit_event(Event::SomethingStored { something, who });
			// Return a successful DispatchResultWithPostInfo
			Ok(())
		} */

		// An example dispatchable that may throw a custom error.
/* 		#[pallet::call_index(1)]
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1).ref_time())]
		pub fn cause_error(origin: OriginFor<T>) -> DispatchResult {
			let _who = ensure_signed(origin)?;

			// Read a value from storage.
			match <Something<T>>::get() {
				// Return an error if the value has not been set.
				None => return Err(Error::<T>::NoneValue.into()),
				Some(old) => {
					// Increment the value read from storage; will error in the event of overflow.
					let new = old.checked_add(1).ok_or(Error::<T>::StorageOverflow)?;
					// Update the value in storage with the incremented result.
					<Something<T>>::put(new);
					Ok(())
				},
			}
		} */