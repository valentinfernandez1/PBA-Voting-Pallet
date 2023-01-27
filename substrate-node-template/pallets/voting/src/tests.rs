use crate::{mock::*, Error, Event, Proposal, ProposalStatus, Vote};
use frame_support::{assert_noop, assert_ok};

#[test]
fn voter_registration(){
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		//Register new voter
		assert_ok!(Voting::register_voter(RuntimeOrigin::root(), 2));
		assert!(Voting::is_registered(&2));
		assert!(System::events().len() == 1);
		System::assert_last_event(Event::VoterRegistered { who: 2 }.into());

		//Try to re-register the same voter;
		assert_noop!(Voting::register_voter(RuntimeOrigin::root(), 2), Error::<Test>::AlreadyRegistered);
	});
}

#[test]
fn register_invalid_origin(){
	new_test_ext().execute_with(|| {
		assert_noop!(Voting::register_voter(RuntimeOrigin::signed(1), 2), sp_runtime::DispatchError::BadOrigin);
	});
}

#[test]
fn make_proposal(){
	new_test_ext().execute_with(|| {
		System::set_block_number(82);
		let initial_proposal_id = Voting::get_proposal_counter();
		let new_proposal_id = initial_proposal_id + 1; 

		assert_ok!(Voting::make_proposal(RuntimeOrigin::signed(1), sp_core::H256::zero(), 90));
		assert!(Voting::proposal_exists(new_proposal_id));

		assert!(System::events().len() == 1);
		System::assert_last_event(Event::ProposalSubmitted { proposal_id: new_proposal_id, who: 1 }.into());

		assert_eq!(initial_proposal_id+1, Voting::get_proposal_counter());
	});
} 


#[test]
fn proposal_time_low(){
	new_test_ext().execute_with(|| {
		System::set_block_number(82);
		
		assert_noop!(Voting::make_proposal(
			RuntimeOrigin::signed(1), 
			sp_core::H256::zero(), 
			80
		), Error::<Test>::TimePeriodToLow );

	});
}

#[test]
fn update_proposal(){
	new_test_ext().execute_with(|| {
		System::set_block_number(30);
		let proposal_id = Voting::get_proposal_counter() + 1;

		assert_ok!(Voting::make_proposal(RuntimeOrigin::signed(1), sp_core::H256::zero(), 90));
		assert_ok!(Voting::increase_proposal_time(RuntimeOrigin::signed(1), proposal_id, 95));

		System::assert_last_event(Event::ProposalUpdated { proposal_id, end_block: 95 }.into());

		let updated_proposal: Proposal<Test> = Voting::get_proposal(&proposal_id).unwrap();
		assert_eq!(updated_proposal.time_period, 95)
	});
}

#[test]
fn update_proposal_invalid(){
	new_test_ext().execute_with(|| {
		System::set_block_number(30);
		let proposal_id = Voting::get_proposal_counter() + 1;

		assert_ok!(Voting::make_proposal(RuntimeOrigin::signed(1), sp_core::H256::zero(), 90));
		assert_noop!(
			Voting::increase_proposal_time(RuntimeOrigin::signed(1), proposal_id, 75),
			Error::<Test>::TimePeriodToLow
		);
	});
}

#[test]
fn invalid_proposer_update(){
	new_test_ext().execute_with(|| {
		System::set_block_number(30);
		let proposal_id = Voting::get_proposal_counter() + 1;

		assert_ok!(Voting::make_proposal(RuntimeOrigin::signed(1), sp_core::H256::zero(), 90));
		assert_noop!(
			Voting::increase_proposal_time(RuntimeOrigin::signed(2), proposal_id, 95),
			Error::<Test>::Unauthorized
		);
	});
}


#[test]
fn proposal_canceled(){
	new_test_ext().execute_with(|| {
		System::set_block_number(30);
		let proposal_id = Voting::get_proposal_counter() + 1;

		assert_ok!(Voting::make_proposal(RuntimeOrigin::signed(1), sp_core::H256::zero(), 90));
		assert_ok!(Voting::cancel_proposal(RuntimeOrigin::signed(1), proposal_id));
		System::assert_last_event(Event::ProposalCanceled { proposal_id }.into());

		let updated_proposal: Proposal<Test> = Voting::get_proposal(&proposal_id).unwrap();
		assert_eq!(updated_proposal.status, ProposalStatus::Canceled);
	});
}

#[test]
fn proposal_cant_be_canceled(){
	new_test_ext().execute_with(|| {
		System::set_block_number(30);
		let proposal_id = Voting::get_proposal_counter() + 1;

		assert_ok!(Voting::make_proposal(RuntimeOrigin::signed(1), sp_core::H256::zero(), 90));

		System::set_block_number(100);

		assert_noop!(Voting::cancel_proposal(RuntimeOrigin::signed(1), proposal_id), Error::<Test>::TimePeriodToLow);
	});
}

#[test]
fn cast_valid_votes(){
	new_test_ext().execute_with(|| {
		//Initial setup
		System::set_block_number(1);
		let proposal_id = Voting::get_proposal_counter() + 1;
		assert_ok!(Voting::register_voter(RuntimeOrigin::root(), 1));
		assert_ok!(Voting::register_voter(RuntimeOrigin::root(), 2));
		assert_ok!(Voting::make_proposal(RuntimeOrigin::signed(1), sp_core::H256::zero(), 90));

		let mut vote = Vote {in_favor: true, amount: 2};
		assert_ok!(Voting::vote(RuntimeOrigin::signed(1), proposal_id, vote.clone()));
		System::assert_last_event(Event::VoteCasted { proposal_id, who: 1 }.into());

		assert!(Voting::vote_casted(&1, &proposal_id));
		let updated_proposal: Proposal<Test> = Voting::get_proposal(&proposal_id).unwrap();
		assert_eq!(updated_proposal.ayes, vote.amount);

		vote.in_favor = false;
		assert_ok!(Voting::vote(RuntimeOrigin::signed(2), proposal_id, vote.clone()));
		System::assert_last_event(Event::VoteCasted { proposal_id, who: 2 }.into());

		assert!(Voting::vote_casted(&2, &proposal_id));
		let updated_proposal: Proposal<Test> = Voting::get_proposal(&proposal_id).unwrap();
		assert_eq!(updated_proposal.nays, vote.amount);
	});
}

#[test]
fn voter_not_registered(){
	new_test_ext().execute_with(|| {
		//Initial setup
		System::set_block_number(1);
		let proposal_id = Voting::get_proposal_counter() + 1;
		assert_ok!(Voting::make_proposal(RuntimeOrigin::signed(1), sp_core::H256::zero(), 90));

		let vote = Vote {in_favor: true, amount: 2};
		assert_noop!(Voting::vote(RuntimeOrigin::signed(1), proposal_id, vote), Error::<Test>::VoterIsNotRegistered);
	});
}

#[test]
fn vote_already_casted(){
	new_test_ext().execute_with(|| {
		//Initial setup
		System::set_block_number(1);
		let proposal_id = Voting::get_proposal_counter() + 1;
		assert_ok!(Voting::register_voter(RuntimeOrigin::root(), 1));
		assert_ok!(Voting::make_proposal(RuntimeOrigin::signed(1), sp_core::H256::zero(), 90));

		let vote = Vote {in_favor: true, amount: 2};
		assert_ok!(Voting::vote(RuntimeOrigin::signed(1), proposal_id, vote.clone()));
		assert_noop!(Voting::vote(RuntimeOrigin::signed(1), proposal_id, vote.clone()), Error::<Test>::VoteAlreadyCasted);
	});
}

#[test]
fn invalid_vote_amount(){
	new_test_ext().execute_with(|| {
		//Initial setup
		System::set_block_number(1);
		let proposal_id = Voting::get_proposal_counter() + 1;
		assert_ok!(Voting::register_voter(RuntimeOrigin::root(), 1));
		assert_ok!(Voting::make_proposal(RuntimeOrigin::signed(1), sp_core::H256::zero(), 90));

		let vote = Vote {in_favor: true, amount: 0};
		assert_noop!(Voting::vote(RuntimeOrigin::signed(1), proposal_id, vote.clone()), Error::<Test>::InvalidVoteAmount);
	});
}

#[test]
fn proposal_passed(){
	new_test_ext().execute_with(||{
		let proposal_id = Voting::get_proposal_counter() + 1;
		assert_ok!(Voting::register_voter(RuntimeOrigin::root(), 1));
		assert_ok!(Voting::make_proposal(RuntimeOrigin::signed(1), sp_core::H256::zero(), 5));

		let vote = Vote {in_favor: true, amount: 1};
		assert_ok!(Voting::vote(RuntimeOrigin::signed(1), proposal_id, vote.clone()));

		System::set_block_number(6);

		assert_ok!(Voting::finish_proposal(RuntimeOrigin::signed(1), proposal_id));
		System::assert_last_event(Event::ProposalEnded { proposal_id, status: ProposalStatus::Passed }.into());
	});
}

#[test]
fn proposal_rejected(){
	new_test_ext().execute_with(||{
		let proposal_id = Voting::get_proposal_counter() + 1;
		assert_ok!(Voting::register_voter(RuntimeOrigin::root(), 1));
		assert_ok!(Voting::make_proposal(RuntimeOrigin::signed(1), sp_core::H256::zero(), 5));

		let vote = Vote {in_favor: false, amount: 1};
		assert_ok!(Voting::vote(RuntimeOrigin::signed(1), proposal_id, vote.clone()));

		System::set_block_number(6);

		assert_ok!(Voting::finish_proposal(RuntimeOrigin::signed(1), proposal_id));
		System::assert_last_event(Event::ProposalEnded { proposal_id, status: ProposalStatus::Rejected }.into());
	});
}

#[test]
fn proposal_tied(){
	new_test_ext().execute_with(||{
		let proposal_id = Voting::get_proposal_counter() + 1;
		assert_ok!(Voting::register_voter(RuntimeOrigin::root(), 1));
		assert_ok!(Voting::make_proposal(RuntimeOrigin::signed(1), sp_core::H256::zero(), 5));

		System::set_block_number(6);

		assert_ok!(Voting::finish_proposal(RuntimeOrigin::signed(1), proposal_id));
		System::assert_last_event(Event::ProposalEnded { proposal_id, status: ProposalStatus::Tied }.into());
	});
}

#[test]
fn finish_proposal_fails_if_canceled(){
	new_test_ext().execute_with(||{
		let proposal_id = Voting::get_proposal_counter() + 1;
		assert_ok!(Voting::register_voter(RuntimeOrigin::root(), 1));
		assert_ok!(Voting::make_proposal(RuntimeOrigin::signed(1), sp_core::H256::zero(), 5));

		System::set_block_number(6);

		assert_ok!(Voting::finish_proposal(RuntimeOrigin::signed(1), proposal_id));
		System::assert_last_event(Event::ProposalEnded { proposal_id, status: ProposalStatus::Tied }.into());
	});
}

#[test]
fn finish_proposal_early_rejects(){
	todo!()
}

/* #[test]
fn it_works_for_default_value() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		// Dispatch a signed extrinsic.
		assert_ok!(Voting::do_something(RuntimeOrigin::signed(1), 42));
		// Read pallet storage and assert an expected result.
		assert_eq!(Voting::something(), Some(42));
		// Assert that the correct event was deposited
		System::assert_last_event(Event::SomethingStored { something: 42, who: 1 }.into());
	});
}

#[test]
fn correct_error_for_none_value() {
	new_test_ext().execute_with(|| {
		// Ensure the expected error is thrown when no value is present.
		assert_noop!(Voting::cause_error(RuntimeOrigin::signed(1)), Error::<Test>::NoneValue);
	});
} */
