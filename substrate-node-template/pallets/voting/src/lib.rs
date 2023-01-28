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
	use frame_system::pallet_prelude::{*, OriginFor};

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

		type VoteRemovalThreshold: Get<u32>;
		
		type MaxVoters: Get<u32>;
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
		pub vote_decision: VoteDecision,
		pub locked: bool
	}

	#[derive(Encode, Debug, Decode, Clone, TypeInfo, MaxEncodedLen, Eq, PartialEq)]
	pub enum VoteDecision {
		Aye(u32),
		Nay(u32)
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
		///A voter canceled his vote for an ongoing proposal
		VoteCanceled { proposal_id: ProposalId, who: T::AccountId},
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
		ProposalAlreadyEnded,
		///Balance for the current vote has already been unreserved.
		BalanceAlreadyUnocked,
		///The time left of the propossal is passed the threshold that allows to reduce or cancel votes.
		PassedRemovalThreshold, 
		///The proposal is still in progress, therefore the user can't unlock the balance.
		ProposalInProgress,
		///Overflow when performing an operation
		Overflow
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
			who: T::AccountId
		) -> DispatchResult {
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
			vote_decision: VoteDecision
		) -> DispatchResult {
			//Verify sender is part of register voters
			let who: T::AccountId = ensure_signed(origin)?;
			ensure!(Self::is_registered(&who), Error::<T>::VoterIsNotRegistered);

			let proposal = Self::get_proposal(&proposal_id).ok_or(Error::<T>::ProposalNotFound)?;

			//Check that propossal is not passed removal_treshold
			let current_block_number = <frame_system::Pallet<T>>::block_number();
			ensure!(
				proposal.time_period>current_block_number && proposal.status == ProposalStatus::InProgress, 
				Error::<T>::ProposalAlreadyEnded
			);
			
			//Verify if voter already casted vote
			ensure!(!Self::vote_casted(&who, &proposal_id), Error::<T>::VoteAlreadyCasted);

			let vote_amount = match vote_decision {
				VoteDecision::Aye(v) => v,
				VoteDecision::Nay(v) => v
			};
			
			ensure!(vote_amount >0, Error::<T>::InvalidVoteAmount);

			//Reserve balance corresponding to vote amount^2.
			let amount_to_reserve: u32 = (vote_amount).checked_pow(2).ok_or(Error::<T>::Overflow)?;
			T::Currency::reserve(&who, amount_to_reserve.into())?;
			
			let vote = Vote { vote_decision: vote_decision.clone(), locked: true};

			//Insert vote and update proposals
			<Votes<T>>::insert(who.clone(), proposal_id, vote.clone());

			<Proposals<T>>::mutate(proposal_id, |proposal|{
				if let Some(p) = proposal.as_mut() {
					match vote_decision {
						VoteDecision::Aye(v) => p.ayes += v,
						VoteDecision::Nay(v) => p.nays += v
					}
				}
			});
			
			Self::deposit_event(Event::VoteCasted {proposal_id, who});
			Ok(())
		}


 		#[pallet::call_index(5)]
		#[pallet::weight(0)]
		pub fn update_vote(
			origin: OriginFor<T>, 
			proposal_id: ProposalId,
			new_vote_decision: VoteDecision
		) -> DispatchResult {
			//Verify sender is part of register voters and vote exists
			let who: T::AccountId = ensure_signed(origin)?;
			ensure!(Self::is_registered(&who), Error::<T>::VoterIsNotRegistered);

			let mut proposal = Self::get_proposal(&proposal_id).ok_or(Error::<T>::ProposalNotFound)?;
						//Check that propossal is not passed removal_treshold
			let current_block_number = <frame_system::Pallet<T>>::block_number();
			ensure!(
				proposal.time_period>current_block_number && proposal.status == ProposalStatus::InProgress, 
				Error::<T>::ProposalAlreadyEnded
			);
			
			//Get vote and verify if it exists
			let current_vote = <Votes<T>>::try_get(&who, &proposal_id).ok().ok_or(Error::<T>::VoteNotFound)?;

			let current_amount: u32 = match current_vote.vote_decision {
				VoteDecision::Aye(v) => {
					proposal.ayes = proposal.ayes.saturating_sub(v);
					v
				},
				VoteDecision::Nay(v) => {
					proposal.nays = proposal.nays.saturating_sub(v);
					v
				},
			};

			let new_amount = match new_vote_decision {
				VoteDecision::Aye(v) => {
					proposal.ayes += v;
					v
				},
				VoteDecision::Nay(v) => {
					//Check threshold
					ensure!(!Self::passed_removal_threshold(&proposal.time_period), Error::<T>::PassedRemovalThreshold);
					proposal.nays += v;
					v
				}
			};
			
			ensure!(new_amount != 0, Error::<T>::InvalidUpdateAmount);

			let current_amount_pow: u32 = current_amount.checked_pow(2).ok_or(Error::<T>::Overflow)?;
			let new_amount_pow: u32 = new_amount.checked_pow(2).ok_or(Error::<T>::Overflow)?;

			//Modify reserved amount
			match new_amount.cmp(&current_amount){
				Ordering::Greater => {T::Currency::reserve(&who, (new_amount_pow-current_amount_pow).into())?;},
				Ordering::Less => {T::Currency::unreserve(&who, (current_amount_pow-new_amount_pow).into());},
				_ => (),
			};


			let new_vote = Vote {vote_decision: new_vote_decision, locked: true};

			<Votes<T>>::insert(who.clone(), proposal_id, new_vote);
			<Proposals<T>>::insert(proposal_id, proposal);
			Self::deposit_event(Event::VoteCasted {proposal_id, who});

			Ok(())
		}
		
		
		#[pallet::call_index(9)]
		#[pallet::weight(0)]
		pub fn cancel_vote(
			origin: OriginFor<T>, 
			proposal_id: ProposalId,
		) -> DispatchResult {
			let who: T::AccountId = ensure_signed(origin)?;
			//Allows to calculate treshold

			let mut proposal = Self::get_proposal(&proposal_id).ok_or(Error::<T>::ProposalNotFound)?;
			let vote: Vote = <Votes<T>>::try_get(who.clone(), proposal_id).ok().ok_or(Error::<T>::VoteNotFound)?;
			let current_block_number = <frame_system::Pallet<T>>::block_number();

			ensure!(
				proposal.time_period>=current_block_number && proposal.status == ProposalStatus::InProgress, 
				Error::<T>::ProposalAlreadyEnded
			);
			
			//Check that propossal is not passed removal_treshold
			ensure!(!Self::passed_removal_threshold(&proposal.time_period), Error::<T>::PassedRemovalThreshold);

			match vote.vote_decision {
				VoteDecision::Aye(v) =>  proposal.ayes = proposal.ayes.saturating_sub(v),
				VoteDecision::Nay(v) =>  proposal.nays = proposal.nays.saturating_sub(v),
			}

			<Proposals<T>>::insert(proposal_id, proposal);
			<Votes<T>>::remove(who.clone(), proposal_id);

			let vote_amount = match vote.vote_decision {
				VoteDecision::Aye(v) => v,
				VoteDecision::Nay(v) => v
			};

			//unreserve balance corresponding to the vote (amount^2).
			let amount_to_unreserve: u32 = (vote_amount).checked_pow(2).ok_or(Error::<T>::Overflow)?;
			T::Currency::unreserve(&who, amount_to_unreserve.into());

			Self::deposit_event(Event::VoteCanceled{proposal_id, who});

			Ok(())
		} 

		
		#[pallet::call_index(7)]
		#[pallet::weight(0)]
		pub fn finish_proposal(
			origin: OriginFor<T>, 
			proposal_id: ProposalId,
		) -> DispatchResult {
			//Verify sender is part of register voters and vote exists
			let who: T::AccountId = ensure_signed(origin)?;
			ensure!(Self::is_registered(&who), Error::<T>::VoterIsNotRegistered);

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
			proposal_id: ProposalId
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let proposal: Proposal<T> = Self::get_proposal(&proposal_id).ok_or(Error::<T>::ProposalNotFound)?;
			ensure!(proposal.status != ProposalStatus::InProgress, Error::<T>::ProposalInProgress);

			let mut vote: Vote = <Votes<T>>::try_get(who.clone(), proposal_id).ok().ok_or(Error::<T>::VoteNotFound)?;
			ensure!(vote.locked, Error::<T>::BalanceAlreadyUnocked);
			vote.locked = false;
			<Votes<T>>::insert(who.clone(), proposal_id, vote.clone());

			let vote_amount = match vote.vote_decision {
				VoteDecision::Aye(v) => v,
				VoteDecision::Nay(v) => v
			}; 
			//unreserve balance corresponding to the vote (amount^2).
			let amount_to_unreserve: u32 = (vote_amount).checked_pow(2).ok_or(Error::<T>::Overflow)?;
			T::Currency::unreserve(&who, amount_to_unreserve.into());

			//Should add event??

			Ok(())
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
		pub fn passed_removal_threshold(end_time_period: &T::BlockNumber) -> bool {
			let current_block_number = <frame_system::Pallet<T>>::block_number();

			let difference = *end_time_period - current_block_number;
			difference < T::VoteRemovalThreshold::get().into()
		}
	}
}

