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
// Copyright (C) 2010-2020 ZERO Technologies.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! # Crowdfunding Campaign Factory + Treasury
//!
//! Run `cargo doc --package module-crowdfunding --open` to view this pallet's documentation.
//!
//! ## Overview
//!
//! This pallet provides a simple on-chain crowdfunding mechanism:
//!
//! - creator can create a campaign with individual length and
//! amount of funds in PLAY to raise
//!
//! - investor can invest his funds into one of the running campaigns
//! and become an investor
//!
//! Upon finalization:
//!
//! - creator can request allocation of funds
//! - investors can collectively approve allocation of funds
//!
//! TODO:
//! - supervisor can lock, cancel campaigns
//!

// 1. create campaigns with custom funding goal and runtime
// 2. invest into open campaigns

#![cfg_attr(not(feature = "std"), no_std)]

// TODO: harden checks on completion
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
// only on nightly
// #![feature(const_fn_fn_ptr_basics)]

use frame_support::{
	decl_error, decl_event, decl_module, decl_storage,
	ensure,
	dispatch::DispatchResult,
	traits::{
		EnsureOrigin,
		Randomness,
		Currency,
		ExistenceRequirement,
		ReservableCurrency,
		Get,
		Time,
		UnixTime,
	},
	storage::child::exists
};
use frame_system::{ self as system, ensure_signed };
use sp_core::{ Hasher, H256 };
use sp_runtime::{
	traits::{ Hash, TrailingZeroInput },
	ModuleId,
};
use sp_std::prelude::*;
use codec::{ Encode, Decode };

use timestamp;
use primitives::{ Balance };

// TODO: tests
// #[cfg(test)]
// mod campaign_tests;

// TODO: pallet benchmarking
// mod benchmarking;

// TODO: weights
// mod default_weights;

// TODO: externalise error messages
// mod errors;

// #[cfg(feature = "with-campaign-governance")]
// mod treasury;

// module header
const PALLET_ID: ModuleId = ModuleId(*b"GAMEcrwd");
const PALLET_VERSION: &str = "1.0";

// TODO: take from runtime limit simultanous campaign initiation
const MAX_CONTRIBUTIONS_PER_BLOCK: usize = 5;

// TODO: take from runtime
const MAX_CONTRIBUTIONS_PER_ADDRESS: usize = 3;

// TODO: take from runtime max campaign duration
const MAX_CAMPAIGN_LENGTH: u32 = 777600;

pub trait Config: system::Config + balances::Config + timestamp::Config {

	/// The module id
	type ModuleId: Get<ModuleId>;

	// type AdminOrigin: Get<Self::AccountId>;

	type Currency: ReservableCurrency<Self::AccountId>;

	type Event: From<Event<Self>> + Into<<Self as system::Config>::Event>;
	type Nonce: Get<u64>;
	type Randomness: Randomness<Self::Hash>;
	type MinLength: Get<usize>;
	type MaxLength: Get<usize>;
	type MaxCampaignsPerAddress: Get<usize>;
	type MaxCampaignsPerBlock: Get<usize>;
	type MinDuration: Get<Self::BlockNumber>;
	type MaxDuration: Get<Self::BlockNumber>;
	type MinCreatorDeposit: Get<Self::Balance>;
	type MinContribution: Get<Self::Balance>;
	// TODO: collect fees for treasury
	// type CreationFee: Get<T::Balance<Self>>;
	// type ContributionFee: Get<T::Balance<Self>>;

	/// The origin that is allowed to make judgements.
	type GameDAOAdminOrigin: EnsureOrigin<Self::Origin>;

}

// TODO: this can be decomposed to improve weight
#[derive(Encode, Decode, Default, Clone, PartialEq, Eq)]
pub struct Campaign<Hash, AccountId, Balance, BlockNumber, Timestamp> {

	/// unique hash to identify campaign (generated)
	id: Hash,

	/// owner account of the campaign (beneficiary)
	owner: AccountId,

	/// admin account of the campaign (operator)
	admin: AccountId,

	/// campaign owners deposit
	deposit: Balance,

	/// blocknumber until campaign has to reach cap
	expiry: BlockNumber,

	/// minimum amount of token to become a successful campaign
	cap: Balance,

	/// name
	name: Vec<u8>,

	/// protocol: 0 grant, 1 prepaid, 2 loan, 3 share, 4 dao
	protocol: u8,

	/// dao governed after success
	/// true: payout through governance
	/// false: 100% payout upon completion
	governance: u8,

	// /// storage
	// ipfs_hash: Vec<u8>,

	// /// token symbol
	// token_symbol: Vec<u8>,

	// /// token name
	// token_name: Vec<u8>,

	created: Timestamp, //Vec<u8>,

}

decl_storage! {
	trait Store for Module<T: Config> as CrowdfundingFactory {

		// TODO:
		//	actually most of the aggregated data only consumes cpu cycles
		//	and should not be stored on chain, but on ipfs.
		//
		// -	statistics:
		// 		total campaigns
		// 		total (indiviual?) contributors sum
		// 		total contributions sum
		//
		// -	all / campaigns meta info
		//
		// -	allCampaignsArray { contributor hash, contribution amount } =>
		// -	allCampaignsCount
		//
		// - 	ownedCampaigns

		/// Get one or all Campaigns
		Campaigns get(fn campaign_by_id): map hasher(blake2_128_concat) T::Hash => Campaign<T::Hash, T::AccountId, T::Balance, T::BlockNumber, T::Moment>;

		/// Get Campaign owner by campaign id
		CampaignOwner get(fn owner_of): map hasher(blake2_128_concat) T::Hash => Option<T::AccountId>;

		/// Get Campaign Admin by campaign id
		CampaignAdmin get(fn admin_of): map hasher(blake2_128_concat) T::Hash => Option<T::AccountId>;

		/// Campaign state
		/// 0 init, 1 active, 2 paused, 3 complete success, 4 complete failed, 5 authority lock
		CampaignState get(fn state_of): map hasher(blake2_128_concat) T::Hash => u8;

		/// Campaigns ending in block x
		CampaignsByBlockNumber get(fn campaign_expire_at): map hasher(blake2_128_concat) T::BlockNumber => Vec<T::Hash>;

		// total number of campaigns
		AllCampaignsCount get(fn all_campaigns_count): u64;
		AllCampaignsArray get(fn campaigns_by_index): map hasher(blake2_128_concat) u64 => T::Hash;
		AllCampaignsIndex: map hasher(blake2_128_concat) T::Hash => u64;

		// caller owned campaigns
		OwnedCampaignsArray get(fn owned_campaigns_by_index): map hasher(blake2_128_concat) (T::AccountId, u64) => T::Hash;
		OwnedCampaignsCount get(fn owned_campaigns_count): map hasher(blake2_128_concat) T::AccountId => u64;
		OwnedCampaignsIndex: map hasher(blake2_128_concat) (T::AccountId, T::Hash) => u64;

		// caller contributed campaigns
		ContributedCampaignsArray get(fn contributed_campaigns_by_index): map hasher(blake2_128_concat) (T::AccountId, u64) => T::Hash;
		ContributedCampaignsCount get(fn contributed_campaigns_count): map hasher(blake2_128_concat) T::AccountId => u64;
		ContributedCampaignsIndex: map hasher(blake2_128_concat) (T::AccountId, T::Hash) => u64;

		// Total contributions per campaign
		TotalContributions get(fn total_contributions): map hasher(blake2_128_concat) T::Hash => T::Balance;

		// Contributions per user
		ContributedAmount get(fn contributed_amount): map hasher(blake2_128_concat) (T::Hash, T::AccountId) => T::Balance;

		// Contributor Accounts
		ContributorAccounts get(fn contributor_accounts): map hasher(blake2_128_concat) T::Hash => Vec<T::AccountId>;
		ContributorAccountsCount get(fn contributor_accounts_count): map hasher(blake2_128_concat) T::Hash => u64;

		/// Max campaign block limit
		CampaignMaxDuration get(fn get_max_duration) config(): T::BlockNumber = T::BlockNumber::from(MAX_CAMPAIGN_LENGTH);

		// Campaign nonce, increases per created campaign
		Nonce: u64;
	}
}

decl_event! {
	pub enum Event<T> where
		<T as system::Config>::Hash,
		<T as system::Config>::AccountId,
		<T as balances::Config>::Balance,
		<T as system::Config>::BlockNumber,
		EventMessage = Vec<u8>,
	{
		CampaignCreated(Hash, AccountId, AccountId, Balance, Balance, BlockNumber, Vec<u8>),
		CampaignContributed(Hash, AccountId, Balance, BlockNumber),
		CampaignFinalized(Hash, Balance, BlockNumber, bool),
		CampaignFailed(Hash, Balance, BlockNumber, bool),
		CampaignUpdated(Hash, u8, BlockNumber),
		Message(EventMessage),
	}
}

decl_module! {
	pub struct Module<T: Config> for enum Call where origin: T::Origin {

		type Error = Error<T>;

		fn deposit_event() = default;

		// update the campaign status
		// 0 init, 1 active, 2 paused, 3 complete success, 4 complete failed, 5 authority lock
	 	// admin can set any status
	 	// owner can pause, cancel
		#[weight = 1_000]
		fn update_status(
			origin,
			campaign_id: T::Hash,
			status: u8
		) -> DispatchResult {

			// access control
			let sender = ensure_signed(origin)?;

			let owner = Self::owner_of(campaign_id).ok_or(Error::<T>::OwnerUnknown)?;
			let admin = Self::admin_of(campaign_id).ok_or(Error::<T>::AdminUnknown)?;
			ensure!( sender == admin, Error::<T>::AuthorizationError );

			// expired?
			let campaign = Self::campaign_by_id(&campaign_id);
			ensure!(<system::Module<T>>::block_number() < campaign.expiry, Error::<T>::CampaignExpired );

			// not finished or locked?
			let current_status = Self::state_of(campaign_id);
			ensure!(current_status < 2, Error::<T>::CampaignExpired );

			// set
			Self::set_status(campaign_id.clone(), status.clone());

			// dispatch status update event
			let now = <system::Module<T>>::block_number();
			Self::deposit_event(
				RawEvent::CampaignUpdated(
					campaign_id,
					status,
					now
				)
			);

			Ok(())
		}

		#[weight = 10_000]
		fn create(
			origin,
			admin: T::AccountId,
			name: Vec<u8>,
			target: T::Balance,
			deposit: T::Balance,
			// TODO: should be duration in days,
			// not target blocknumber
			expiry: T::BlockNumber,
			protocol: u8,
			governance: u8
		) {

			// get the creator
			let creator = ensure_signed(origin)?;

			// check name length boundary
			ensure!(
				name.len() >= T::MinLength::get(),
				Error::<T>::NameTooShort
			);
			ensure!(
				name.len() <= T::MaxLength::get(),
				Error::<T>::NameTooLong
			);

			// get the nonce to help generate unique id
			let nonce = T::Nonce::get();

			// blocktime
			let now = <system::Module<T>>::block_number();

			// timestamp
			let timestamp = <timestamp::Module<T>>::get();

			// ensure campaign expires after now
			ensure!(
				expiry > now,
				Error::<T>::EndTooEarly
			);


			// TODO: refactor calculate dest. block
			// let blocktime = 5;
			// let target_block_number =

			// TODO: fix BlockNumber sa
			// ensure!(
			// 	expiry <= <system::Module<T>>::block_number() + Self::campaign_max_duration_limit(),
			// 	Error::<T>::EndTooLate
			// );

			// generate the unique campaign id
			let phrase = b"crowdfunding_campaign";
			let id = T::Randomness::random(phrase);

			// TODO: check for correct padding
			// let seed = <[u8; 32]>::decode(&mut TrailingZeroInput::new(seed.as_ref()))
			// 	.expect("input is padded with zeroes; qed");
			// let id = seed.clone();

			// ensure unique id
			// ensure!(
			// 	!<CampaignOwner<T>>::exists(&id),
			// 	Error::<T>::CampaignIdExists
			// );

			// ensure deposit <= target
			// ensure!(
			// 	deposit <= target,
			// 	Error::<T>::CampaignDepositTooHigh
			// );

			// check contribution limit per block
			// let contributions = Self::campaign_expire_at(expiry);
			// ensure!(
			// 	contributions.len() < MAX_CONTRIBUTIONS_PER_BLOCK,
			// 	Error::<T>::ContributionsPerBlockExceeded
			// );

			// let protocol :u8 = 0;
			// let governance: bool = false;

			// create a new campaign
			// id: Hash,
			// name: Vec<u8>,
			// owner: AccountId,
			// admin: AccountId,
			// deposit: Balance,
			// expiry: BlockNumber,
			// cap: Balance,
			// protocol: u8
			// governance: bool
			// status: u8,
			let new_campaign = Campaign {
				id: id.clone(),
				name: name.clone(),
				owner: creator.clone(),
				admin: admin.clone(),
				deposit: deposit.clone(),
				expiry: expiry,
				cap: target,
				protocol: protocol.clone(),
				governance: governance.clone(),
				created: timestamp
			};

			// mint the campaign
			Self::mint(
				new_campaign
			)?;

			// 0 init, 1 active, 2 paused, 3 complete success, 4 complete failed, 5 authority lock
			Self::set_status(
				id.clone(),
				1
			);

			// deposit the event
			Self::deposit_event(
				RawEvent::CampaignCreated(
					id,
					creator,
					admin,
					target,
					deposit,
					expiry,
					name
				)
			);

			// No fees are paid here if we need to create this account;
			// that's why we don't just use the stock `transfer`.
			// T::Currency::resolve_creating(&Self::campaign_account_id(index), imb);

		}

		/// contribute to project
		#[weight = 10_000]
		fn contribute (
			origin,
			campaign_id: T::Hash,
			contribution: T::Balance
		) -> DispatchResult {

			let sender = ensure_signed(origin)?;

			// owners cannot contribute to their own campaign..
			let owner = Self::owner_of(campaign_id) .ok_or(Error::<T>::OwnerUnknown)?;
			ensure!( owner != sender, Error::<T>::NoContributionToOwnCampaign );

			// 0 init, 1 active, 2 paused, 3 complete success, 4 complete failed, 5 authority lock
			// contribution only possible when state active..
			let state = Self::state_of(campaign_id);
			ensure!( state == 1, Error::<T>::NoContributionsAllowed);

			// submit
			Self::create_contribution(sender.clone(), campaign_id.clone(), contribution.clone())?;

			// get current blocktime
			let now = <system::Module<T>>::block_number();

			// send event
			Self::deposit_event(
				RawEvent::CampaignContributed(
					campaign_id,
					sender,
					contribution,
					now,
				)
			);

			Ok(())
		}


		// depending on the protocol, a successful campaign can resolve into the following:
		//
		// 1. campaign not successful: funds are unreserved in the contributor accounts.
		//
		// 2. campaign successful:
		//
		//    a. funds are collected and sent to creator ( Grant without DAO )
		//
		//    b. funds are collected and moved to a treasury account, where they are reserved again.
		//       - in governance creator can create motions to withdraw funds.

		/// finalize campaigns ending in current block
		fn on_finalize() {

			// get all the contributions of current block
			let block_number = <system::Module<T>>::block_number();
			let campaign_hashes = Self::campaign_expire_at(block_number);

			// iterate over hashes
			for campaign_id in &campaign_hashes {

				let campaign = Self::campaign_by_id(campaign_id);
				let total_contributions = Self::total_contributions(campaign_id);

				// check for cap reached
				if total_contributions >= campaign.cap {

					// update campaign state to success
					// campaign.status = 3;
					Self::set_status(campaign.id.clone(),3);
					<Campaigns<T>>::insert(campaign_id.clone(), campaign);

					// get campaign owner
					let _campaign_owner = Self::owner_of(campaign_id);

					match _campaign_owner {
						Some(campaign_owner) => {

							// get all contributors
							let contributors = Self::contributor_accounts(campaign_id);
							let mut transaction_complete = true;

							// 1 iterate over contributors
							// 2 unreserve contribution
							// 3 transfer contribution to campaign owner -> should be treasury!
							'inner: for contributor in &contributors {

								// get contributed balance
								let contributor_balance = Self::contributed_amount((*campaign_id, contributor.clone()));
								// unreserve balance
								let _ = <balances::Module<T>>::unreserve(&contributor, contributor_balance.clone());
								// if contributor == campaign owner
								if contributor == &campaign_owner { continue; }

								// transfer balance from contributor to owner..
								let _transfer = <balances::Module<T> as Currency<_>>::transfer(
									&contributor,
									&campaign_owner,
									contributor_balance,
									ExistenceRequirement::AllowDeath
								);

								match _transfer {
									Err(_e) => {
										transaction_complete = false;
										break 'inner;
									},
									Ok(_v) => {}
								}

							}

							// If all transactions are settled
							// reserve all money of the funding
							if transaction_complete {
								let _ = <balances::Module<T>>::reserve(&campaign_owner, total_contributions);
								// deposit the event
								Self::deposit_event(RawEvent::CampaignFinalized(*campaign_id, total_contributions, block_number, true));
							}

						},
						None => continue,
					}

				// campaign cap not reached
				} else {

					// campaign failed
					// refund all of the money

					// update camapign state to failed
					// campaign.status = 4;
					Self::set_status(campaign.id,4);
					<Campaigns<T>>::insert(campaign_id.clone(), campaign);

					// revert all contributions
					let contributors = Self::contributor_accounts(campaign_id);
					for account in contributors {
						let contribution = Self::contributed_amount((*campaign_id, account.clone()));
						let _ = <balances::Module<T>>::unreserve(&account, contribution);
					}

					// deposit the event
					Self::deposit_event(RawEvent::CampaignFailed(*campaign_id, total_contributions, block_number, false));

				}
			}

		}

 	}
}

impl<T: Config> Module<T> {

	fn set_status( campaign_id: T::Hash, status: u8 ) -> DispatchResult {
		<CampaignState<T>>::insert(&campaign_id, status );
		Ok(())
	}

	fn mint(
		// campaign creator
		// sender: T::AccountId,
		// generated campaign id
		// campaign_id: T::Hash,
		// expiration blocktime
		// example: desired lifetime == 30 days
		// 30 days * 24h * 60m / 5s avg blocktime ==
		// 2592000s / 5s == 518400 blocks from now.
		// expiry: T::BlockNumber,
		// campaign creator deposit to invoke the campaign
		// deposit: T::Balance,
		// funding protocol
		// 0 grant, 1 prepaid, 2 loan, 3 shares, 4 dao
		// proper assignment of funds into the instrument
		// happens after successful funding of the campaing
		// protocol: u8,
		// campaign object
		campaign: Campaign<T::Hash, T::AccountId, T::Balance, T::BlockNumber, T::Moment>
	) -> DispatchResult {

		// campaigns
		<Campaigns<T>>::insert(campaign.id.clone(), campaign.clone());

		// owners
		<CampaignOwner<T>>::insert(campaign.id.clone(), campaign.owner.clone());

		// admins
		// let admin = new_campaign.admin;
		<CampaignAdmin<T>>::insert(campaign.id.clone(), campaign.admin.clone());

		// expiration
		//<CampaignsByBlockNumber<T>>::mutate(expiry.clone(), |campaigns| campaigns.push(id.clone()));
		<CampaignsByBlockNumber<T>>::mutate(
			campaign.expiry.clone(),
			|campaigns| campaigns.push(campaign.id.clone())
		);

		// collect by state

		// global campaigns count
		let all_campaigns_count = Self::all_campaigns_count();
		let update_all_campaigns_count = all_campaigns_count.checked_add(1).ok_or(Error::<T>::AddCampaignOverflow)?;

		// owned campaigns count
		let owned_campaigns_count = Self::owned_campaigns_count(&campaign.owner);
		let update_owned_campaigns_count = owned_campaigns_count.checked_add(1).ok_or(Error::<T>::AddContributionOverflow)?;

		// update global campaign count
		<AllCampaignsArray<T>>::insert(&all_campaigns_count, campaign.id.clone());
		<AllCampaignsCount>::put(update_all_campaigns_count);
		<AllCampaignsIndex<T>>::insert(campaign.id.clone(), all_campaigns_count);

		// update owned campaign count
		<OwnedCampaignsArray<T>>::insert((campaign.owner.clone(), owned_campaigns_count.clone()), campaign.id.clone());
		<OwnedCampaignsCount<T>>::insert(&campaign.owner, update_owned_campaigns_count);
		<OwnedCampaignsIndex<T>>::insert((campaign.owner.clone(), campaign.id.clone()), owned_campaigns_count);

		// write

		// TODO: final check
		// TODO: final check
		// if deposit exceeds available balance,
		// revert the campaign

		// if deposit > Balance::sa(0) {
		// 	match Self::create_contribution(sender.clone(), id.clone(), deposit.clone()){
		// 		Err(_e) => {
		// 			<Campaigns<T>>::remove(id.clone());
		// 			<CampaignOwner<T>>::remove(id.clone());
		// 			<CampaignsByBlockNumber<T>>::mutate(expiry,|campaigns| campaigns.pop());
		// 			<AllCampaignsArray<T>>::remove(&all_campaigns_count);
		// 			<AllCampaignsCount>::put(all_campaigns_count.clone());
		// 			<AllCampaignsIndex<T>>::remove(id.clone());
		// 			<OwnedCampaignsArray<T>>::remove((sender.clone(), owned_campaigns_count.clone()));
		// 			<OwnedCampaignsCount<T>>::remove(&sender);
		// 			<OwnedCampaignsIndex<T>>::remove((sender.clone(), id.clone()));
		// 		},
		// 		Ok(_v) => {}
		// 	}
		// }

		// nonce ++
		Nonce::mutate(|n| *n += 1);

		Ok(())
	}

	fn create_contribution(
		sender: T::AccountId,
		campaign_id: T::Hash,
		contribution: T::Balance
	) -> DispatchResult {

		// campaign exists ?
		ensure!( <Campaigns<T>>::contains_key(campaign_id), Error::<T>::InvalidId );
		let campaign = Self::campaign_by_id(&campaign_id);

		// campaign still active ?
		ensure!(<system::Module<T>>::block_number() < campaign.expiry, Error::<T>::CampaignExpired );

		// contributor has sufficient balance ?
		ensure!( <balances::Module<T>>::free_balance(sender.clone()) >= contribution, Error::<T>::BalanceTooLow );

		// meta data
		// check if contributor exists
		// if not, update metadata
		if !<ContributedAmount<T>>::contains_key((&campaign_id, &sender)) {

			// increase the number of campaigncontributors invested in
			let contributed_campaigns = Self::contributed_campaigns_count(&sender);
			let update_contributed_campaigns = contributed_campaigns.checked_add(1).ok_or(Error::<T>::AddContributionOverflow)?;

			// increase the number of contributors into the campaign
			let contributors = <ContributorAccountsCount<T>>::get(&campaign_id);
			let update_contributors = contributors.checked_add(1).ok_or(Error::<T>::UpdateContributorOverflow)?;

			// change the state of invest related fields
			// <ContributedAmount<T>>::insert((campaign_key.clone(), sender.clone()), contribution.clone());
			<ContributorAccounts<T>>::mutate(&campaign_id, |accounts| accounts.push(sender.clone()));

			// update total contributor count
			<ContributorAccountsCount<T>>::insert(campaign_id.clone(), update_contributors);

			// update contributed campaigns
			<ContributedCampaignsArray<T>>::insert((sender.clone(), contributed_campaigns), campaign_id);
			<ContributedCampaignsCount<T>>::insert(&sender, update_contributed_campaigns);
			<ContributedCampaignsIndex<T>>::insert((sender.clone(), campaign_id.clone()), contributed_campaigns);

		}

		// reserve
		<balances::Module<T>>::reserve(&sender, contribution)?;

		// update contributions to campaign per contributor
		let total_contributions = Self::contributed_amount((&campaign_id, &sender));
		let update_total_contributions = total_contributions + contribution;
		<ContributedAmount<T>>::insert((&campaign_id, &sender), update_total_contributions);

		// update total contributions to campaign
		let total_campaign_contributions = Self::total_contributions(&campaign_id);
		let update_campaign_contributions = total_campaign_contributions + contribution;
		<TotalContributions<T>>::insert(&campaign_id, update_campaign_contributions);

		Ok(())
	}

}

//
//
//
//
//

// TODO
// Simple ensure origin struct to filter for the founder account.
// pub struct EnsureFounder<T>(sp_std::marker::PhantomData<T>);
// impl<T: Config> EnsureOrigin<T::Origin> for EnsureFounder<T> {
// 	type Success = T::AccountId;
// 	fn try_origin(o: T::Origin) -> Result<Self::Success, T::Origin> {
// 		o.into().and_then(|o| match (o, Founder::<T>::get()) {
// 			(system::RawOrigin::Signed(ref who), Some(ref f)) if who == f => Ok(who.clone()),
// 			(r, _) => Err(T::Origin::from(r)),
// 		})
// 	}

// 	#[cfg(feature = "runtime-benchmarks")]
// 	fn successful_origin() -> T::Origin {
// 		let founder = Founder::<T>::get().expect("society founder should exist");
// 		T::Origin::from(system::RawOrigin::Signed(founder))
// 	}
// }

//
//
//
//
//

decl_error! {
	pub enum Error for Module<T: Config> {

		//
		//	general
		//
		/// Must contribute at least the minimum amount of Campaigns
		ContributionTooSmall,
		/// Balance too low.
		BalanceTooLow,
		/// The Campaign id specified does not exist
		InvalidId,
		/// The Campaign's contribution period has ended; no more contributions will be accepted
		ContributionPeriodOver,
		/// You may not withdraw or dispense Campaigns while the Campaign is still active
		CampaignStillActive,
		/// You cannot withdraw Campaigns because you have not contributed any
		NoContribution,
		/// You cannot dissolve a Campaign that has not yet completed its retirement period
		CampaignNotRetired,
		/// Campaign expired
		CampaignExpired,
		/// Cannot dispense Campaigns from an unsuccessful Campaign
		UnsuccessfulCampaign,

		//
		//	create
		//
		/// Campaign must end after it starts
		EndTooEarly,
		/// Campaign expiry has be lower than the block number limit
		EndTooLate,
		/// Max contributions per block exceeded
		ContributionsPerBlockExceeded,
		/// Name too long
		NameTooLong,
		/// Name too short
		NameTooShort,
		/// Deposit exceeds the campaign target
		DepositTooHigh,
		/// Campaign id exists
		IdExists,

		//
		//	mint
		//
		/// Overflow adding a new campaign to total fundings
		AddCampaignOverflow,
		/// Overflow adding a new contribution to account balance
		AddContributionOverflow,
		/// Overflow adding to the total number of contributors of a camapaign
		UpdateContributorOverflow,
		/// Campaign owner unknown
		OwnerUnknown,
		/// Campaign admin unknown
		AdminUnknown,
		/// Cannot contribute to owned campaign
		NoContributionToOwnCampaign,
		/// Guru Meditation
		GuruMeditation,
		/// Zou are not authorized for this call
		AuthorizationError,
		/// Contributions not allowed
		NoContributionsAllowed,

	}
}

