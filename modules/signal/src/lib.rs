//
//           _______________________________ ________
//           \____    /\_   _____/\______   \\_____  \
//             /     /  |    __)_  |       _/ /   |   \
//            /     /_  |        \ |    |   \/    |    \
//           /_______ \/_______  / |____|_  /\_______  /
//                   \/        \/         \/         \/
//           Z  E  R  O  .  I  O     N  E  T  W  O  R  K
//           © C O P Y R I O T   2 0 7 5 @ Z E R O . I O

// This file is part of ZERO Network.
// Copyright (C) 2010-2020 ZERO Labs.
// SPDX-License-Identifier: Apache-2.0

#![cfg_attr(not(feature = "std"), no_std)]
#![feature(derive_default_enum)]

// TODO: harden checks on completion
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

use flow;
use control;

use frame_system::{ self as system, ensure_signed };
use frame_support::{
	decl_storage, decl_module, decl_event, decl_error,
	StorageValue, StorageMap,
	dispatch::DispatchResult, ensure,
	traits::{
		Currency,
		ReservableCurrency,
		Get,
		Randomness,
	}
};
use sp_core::{ Hasher, H256 };
use sp_std::prelude::*;
use codec::{ Encode, Decode };
use sp_runtime::traits::{ Hash, Zero };

#[cfg(feature = "std")]
use serde::{ Deserialize, Serialize };

use primitives::{ Balance, BlockNumber, Index, Moment};
use scale_info::TypeInfo;

//
//
//

#[derive(Encode, Decode, Clone, PartialEq, Default, Eq, PartialOrd, Ord, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[repr(u8)]
pub enum ProposalState {
	#[default]
	Init = 0,		// waiting for start block
	Active = 1,		// voting is active
	Accepted = 2,	// voters did approve
	Rejected = 3,	// voters did not approve
	Expired = 4,	// ended without votes
	Aborted = 5,	// sudo abort
	Finalized = 6,	// accepted withdrawal proposal is processed
}

#[derive(Encode, Decode, Clone, PartialEq, Default, Eq, PartialOrd, Ord, TypeInfo)]
#[repr(u8)]
pub enum ProposalType {
	#[default]
	General = 0,
	Multiple = 1,
	Member = 2,
	Withdrawal = 3,
	Spending = 4
}

#[derive(Encode, Decode, Clone, PartialEq, Default, Eq, PartialOrd, Ord, TypeInfo)]
#[repr(u8)]
pub enum VotingType {
	#[default]
	Simple = 0,   // votes across participating votes
	Token = 1,    // weight across participating votes
	Absolute = 2, // votes vs all eligible voters
	Quadratic = 3,
	Ranked = 4,
	Conviction = 5
}

type TitleText = Vec<u8>;
type CID = Vec<u8>;
// type ProposalType = u8;
// type VotingType = u8;

type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

#[derive(Encode, Decode, Default, Clone, PartialEq)]
pub struct Proposal<Hash, BlockNumber, ProposalType, VotingType> {
	proposal_id: Hash,
	context_id: Hash,
	proposal_type: ProposalType,
	voting_type: VotingType,
	start: BlockNumber,
	expiry: BlockNumber
}

#[derive(Encode, Decode, Default, Clone, PartialEq)]
pub struct ProposalMetadata<Balance> {
	title: Vec<u8>,
	cid: Vec<u8>,
	amount: Balance,
}

//
//
//

pub trait Config: system::Config + balances::Config + timestamp::Config + flow::Config + control::Config {
	type Currency: ReservableCurrency<Self::AccountId>;
	type Event: From<Event<Self>> + Into<<Self as system::Config>::Event>;
	type Nonce: Get<u64>;
	type Randomness: Randomness<Self::Hash>;
	type MaxProposalsPerBlock: Get<usize>;
	// type MaxDuration: Get<usize>;
}

// TODO: replace with config
const MAX_PROPOSALS_PER_BLOCK: usize = 3;
const MAX_PROPOSAL_DURATION: u32 = 864000; // 60 * 60 * 24 * 30 / 3

//
//
//

decl_storage! {
	trait Store for Module<T: Config> as Signal48 {

		/// Global status
		Proposals get(fn proposals): map hasher(blake2_128_concat) T::Hash => Proposal<T::Hash, T::BlockNumber, ProposalType, VotingType>;
		Metadata get(fn metadata): map hasher(blake2_128_concat) T::Hash => ProposalMetadata<T::Balance>;
		Owners get(fn owners): map hasher(blake2_128_concat) T::Hash => Option<T::AccountId>;
		/// Get the state of a proposal
       	ProposalStates get(fn proposal_states): map hasher(blake2_128_concat) T::Hash => ProposalState = ProposalState::Init;

		/// Maximum time limit for a proposal
		ProposalTimeLimit get(fn proposal_time_limit) config(): T::BlockNumber = T::BlockNumber::from(MAX_PROPOSAL_DURATION);

		/// All proposals
		ProposalsArray get(fn proposals_by_index): map hasher(blake2_128_concat) u64 => T::Hash;
		ProposalsCount get(fn proposals_count): u64;
		ProposalsIndex: map hasher(blake2_128_concat) T::Hash => u64;

		/// Proposals by campaign / org
		ProposalsByContextArray get(fn proposals_by_campaign_by_index): map hasher(blake2_128_concat)  (T::Hash, u64) => T::Hash;
		ProposalsByContextCount get(fn proposals_by_campaign_count): map hasher(blake2_128_concat) T::Hash => u64;
		ProposalsByContextIndex: map hasher(blake2_128_concat) (T::Hash, T::Hash) => u64;

		/// all proposals for a given context
		ProposalsByContext get(fn proposals_by_context): map hasher(blake2_128_concat) T::Hash => Vec<T::Hash>;

		/// Proposals by owner
		ProposalsByOwnerArray get(fn proposals_by_owner): map hasher(blake2_128_concat) (T::AccountId, u64) => T::Hash;
		ProposalsByOwnerCount get(fn proposals_by_owner_count): map hasher(blake2_128_concat) T::AccountId => u64;
		ProposalsByOwnerIndex: map hasher(blake2_128_concat) (T::AccountId, T::Hash) => u64;

		/// Proposals where voter participated
		ProposalsByVoter get(fn proposals_by_voter): map hasher(blake2_128_concat) T::AccountId => Vec<(T::Hash, bool)>;
		/// Proposal voters and votes by proposal
		ProposalVotesByVoters get(fn proposal_votes_by_voters): map hasher(blake2_128_concat) T::Hash => Vec<(T::AccountId, bool)>;
		/// Total proposals voted on by voter
		ProposalsByVoterCount get(fn proposals_by_voter_index): map hasher(blake2_128_concat) T::AccountId => u64;

		/// Proposals ending in a block
		ProposalsByBlock get(fn proposals_by_block): map hasher(blake2_128_concat) T::BlockNumber => Vec<T::Hash>;

		/// The amount of currency that a project has used
		CampaignBalanceUsed get(fn used_balance): map hasher(blake2_128_concat) T::Hash => T::Balance;

		/// The number of people who approve a proposal
		ProposalApprovers get(fn proposal_approvers): map hasher(blake2_128_concat) T::Hash => u64 = 0;
		/// The number of people who deny a proposal
		ProposalDeniers get(fn proposal_deniers): map hasher(blake2_128_concat) T::Hash => u64 = 0;
		/// Voters per proposal
		ProposalVoters get(fn proposal_voters): map hasher(blake2_128_concat) T::Hash => Vec<T::AccountId>;
		/// Voter count per proposal
		ProposalVotes get(fn proposal_votes): map hasher(blake2_128_concat) T::Hash => u64 = 0;

		/// Ack vs Nack
		ProposalSimpleVotes get(fn proposal_simple_votes): map hasher(blake2_128_concat) T::Hash => (u64,u64) = (0,0);
		/// User has voted on a proposal
		VotedBefore get(fn has_voted): map hasher(blake2_128_concat) (T::AccountId, T::Hash) => bool = false;
		// TODO: ProposalTotalEligibleVoters

		// TODO: ProposalApproversWeight
		// TODO: ProposalDeniersWeight
		// TODO: ProposalTotalEligibleWeight

		/// The total number of proposals
		Nonce: u64;
	}
}

//
//
//

decl_module! {
	pub struct Module<T: Config> for enum Call where origin: T::Origin {

		type Error = Error<T>;
		fn deposit_event() = default;

		// TODO: general proposal for a DAO
		#[weight = 5_000_000]
		fn general_proposal(
			origin,
			context_id: T::Hash,
			title: Vec<u8>,
			cid: Vec<u8>,
			start: T::BlockNumber,
			expiry: T::BlockNumber
		) -> DispatchResult {

			let sender = ensure_signed(origin)?;

			// active/existing dao?
			ensure!( <control::Module<T>>::body_state(&context_id) == control::ControlState::Active, Error::<T>::DAOInactive );

			// member of body?
			let member = <control::Module<T>>::body_member_state((&context_id,&sender));
			ensure!( member == control::ControlMemberState::Active, Error::<T>::AuthorizationError );

			// ensure that start and expiry are in bounds
			let current_block = <system::Module<T>>::block_number();
			// ensure!(start > current_block, Error::<T>::OutOfBounds );
			ensure!(expiry > current_block, Error::<T>::OutOfBounds );
			ensure!(expiry <= current_block + Self::proposal_time_limit(), Error::<T>::OutOfBounds );

			// ensure that number of proposals
			// ending in target block
			// do not exceed the maximum
			let proposals = Self::proposals_by_block(expiry);
			ensure!(proposals.len() < MAX_PROPOSALS_PER_BLOCK, "Maximum number of proposals is reached for the target block, try another block");

			//

			let proposal_type = ProposalType::General;
			let proposal_state = ProposalState::Active;
			let voting_type = VotingType::Simple;
			let nonce = Nonce::get();

			// generate unique id
			let phrase = b"just another proposal";
			let proposal_id = <T as Config>::Randomness::random(phrase);
			ensure!(!<Proposals<T>>::contains_key(&context_id), "Proposal id already exists");

			// proposal

			let new_proposal = Proposal {
				proposal_id: proposal_id.clone(),
				context_id: context_id.clone(),
				proposal_type,
				voting_type,
				start,
				expiry,
			};

			// metadata

			let metadata = ProposalMetadata {
				title: title,
				cid: cid,
				amount: T::Balance::zero()
			};

			//
			//
			//

			// check add
			let proposals_count = Self::proposals_count();
			let updated_proposals_count = proposals_count.checked_add(1).ok_or( Error::<T>::OverflowError)?;
			let proposals_by_campaign_count = Self::proposals_by_campaign_count(&context_id);
			let updated_proposals_by_campaign_count = proposals_by_campaign_count.checked_add(1).ok_or( Error::<T>::OverflowError )?;
			let proposals_by_owner_count = Self::proposals_by_owner_count(&sender);
			let updated_proposals_by_owner_count = proposals_by_owner_count.checked_add(1).ok_or( Error::<T>::OverflowError )?;

			// insert proposals
			Proposals::<T>::insert(proposal_id.clone(), new_proposal.clone());
			Metadata::<T>::insert(proposal_id.clone(), metadata.clone());
			Owners::<T>::insert(proposal_id.clone(), sender.clone());
			ProposalStates::<T>::insert(proposal_id.clone(), proposal_state);
			// update max per block
			ProposalsByBlock::<T>::mutate(expiry, |proposals| proposals.push(proposal_id.clone()));
			// update proposal map
			ProposalsArray::<T>::insert(&proposals_count, proposal_id.clone());
			ProposalsCount::put(updated_proposals_count);
			ProposalsIndex::<T>::insert(proposal_id.clone(), proposals_count);
			// update campaign map
			ProposalsByContextArray::<T>::insert((context_id.clone(), proposals_by_campaign_count.clone()), proposal_id.clone());
			ProposalsByContextCount::<T>::insert(context_id.clone(), updated_proposals_by_campaign_count);
			ProposalsByContextIndex::<T>::insert((context_id.clone(), proposal_id.clone()), proposals_by_campaign_count);
			ProposalsByContext::<T>::mutate( context_id.clone(), |proposals| proposals.push(proposal_id.clone()) );
			// update owner map
			ProposalsByOwnerArray::<T>::insert((sender.clone(), proposals_by_owner_count.clone()), proposal_id.clone());
			ProposalsByOwnerCount::<T>::insert(sender.clone(), updated_proposals_by_owner_count);
			ProposalsByOwnerIndex::<T>::insert((sender.clone(), proposal_id.clone()), proposals_by_owner_count);
			// init votes
			ProposalSimpleVotes::<T>::insert(context_id, (0,0));

			//
			//
			//

			// nonce++
			Nonce::mutate(|n| *n += 1);

			// deposit event
			Self::deposit_event(
				RawEvent::Proposal(
					sender,
					proposal_id
				)
			);
			Ok(())
		}

//
//
//

		// TODO: membership proposal for a DAO

		#[weight = 5_000_000]
		fn membership_proposal(
			origin,
			context: T::Hash,
			member: T::Hash,
			action: u8,
			start: T::BlockNumber,
			expiry: T::BlockNumber
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			// ensure active
			// ensure member
			// match action
			// action
			// deposit event
			Self::deposit_event(
				RawEvent::Proposal(
					sender,
					context
				)
			);
			Ok(())
		}

//
//
//

		//	create a withdrawal proposal
		//	origin must be controller of the campaign == controller of the dao
		//	beneficiary must be the treasury of the dao

		#[weight = 5_000_000]
		fn withdraw_proposal(
			origin,
			context_id: T::Hash,
			title: Vec<u8>,
			cid: Vec<u8>,
			amount: T::Balance,
			start: T::BlockNumber,
			expiry: T::BlockNumber,
		) -> DispatchResult {

			let sender = ensure_signed(origin)?;

			//	A C C E S S

			// ensure!( flow::Module::<T>::campaign_by_id(&context_id), Error::<T>::CampaignUnknown );
			let state = flow::Module::<T>::campaign_state(&context_id);
			ensure!( state == flow::FlowState::Success, Error::<T>::CampaignFailed );
			// let owner = flow::Module::<T>::campaign_owner(&context_id);
			// ensure!( sender == owner, Error::<T>::AuthorizationError );

			//	B O U N D S

			let current_block = <system::Module<T>>::block_number();
			// ensure!(start > current_block, Error::<T>::OutOfBounds );
			// ensure!(expiry > start, Error::<T>::OutOfBounds );
			// ensure!(expiry <= current_block + Self::proposal_time_limit(), Error::<T>::OutOfBounds );

			//	B A L A N C E

			let used_balance = Self::used_balance(&context_id);
			let total_balance = flow::Module::<T>::campaign_balance(context_id);
			let remaining_balance = total_balance - used_balance;
			ensure!(remaining_balance >= amount, Error::<T>::BalanceInsufficient );

			//	T R A F F I C

			let proposals = Self::proposals_by_block(expiry);
			ensure!(proposals.len() < MAX_PROPOSALS_PER_BLOCK, Error::<T>::TooManyProposals );

			//	C O N F I G

			let proposal_type = ProposalType::Withdrawal; // treasury
			let voting_type = VotingType::Simple; // votes
			let nonce = Nonce::get();
			let phrase = b"just another withdrawal";

			let proposal_id = <T as Config>::Randomness::random(phrase);
			ensure!(!Proposals::<T>::contains_key(&context_id), Error::<T>::HashCollision );

			let proposal = Proposal {
				proposal_id: proposal_id.clone(),
				context_id: context_id.clone(),
				proposal_type,
				voting_type,
				start,
				expiry,
			};

			let metadata = ProposalMetadata {
				title: title,
				cid: cid,
				amount,
			};

			//	C O U N T S

			let proposals_count = Self::proposals_count();
			let updated_proposals_count = proposals_count.checked_add(1).ok_or(Error::<T>::OverflowError)?;
			let proposals_by_campaign_count = Self::proposals_by_campaign_count(context_id);
			let updated_proposals_by_campaign_count = proposals_by_campaign_count.checked_add(1).ok_or(Error::<T>::OverflowError)?;
			let proposals_by_owner_count = Self::proposals_by_owner_count(&sender);
			let updated_proposals_by_owner_count = proposals_by_owner_count.checked_add(1).ok_or(Error::<T>::OverflowError)?;

			//	W R I T E

			Proposals::<T>::insert(&proposal_id, proposal.clone());
			Metadata::<T>::insert(&proposal_id, metadata.clone());
			Owners::<T>::insert(&proposal_id, sender.clone());
			ProposalStates::<T>::insert(proposal_id.clone(), ProposalState::Active);

			ProposalsByBlock::<T>::mutate(expiry, |proposals| proposals.push(proposal_id.clone()));
			ProposalsArray::<T>::insert(&proposals_count, proposal_id.clone());
			ProposalsCount::put(updated_proposals_count);
			ProposalsIndex::<T>::insert(proposal_id.clone(), proposals_count);
			ProposalsByContextArray::<T>::insert((context_id.clone(), proposals_by_campaign_count.clone()), proposal_id.clone());
			ProposalsByContextCount::<T>::insert(context_id.clone(), updated_proposals_by_campaign_count);
			ProposalsByContextIndex::<T>::insert((context_id.clone(), proposal_id.clone()), proposals_by_campaign_count);
			ProposalsByOwnerArray::<T>::insert((sender.clone(), proposals_by_owner_count.clone()), proposal_id.clone());
			ProposalsByOwnerCount::<T>::insert(sender.clone(), updated_proposals_by_owner_count);
			ProposalsByOwnerIndex::<T>::insert((sender.clone(), proposal_id.clone()), proposals_by_owner_count);
			ProposalsByContext::<T>::mutate( context_id.clone(), |proposals| proposals.push(proposal_id.clone()) );

			// ++

			Nonce::mutate(|n| *n += 1);

			//	E V E N T

			Self::deposit_event(
				RawEvent::ProposalCreated(
					sender,
					context_id,
					proposal_id,
					amount,
					expiry
				)
			);
			Ok(())

		}

		// TODO:
		// voting vs staking, e.g.
		// 1. token weighted and democratic voting require yes/no
		// 2. conviction voting requires ongoing staking
		// 3. quadratic voting

		#[weight = 5_000_000]
		fn simple_vote(
			origin,
			proposal_id: T::Hash,
			vote: bool
		) -> DispatchResult {

			let sender = ensure_signed(origin)?;

			// Ensure the proposal exists
			ensure!(<Proposals<T>>::contains_key(&proposal_id), Error::<T>::ProposalUnknown);

			// Ensure the proposal has not ended
			let proposal_state = Self::proposal_states(&proposal_id);
			ensure!(proposal_state == ProposalState::Active, Error::<T>::ProposalEnded);

			// Ensure the contributor did not vote before
			ensure!(!<VotedBefore<T>>::get((sender.clone(), proposal_id.clone())), Error::<T>::AlreadyVoted);

			// Get the proposal
			let proposal = Self::proposals(&proposal_id);
			// Ensure the proposal is not expired
			ensure!(<system::Module<T>>::block_number() < proposal.expiry, Error::<T>::ProposalExpired);

			// TODO:
			// ensure origin is one of:
			// a. member when the proposal is general
			// b. contributor when the proposal is a withdrawal request
			// let sender_balance = <campaign::Module<T>>::campaign_contribution(proposal.campaign_id, sender.clone());
			// ensure!( sender_balance > T::Balance::from(0), "You are not a contributor of this Campaign");

			match &proposal.proposal_type {
				// DAO Democratic Proposal
				// simply one member one vote yes / no,
				// TODO: ratio definable, now > 50% majority wins
				ProposalType::General => {

					let (mut yes, mut no) = Self::proposal_simple_votes(&proposal_id);

					match vote {
						true => {
							yes = yes.checked_add(1).ok_or(Error::<T>::OverflowError)?;
							let proposal_approvers = Self::proposal_approvers(&proposal_id);
							let updated_proposal_approvers = proposal_approvers.checked_add(1).ok_or(Error::<T>::OverflowError)?;
							ProposalApprovers::<T>::insert(
								proposal_id.clone(),
								updated_proposal_approvers.clone()
							);
						},
						false => {
							no = no.checked_add(1).ok_or(Error::<T>::OverflowError)?;
							let proposal_deniers = Self::proposal_deniers(&proposal_id);
							let updated_proposal_deniers = proposal_deniers.checked_add(1).ok_or(Error::<T>::OverflowError)?;
							ProposalDeniers::<T>::insert(
								proposal_id.clone(),
								updated_proposal_deniers.clone()
							);
						}
					}

					ProposalSimpleVotes::<T>::insert(
						proposal_id.clone(),
						(yes,no)
					);

				},
				// 50% majority over total number of campaign contributors
				ProposalType::Withdrawal => {

					let (mut yes, mut no) = Self::proposal_simple_votes(&proposal_id);

					match vote {
						true => {
							yes = yes.checked_add(1).ok_or(Error::<T>::OverflowError)?;

							let current_approvers = Self::proposal_approvers(&proposal_id);
							let updated_approvers = current_approvers.checked_add(1).ok_or(Error::<T>::OverflowError)?;
							ProposalApprovers::<T>::insert(proposal_id.clone(), updated_approvers.clone());

							// TODO: make this variable
							let contributors = flow::Module::<T>::campaign_contributors_count(proposal.context_id);
							let threshold = contributors.checked_div(2).ok_or(Error::<T>::DivisionError)?;
							if updated_approvers > threshold {
								Self::unlock_balance(proposal_id, updated_approvers)?;
							}
							// remove
							let proposal_approvers = Self::proposal_approvers(&proposal_id);
							let updated_proposal_approvers = proposal_approvers.checked_add(1).ok_or(Error::<T>::OverflowError)?;
							ProposalApprovers::<T>::insert(
								proposal_id.clone(),
								updated_proposal_approvers.clone()
							);

						},
						false => {
							no = no.checked_add(1).ok_or(Error::<T>::OverflowError)?;
							// remove
							let proposal_deniers = Self::proposal_deniers(&proposal_id);
							let updated_proposal_deniers = proposal_deniers.checked_add(1).ok_or(Error::<T>::OverflowError)?;
							ProposalDeniers::<T>::insert(
								proposal_id.clone(),
								updated_proposal_deniers.clone()
							);
						}
					}

					ProposalSimpleVotes::<T>::insert(
						proposal_id.clone(),
						(yes,no)
					);


				},

				// Campaign Token Weighted Proposal
				// total token balance yes vs no
				// TODO: ratio definable, now > 50% majority wins
				// ProposalType:: => {
				// },

				// Membership Voting
				// simply one token one vote yes / no,
				// TODO: ratio definable, now simple majority wins
				ProposalType::Member => {
					// approve
					// deny
					// kick
					// ban
				},
				// default
				_ => {
				},
			}

			VotedBefore::<T>::insert( ( &sender, proposal_id.clone() ), true );
			ProposalsByVoterCount::<T>::mutate( &sender, |v| *v +=1 );
			ProposalVotesByVoters::<T>::mutate(&proposal_id, |votings| votings.push(( sender.clone(), vote.clone() )) );
			ProposalsByVoter::<T>::mutate( &sender, |votings| votings.push((proposal_id.clone(), vote)));

			let mut voters = ProposalVoters::<T>::get(&proposal_id);
			match voters.binary_search(&sender) {
				Ok(_) => {}, // should never happen
				Err(index) => {
					voters.insert(index, sender.clone());
					ProposalVoters::<T>::insert( &proposal_id, voters );
				}
			}

			// dispatch vote event
			Self::deposit_event(
				RawEvent::ProposalVoted(
					sender,
					proposal_id.clone(),
					vote
				)
			);
			Ok(())

		}

		fn on_finalize() {

			// i'm still jenny from the block
			let block_number = <system::Module<T>>::block_number();
			let proposal_hashes = Self::proposals_by_block(block_number);

			for proposal_id in &proposal_hashes {

				let mut proposal_state = Self::proposal_states(&proposal_id);
				if proposal_state != ProposalState::Active { continue };

				let proposal = Self::proposals(&proposal_id);

				// TODO:
				// a. result( accepted, rejected )
				// b. result( accepted, rejected, total_allowed )
				// c. result( required_majority, staked_accept, staked_reject, slash_amount )
				// d. threshold reached
				// e. conviction

				match &proposal.proposal_type {
					ProposalType::General => {
						// simple vote
						let (yes,no) = Self::proposal_simple_votes(&proposal_id);
						if yes > no { proposal_state = ProposalState::Accepted; }
						if yes < no { proposal_state = ProposalState::Rejected; }
						if yes == 0 && no == 0 { proposal_state = ProposalState::Expired; }
					},
					ProposalType::Withdrawal => {
						// treasury
						// 50% majority of eligible voters
						let (yes,no) = Self::proposal_simple_votes(&proposal_id);
						let context = proposal.context_id.clone();
						let contributors = flow::Module::<T>::campaign_contributors_count(context);
						// TODO: dynamic threshold
						let threshold = contributors.checked_div(2).ok_or(Error::<T>::DivisionError);
						match threshold {
							Ok(t) => {
								if yes > t {
									proposal_state = ProposalState::Accepted;
									Self::unlock_balance( proposal.proposal_id, yes );
								} else {
									proposal_state = ProposalState::Rejected;
								}
							},
							Err(err) => {  }
						}
					},
					ProposalType::Member => {
						// membership
						//
					},
					_ => {
						// no result - fail
						proposal_state = ProposalState::Expired;
					}
				}

				<ProposalStates<T>>::insert(&proposal_id, proposal_state.clone());

				match proposal_state {
					ProposalState::Accepted => {
						Self::deposit_event(
							RawEvent::ProposalApproved(proposal_id.clone())
						);
					},
					ProposalState::Rejected => {
						Self::deposit_event(
							RawEvent::ProposalRejected(proposal_id.clone())
						);
					},
					ProposalState::Expired => {
						Self::deposit_event(
							RawEvent::ProposalExpired(proposal_id.clone())
						);
					},
					_ => {}
				}

			}



		}

	}
}

//
//
//

impl<T:Config> Module<T> {

	// TODO: DISCUSSION
	// withdrawal proposals are accepted
	// when the number of approvals is higher
	// than the number of rejections
	// accepted / denied >= 1
	fn unlock_balance(
		proposal_id: T::Hash,
		supported_count: u64
	) -> DispatchResult {

		// Get proposal and metadata
		let proposal = Self::proposals(proposal_id.clone());
		let metadata = Self::metadata(proposal_id.clone());

		// Ensure sufficient balance
		let proposal_balance = metadata.amount;
		let total_balance = <flow::Module<T>>::campaign_balance(proposal.context_id);

		// let used_balance = Self::balance_used(proposal.context_id);
		let used_balance = <CampaignBalanceUsed<T>>::get(proposal.context_id);
		let available_balance = total_balance - used_balance.clone();
		ensure!(available_balance >= proposal_balance, Error::<T>::BalanceInsufficient );

		// Get the owner of the campaign
		let owner = <Owners<T>>::get(&proposal_id).ok_or("No owner for proposal")?;

		// get treasury account for related body and unlock balance
		let body = flow::Module::<T>::campaign_org(&proposal.context_id);
		let treasury_account = control::Module::<T>::body_treasury(&body);
		let _ = <balances::Module<T>>::unreserve(&treasury_account, proposal_balance);

		// Change the used amount
		let new_used_balance = used_balance + proposal_balance;
		<CampaignBalanceUsed<T>>::insert(proposal.context_id, new_used_balance);

		// proposal completed
		let proposal_state = ProposalState::Finalized;
		<ProposalStates<T>>::insert(proposal_id.clone(), proposal_state);

		<Proposals<T>>::insert(proposal_id.clone(), proposal.clone());

		Self::deposit_event(
			RawEvent::WithdrawalGranted(
				proposal_id,
				proposal.context_id,
				body
			)
		);
		Ok(())

	}
}

//
//	E V E N T S
//

decl_event!(
	pub enum Event<T> where
		<T as system::Config>::AccountId,
		<T as system::Config>::Hash,
		<T as balances::Config>::Balance,
		<T as system::Config>::BlockNumber
	{
		Proposal(AccountId, Hash),
		ProposalCreated(AccountId, Hash, Hash, Balance, BlockNumber),
		ProposalVoted(AccountId, Hash, bool),
		ProposalFinalized(Hash, u8),
		ProposalApproved(Hash),
		ProposalRejected(Hash),
		ProposalExpired(Hash),
		ProposalAborted(Hash),
		ProposalError(Hash, Vec<u8>),
		WithdrawalGranted(Hash,Hash,Hash),
	}
);

//
//	E R R O R S
//

decl_error! {
	pub enum Error for Module<T: Config> {

		/// Proposal Ended
		ProposalEnded,
		/// Proposal Expired
		ProposalExpired,
		/// Already Voted
		AlreadyVoted,
		/// Proposal Unknown
		ProposalUnknown,
		/// DAO Inactive
		DAOInactive,
		/// Authorization Error
		AuthorizationError,
		/// Tangram Creation Failed
		TangramCreationError,
		/// Out Of Bounds Error
		OutOfBounds,
		/// Unknown Error
		UnknownError,
		///MemberExists
		MemberExists,
		/// Unknown Campaign
		CampaignUnknown,
		/// Campaign Failed
		CampaignFailed,
		/// Balance Too Low
		BalanceInsufficient,
		/// Hash Collision
		HashCollision,
		/// Unknown Account
		UnknownAccount,
		/// Too Many Proposals for block
		TooManyProposals,
		/// Overflow Error
		OverflowError,
		/// Division Error
		DivisionError,
	}
}
