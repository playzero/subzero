//
//           _______________________________ ________
//           \____    /\_   _____/\______   \\_____  \
//             /     /  |    __)_  |       _/ /   |   \
//            /     /_  |        \ |    |   \/    |    \
//           /_______ \/_______  / |____|_  /\_______  /
//                   \/        \/         \/         \/
//           Z  E  R  O  .  I  O     N  E  T  W  O  R  K
//           © C O P Y R I O T   2 0 7 5 @ Z E R O . I O

// crowdfunding
// campaign treasury

// 1. proposal withdrawal (unreserve) as creator from successful campaign
// 2. approve withdrawals (unreserve) as investor from successfully funded campaigns

#![cfg_attr(not(feature = "std"), no_std)]

// TODO: harden checks on completion
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
// only on nightly
// #![feature(const_fn_fn_ptr_basics)]

use crate::campaign;

use frame_system::{ self as system, ensure_signed };
use frame_support::{
	decl_storage, decl_module, decl_event, decl_error,
	StorageValue, StorageMap,
	dispatch::DispatchResult, ensure,
	traits::{Currency, ReservableCurrency}
};
use sp_core::{ Hasher, H256 };
use sp_std::prelude::*;
use codec::{ Encode, Decode };
// TODO: replace deprecated
use sp_runtime::traits::{As, Hash, Zero};

// #[cfg_attr(feature = "std", derive(Debug))]

//
//
//
//
//

#[derive(Encode, Decode, Default, Clone, PartialEq)]
pub struct Proposal<Hash, Balance, BlockNumber>{
	// unique proposal id
	proposal_id: Hash,
	// unique campaign id
	campaign_id: Hash,
	// short description
	purpose: Vec<u8>,
	// ipfs content hash
	ipfs: Vec<u8>,
	// proposaled funds
	amount: Balance,
	// expiration block of proposal
	expiry: BlockNumber,
	// status
	// 0 open, 1 approved, 2 declined, 3 expired
	status: u8,
}

pub trait Config: timestamp::Config + campaigns::Config {
	type Event: From<Event<Self>> + Into<<Self as system::Config>::Event>;
}

const MAX_REQUESTS_PER_BLOCK: usize = 3;

//
//
//
//
//

decl_event!(
	pub enum Event<T>
	where
		<T as system::Config>::AccountId,
		<T as system::Config>::Hash,
		<T as balances::Config>::Balance,
		<T as system::Config>::BlockNumber
	{
		ProposalCreated(AccountId, Hash, Hash, Balance, BlockNumber),
		ProposalVoted(AccountId, Hash),
		ProposalFinalized(Hash, u64, BlockNumber, bool),
	}
);

//
//
//
//
//

decl_storage! {

	trait Store for Module<T: Trait> as ProposalProposal {

		// Global status
		Proposals get(fn proposals): map hasher(blake2_128_concat) T::Hash => Proposal<T::Hash, T::Balance, T::BlockNumber>;
		ProposalOwner get(fn owner_of_proposal): map hasher(blake2_128_concat) T::Hash => Option<T::AccountId>;

		// Maximum time limit for the proposal
		ProposalPeriodLimit get(fn proposal_period_limit) config(): T::BlockNumber = T::BlockNumber::sa(60480);

		// All proposals
		AllProposalArray get(fn proposal_by_index): map hasher(blake2_128_concat) u64 => T::Hash;
		AllProposalCount get(fn all_proposal_count): u64;
		AllProposalIndex: map hasher(blake2_128_concat) T::Hash => u64;

		// The funding's proposals
		ProposalOfFundingArray get(fn proposal_of_funding_by_index): map hasher(blake2_128_concat)  (T::Hash, u64) => T::Hash;
		ProposalOfFundingCount get(fn proposal_of_funding_count): map hasher(blake2_128_concat) T::Hash => u64;
		ProposalOfFundingIndex: map hasher(blake2_128_concat) (T::Hash, T::Hash) => u64;

		// The owner's proposals
		ProposalOfOwnerArray get(fn proposal_of_owner): map hasher(blake2_128_concat) (T::AccountId, u64) => T::Hash;
		ProposalOfOwnerCount get(fn proposal_of_owner_count): map hasher(blake2_128_concat) T::AccountId => u64;
		ProposalOfOwnerIndex: map hasher(blake2_128_concat) (T::AccountId, T::Hash) => u64;

		// Proposals ending in a block
		ProposalsByBlockNumber get(fn proposal_expire_at): map hasher(blake2_128_concat) T::BlockNumber => Vec<T::Hash>;

		// The amount of money that the project has used
		UsedMoneyOfCampaign get(fn used_money_of_campaign): map hasher(blake2_128_concat) T::Hash => T::Balance;

		// The number of people who support the proposal
		SupporterOfProposal get(fn supporter_of_proposal): map hasher(blake2_128_concat) T::Hash => u64;

		// Judge if the user has voted the proposal already
		VotedBefore get(fn voted_before): map hasher(blake2_128_concat) (T::AccountId, T::Hash) => bool;

		// Get the status of a proposal: 0 undecided 1 accepted 2 declined 3 expired
       	ProposalStatus get(fn status_of_proposal): map hasher(blake2_128_concat) T::Hash => u8;

		// Record the number of proposals
		Nonce: u64;
	}
}

//
//
//
//
//

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		// Initializing events
		fn deposit_event<T>() = default;

		fn create_proposal(origin, campaign_id: T::Hash, purpose: Vec<u8>, amount: T::Balance, expiry: T::BlockNumber) -> Result{
			let sender = ensure_signed(origin)?;

			// Ensure the funding exists
			ensure!(<funding_factory::Module<T>>::is_funding_exists(campaign_id), "The funding does not exist");
			// Ensure the funding is success
			ensure!(<funding_factory::Module<T>>::is_funding_success(campaign_id) == 1, "The funding does not succeed");
			// Ensure the sender is the owner
			let owner = <funding_factory::Module<T>>::get_funding_owner(campaign_id).ok_or("The owner does not exist")?;
			ensure!(sender == owner, "The sender must be the owner of the funding");

			// ensure that the expiry is valid
			ensure!(expiry > <system::Module<T>>::block_number(), "The expiry has to be greater than the current block number");
			ensure!(expiry <= <system::Module<T>>::block_number() + Self::proposal_period_limit(), "The expiry has be lower than the limit block number");

			let used_balance = Self::used_money_of_campaign(&campaign_id);
			let total_balance = <funding_factory::Module<T>>::get_funding_total_balance(campaign_id);
			let remain_balance = total_balance - used_balance;
			ensure!(remain_balance >= amount, "The remain money is not enough");
			// get the nonce to help generate unique id
			let nonce = <Nonce<T>>::get();

			// generate the unique id
			let proposal_id = (<system::Module<T>>::random_seed(), &amount, &sender, nonce)
				.using_encoded(<T as system::Trait>::Hashing::hash);
			// ensure that the proposal id is unique
			ensure!(!<Proposals<T>>::exists(&campaign_id), "Proposal already exists");

			let new_proposal = Proposal{
				proposal_id,
				campaign_id: campaign_id.clone(),
				purpose,
				amount,
				expiry,
				status: 0,
			};

			// ensure that the number of proposals in the block does not exceed maximum
			let proposals = Self::proposal_expire_at(expiry);
			ensure!(proposals.len() < MAX_REQUESTS_PER_BLOCK, "Maximum number of proposals is reached for the target block, try another block");

			// Verify adding count is ok first
			// Check adding all proposal count
			let all_proposal_count = Self::all_proposal_count();
			let new_all_proposal_count = all_proposal_count.checked_add(1).ok_or("Overflow adding a new proposal to total proposals")?;

			// Check adding proposals of funding count
			let proposal_of_funding_count = Self::proposal_of_funding_count(campaign_id);
			let new_proposal_of_funding_count = proposal_of_funding_count.checked_add(1).ok_or("Overflow adding a new proposal to the funding's proposals")?;

			// Check adding proposals of owner count
			let proposal_of_owner_count = Self::proposal_of_owner_count(&sender);
			let new_proposal_of_owner_count = proposal_of_owner_count.checked_add(1).ok_or("Overflow adding a new proposal to the owner's proposals")?;

			// change the global states
			<Proposals<T>>::insert(proposal_id.clone(), new_proposal.clone());
			<ProposalOwner<T>>::insert(proposal_id.clone(), sender.clone());

			<ProposalsByBlockNumber<T>>::mutate(expiry, |proposals| proposals.push(proposal_id.clone()));

			// change the state of all proposals
			<AllProposalArray<T>>::insert(&all_proposal_count, proposal_id.clone());
			<AllProposalCount<T>>::put(new_all_proposal_count);
			<AllProposalIndex<T>>::insert(proposal_id.clone(), all_proposal_count);

			// change the state of funding's proposals
			<ProposalOfFundingArray<T>>::insert((campaign_id.clone(), proposal_of_funding_count.clone()), proposal_id.clone());
			<ProposalOfFundingCount<T>>::insert(campaign_id.clone(), new_proposal_of_funding_count);
			<ProposalOfFundingIndex<T>>::insert((campaign_id.clone(), proposal_id.clone()), proposal_of_funding_count);

			// change the state of owner's proposals
			<ProposalOfOwnerArray<T>>::insert((sender.clone(), proposal_of_owner_count.clone()), proposal_id.clone());
			<ProposalOfOwnerCount<T>>::insert(sender.clone(), new_proposal_of_owner_count);
			<ProposalOfOwnerIndex<T>>::insert((sender.clone(), proposal_id.clone()), proposal_of_owner_count);

			// add the nonce
			<Nonce<T>>::mutate(|n| *n += 1);

			// deposit the event
			Self::deposit_event(RawEvent::CreateProposal(sender, campaign_id, proposal_id, amount, expiry));
			Ok(())
		}

		fn support_proposal(origin, proposal_id: T::Hash) -> Result{
			let sender = ensure_signed(origin)?;
			// Ensure the proposal exists
			ensure!(<Proposals<T>>::exists(&proposal_id), "The proposal does not exist");
			// Get the proposal
			let mut proposal = Self::proposals(&proposal_id);
			// Ensure the user is investor
			ensure!(<funding_factory::Module<T>>::is_investor(proposal.campaign_id, sender.clone()), "You are not the investor");
			// Ensure the investor does not vote before
			ensure!(!<VotedBefore<T>>::get((sender.clone(), proposal_id.clone())), "You have voted before");
			// Ensure the proposal is not over
			ensure!(proposal.status == 0, "The proposal is over");
			// Ensure the proposal is not expire
			ensure!(<system::Module<T>>::block_number() < proposal.expiry, "This proposal is expired.");
			// Get the number of people who have supported the proposal and add 1
			let supported_proposal_count = Self::supporter_of_proposal(&proposal_id);
			let new_supported_proposal_count = supported_proposal_count.checked_add(1).ok_or("Overflow adding the number of people who have voted the proposal")?;
			// Check if the number is bigger than half
			let invested_number = <funding_factory::Module<T>>::get_invested_number(proposal.campaign_id);
			let half_number = invested_number.checked_div(2).ok_or("Error when get half of the invested number")?;
			let supported_count = new_supported_proposal_count.clone();
			// If the supported_count is bigger than the half, the proposal is success
			if new_supported_proposal_count > half_number{
				Self::can_use_balance(proposal_id, supported_count)?;
			}
			// Change the investor voting status
			<VotedBefore<T>>::insert((sender.clone(), proposal_id.clone()), true);
			// Change the number of supporters
			<SupportedOfProposal<T>>::insert(proposal_id.clone(), new_supported_proposal_count.clone());
			// Deposit the Vote event
			Self::deposit_event(RawEvent::Vote(sender, proposal_id.clone()));
			Ok(())
		}

		fn on_finalize() {
			// get all the fundings of the block
			let block_number = <system::Module<T>>::block_number();
			let proposal_hashs = Self::proposal_expire_at(block_number);

			for proposal_id in &proposal_hashs{
				// Get the proposal
				let mut proposal = Self::proposals(proposal_id);
				// Check if the proposal is success before
				if proposal.status == 1{
					continue;
				}
				// Else the proposal fails
				proposal.status = 2;
				<Proposals<T>>::insert(proposal_id.clone(), proposal.clone());
				let supported_count = <SupportedOfProposal<T>>::get(proposal.proposal_id);
				Self::deposit_event(RawEvent::ProposalFinalized(proposal.proposal_id, supported_count, proposal.expiry, false));
			}
		}
	}
}

//
//
//
//
//

impl<T:Trait> Module<T>{
	fn can_use_balance(proposal_id: T::Hash, supported_count: u64) -> Result{
		// Get the proposal
		let mut proposal = Self::proposals(&proposal_id);
		let proposal_balance = proposal.amount;
		// Ensure that there is enough money
		let used_balance = <UsedMoneyOfCampaign<T>>::get(proposal.campaign_id);
		let total_balance = <funding_factory::Module<T>>::get_funding_total_balance(proposal.campaign_id);
		let remain_balance = total_balance - used_balance.clone();
		ensure!(remain_balance >= proposal_balance, "The remain balance is not enough");
		// Get the owner of the funding
		let owner = <ProposalOwner<T>>::get(&proposal_id).ok_or("No owner of the proposal")?;
		// Unreserve the proposal balance
		let _ = <balances::Module<T>>::unreserve(&owner, proposal_balance.clone());
		// Change the used amount
		let new_used_balance = used_balance + proposal_balance;
		<UsedMoneyOfCampaign<T>>::insert(proposal.campaign_id, new_used_balance);
		// Change the proposal status
		proposal.status = 1;
		<Proposals<T>>::insert(proposal_id.clone(), proposal.clone());
		Self::deposit_event(RawEvent::ProposalFinalized(proposal_id, supported_count, proposal.expiry, true));
		Ok(())
	}
}

