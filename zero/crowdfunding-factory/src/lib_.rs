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

//! # Crowdfunding Factory
//!
//! Run `cargo doc --package crowdfunding-factory --open` to view this pallet's documentation.
//!
//! ## Overview
//!
//! This pallet provides a simple on-chain crowdfunding mechanism
//! using the networks native currency PLAY.

#![allow(dead_code)]
#![allow(unused_imports)]
#![cfg_attr(not(feature = "std"), no_std)]

use sp_runtime::{
	traits::{ Hash, SignedExtension, Bounded, SaturatedConversion, DispatchInfoOf },
	transaction_validity::{ ValidTransaction, TransactionValidityError, InvalidTransaction, TransactionValidity },
	ModuleId,
};
use sp_core::Hasher;
use sp_std::prelude::*;

use frame_support::{
	storage::child,
	decl_module, decl_storage, decl_event, decl_error, ensure,
	dispatch::DispatchResult,
	traits::{ Randomness, Currency, ReservableCurrency, Get},
	weights::Weight,
};

use frame_system::{self as system, ensure_signed};

use codec::{Encode, Decode};
use serde::*;

// #[cfg(test)]
// mod tests;
// mod benchmarking;
// mod default_weights;

const PALLET_ID: ModuleId = ModuleId(*b"zerocrwd");
const PALLET_VERSION: &str = "1.0";
const MAX_CONTRIBUTIONS_PER_BLOCK: usize = 5;
const MAX_CAMPAIGN_LENGTH: u64 = 777600;

//
//	configuration trait
//

pub trait Trait: system::Trait + balances::Trait {
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
	// type Currency: ReservableCurrency<Self::AccountId>;
	// type CreationFee: Get<BalanceOf<Self>>;
	// type ContributionFee: Get<BalanceOf<Self>>;
	// type MinLength: Get<usize>;
	// type MaxLength: Get<usize>;
	// type Nonce: Get<u64>;
}

//
//	primitives
//

type AccountIdOf<T> = <T as system::Trait>::AccountId;
type BalanceOf<T> = <<T as Trait>::Currency as Currency<AccountIdOf<T>>>::Balance;
// type HashOf<T> = <T as sp_runtime>::Hash;
// type CampaignOf<T> = Campaign<T::Hash, AccountIdOf<T>, BalanceOf<T>, <T as system::Trait>::BlockNumber>;

#[derive(Encode, Decode, Default, Clone, PartialEq, Eq)]
// #[cfg_attr(feature = "std", derive(Debug))]
pub struct Campaign<Hash, AccountId, Balance, BlockNumber> {
	/// unique hash to identify campaign
	id: Hash,
	// project name
	name: Vec<u8>,
	/// controller account of the campaign
	manager: AccountId,
	/// campaign owners deposit
	deposit: Balance,
	/// blocknumber until campaign has to reach cap
	expiry: BlockNumber,
	/// minimum amount of token to become a successful campaign
	cap: Balance,
	/// status: 0 not started, 1 in progress, 2 success, 3 fail
	status: u8,
}

//
//	pallet storage
//

decl_storage! {
	trait Store for Module<T: Trait> as CrowdfundingFactory {

		/// Campaigns
		Campaigns get(fn campaign_by_id):
			map hasher(blake2_128_concat) T::Hash => Campaign<T::Hash, AccountIdOf<T>, BalanceOf<T>, T::BlockNumber>;

		/// Campaign owner by campaign id
		CampaignOwner get(fn owner_of): map hasher(blake2_128_concat) T::Hash => Option<AccountIdOf<T>>;

		// /// Max campaign time limit
		// CampaignMaxDurationLimit get(fn campaign_max_duration_limit) config(): T::BlockNumber = T::BlockNumber::sa(MAX_CAMPAIGN_LENGTH);

		/// Campaigns ending in a block
		CampaignsByBlockNumber get(fn campaign_expire_at): map hasher(blake2_128_concat) T::BlockNumber => Vec<T::Hash>;

		/// Campaign state
		CampaignState get(fn campaign_state): map hasher(blake2_128_concat) u8 => T::Hash;

		// // All contributions
		AllContributionsArray get(fn contributions_by_index): map hasher(blake2_128_concat) u64 => T::Hash;
		AllContributionsCount get(fn all_contributions_count): u64;
		AllContributionsIndex: map hasher(blake2_128_concat) T::Hash => u64;

		// User contributions
		OwnContributionsArray get(fn own_contributions_by_index): map hasher(blake2_128_concat) (AccountIdOf<T>, u64) => T::Hash;
		OwnContributionsCount get(fn own_contributions_count): map hasher(blake2_128_concat) AccountIdOf<T> => u64;
		OwnContributionsIndex: map hasher(blake2_128_concat) (AccountIdOf<T>, T::Hash) => u64;

		// Investor contributions
		InvestedCampaignsArray get(fn invested_campaigns_by_index): map hasher(blake2_128_concat) (AccountIdOf<T>, u64) => T::Hash;
		InvestedCampaignsCount get(fn invested_campaigns_count): map hasher(blake2_128_concat) AccountIdOf<T> => u64;
		InvestedCampaignsIndex: map hasher(blake2_128_concat) (AccountIdOf<T>, T::Hash) => u64;

		// Investment per project
		InvestmentAmount get(fn investment_amount): map hasher(blake2_128_concat) (T::Hash, AccountIdOf<T>) => BalanceOf<T>;

		// Investor Accounts
		InvestorAccounts get(fn investment_accounts): map hasher(blake2_128_concat) T::Hash => Vec<AccountIdOf<T>>;
		InvestorAccountsCount get(fn investment_accounts_count): map hasher(blake2_128_concat) T::Hash => u64;

		// The campaigns current balance
		CampaignContributionsAmount get(fn total_amount_of_funding): map hasher(blake2_128_concat) T::Hash => BalanceOf<T>;

		// Campaign nonce
		Nonce: u64;

	}
}

//
//	events
//

decl_event! {
	pub enum Event<T> where
		<T as system::Trait>::Hash,
		<T as system::Trait>::AccountId,
		// Balance = BalanceOf<T>, //
		<T as balances::Trait>::Balance,
		<T as system::Trait>::BlockNumber
	{
		CampaignCreated(Hash, AccountId, Balance, Balance, BlockNumber),
		CampaignContributed(Hash, AccountId, Balance, BlockNumber),
		CampaignCompleted(Hash, Balance, BlockNumber, bool),
		// Revert(Hash, Balance, BlockNumber, bool),
		// Withdraw(AccountId, CampaignIndex, Balance, BlockNumber),
		// Retire(CampaignIndex, BlockNumber),
		// Dissolve(CampaignIndex, BlockNumber, AccountId),
		// Dispense(CampaignIndex, BlockNumber, AccountId),

	}
}

//
//
//

// decl_error! {
// 	pub enum Error for Module<T: Trait> {
// 		/// Must contribute at least the minimum amount of Campaigns
// 		ContributionTooSmall,
// 		/// The Campaign id specified does not exist
// 		InvalidId,
// 		/// The Campaign's contribution period has ended; no more contributions will be accepted
// 		ContributionPeriodOver,
// 		/// You may not withdraw or dispense Campaigns while the Campaign is still active
// 		CampaignStillActive,
// 		/// You cannot withdraw Campaigns because you have not contributed any
// 		NoContribution,
// 		/// You cannot dissolve a Campaign that has not yet completed its retirement period
// 		CampaignNotRetired,
// 		/// Cannot dispense Campaigns from an unsuccessful Campaign
// 		UnsuccessfulCampaign,
// 		//
// 		//	create
// 		//
// 		/// Campaign must end after it starts
// 		EndTooEarly,
// 		/// Campaign expiry has be lower than the block number limit
// 		EndTooLate,
// 		/// Max contributions per block exceeded
// 		ContributionsPerBlockExceeded,
// 		/// Name too long
// 		CampaignNameTooLong,
// 		/// Name too short
// 		CampaignNameTooShort,
// 		/// Deposit exceeds the campaign target
// 		CampaignDepositTooHigh,
// 		/// Campaign id exists
// 		CampaignIdExists,
// 		//
// 		//	mint
// 		//
// 		/// Overflow adding a new campaign to total fundings
// 		AddCampaignOverflow,
// 		/// Overflow adding a new contribution to account balance
// 		/// Overflow adding the total number of investors of a camapaign
// 		AddInvestorsOverflow,
// 		//
// 		//	unknown
// 		//
// 		/// Guru Meditation
// 		GuruMeditation,

// 	}
// }

// decl_module! {
// 	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
// 		// errors
// 		type Error = Error<T>;

// 		// init events
// 		fn deposit_event() = default;

// 		/// The minimum length a name may be.
// 		const MinLength: u32 = T::MinLength::get() as u32;

// 		/// The maximum length a name may be.
// 		const MaxLength: u32 = T::MaxLength::get() as u32;

// 		/// Create a campaign
// 		#[weight = 10_000]
// 		fn create_campaign(
// 			origin,
// 			name: Vec<u8>,
// 			target: BalanceOf<T>,
// 			deposit: BalanceOf<T>,
// 			expiry: T::BlockNumber,
// 		) {

// 			// get the creator
// 			let creator = ensure_signed(origin)?;

// 			// check name length boundary
// 			ensure!(
// 				name.len() >= T::MinLength::get(),
// 				Error::<T>::CampaignNameTooShort
// 			);
// 			ensure!(
// 				name.len() <= T::MaxLength::get(),
// 				Error::<T>::CampaignNameTooLong
// 			);

// 			// get the nonce to help generate unique id
// 			let nonce = T::Nonce::get();

// 			// generate the unique id
// 			let id =(<system::Module<T>>::random_seed(), &creator, nonce)
// 				.using_encoded(<T as system::Trait>::Hashing::hash);

// 			// ensure unique id
// 			ensure!(
// 				!<CampaignOwner<T>>::exists(&id),
// 				Error::<T>::CampaignIdExists
// 			);

// 			// ensure deposit <= target
// 			ensure!(
// 				deposit <= target,
// 				Error::<T>::CampaignDepositTooHigh
// 			);

// 			let now = <system::Module<T>>::block_number();

// 			// ensure campaign expires after now
// 			ensure!(
// 				expiry > now,
// 				Error::<T>::EndTooEarly
// 			);
// 			ensure!(
// 				expiry <= <system::Module<T>>::block_number() + Self::campaign_max_duration_limit(),
// 				Error::<T>::EndTooLate
// 			);

// 			// check contribution limit per block
// 			let contributions = Self::campaign_expire_at(expiry);
// 			ensure!(
// 				contributions.len() < MAX_CONTRIBUTIONS_PER_BLOCK,
// 				Error::<T>::ContributionsPerBlockExceeded
// 			);

// 			// create a new campaign
// 			// id: Hash,
// 			// name: Vec<u8>,
// 			// manager: AccountId,
// 			// deposit: Balance,
// 			// expiry: BlockNumber,
// 			// cap: Balance,
// 			// status: u8,
// 			let new_campaign = Campaign {
// 				id: id.clone(),
// 				name: name,
// 				manager: creator.clone(),
// 				deposit: deposit.clone(),
// 				expiry: expiry,
// 				cap: target,
// 				status: 0,
// 			};

// 			// mint the campaign
// 			Self::mint(
// 				creator.clone(),
// 				id.clone(),
// 				expiry.clone(),
// 				deposit.clone(),
// 				new_campaign
// 			)?;



// 			// deposit the event
// 			Self::deposit_event(
// 				RawEvent::CampaignCreated(
// 					id,
// 					creator,
// 					target,
// 					deposit,
// 					expiry
// 				)
// 			);
// 			Ok(())


// 			// No fees are paid here if we need to create this account; that's why we don't just
// 			// use the stock `transfer`.
// 			// T::Currency::resolve_creating(&Self::campaign_account_id(index), imb);

// 			// <Campaigns<T>>::insert(index, Campaign {
// 			// 	beneficiary,
// 			// 	deposit,
// 			// 	raised: Zero::zero(),
// 			// 	end,
// 			// 	goal,
// 			// });

// 			// Self::deposit_event(RawEvent::Created(index, now));
// 		}

// 		// /// Contribute Campaigns to an existing Campaign
// 		// #[weight = 10_000]
// 		// fn contribute(origin, index: CampaignIndex, value: BalanceOf<T>) {
// 		// 	let who = ensure_signed(origin)?;

// 		// 	ensure!(value >= T::MinContribution::get(), Error::<T>::ContributionTooSmall);
// 		// 	let mut campaign = Self::campaigns(index).ok_or(Error::<T>::InvalidIndex)?;

// 		// 	// Make sure crowdCampaign has not ended
// 		// 	let now = <system::Module<T>>::block_number();
// 		// 	ensure!(campaign.end > now, Error::<T>::ContributionPeriodOver);

// 		// 	// Add contribution to the campaign
// 		// 	T::Currency::transfer(
// 		// 		&who,
// 		// 		&Self::campaign_account_id(index),
// 		// 		value,
// 		// 		ExistenceRequirement::AllowDeath
// 		// 	)?;
// 		// 	campaign.raised += value;
// 		// 	Campaigns::<T>::insert(index, &campaign);

// 		// 	let balance = Self::contribution_get(index, &who);
// 		// 	let balance = balance.saturating_add(value);
// 		// 	Self::contribution_put(index, &who, &balance);

// 		// 	Self::deposit_event(RawEvent::Contributed(who, index, balance, now));
// 		// }

// 		// /// Withdraw full balance of a contributor to a Campaign
// 		// #[weight = 10_000]
// 		// fn withdraw(origin, #[compact] index: CampaignIndex) {
// 		// 	let who = ensure_signed(origin)?;

// 		// 	let mut campaign = Self::campaigns(index).ok_or(Error::<T>::InvalidIndex)?;
// 		// 	let now = <system::Module<T>>::block_number();
// 		// 	ensure!(campaign.end < now, Error::<T>::CampaignStillActive);

// 		// 	let balance = Self::contribution_get(index, &who);
// 		// 	ensure!(balance > Zero::zero(), Error::<T>::NoContribution);

// 		// 	// Return Campaigns to caller without charging a transfer fee
// 		// 	let _ = T::Currency::resolve_into_existing(&who, T::Currency::withdraw(
// 		// 		&Self::campaign_account_id(index),
// 		// 		balance,
// 		// 		WithdrawReasons::from(WithdrawReason::Transfer),
// 		// 		ExistenceRequirement::AllowDeath
// 		// 	)?);

// 		// 	// Update storage
// 		// 	Self::contribution_kill(index, &who);
// 		// 	campaign.raised = campaign.raised.saturating_sub(balance);
// 		// 	<Campaigns<T>>::insert(index, &campaign);

// 		// 	Self::deposit_event(RawEvent::Withdrew(who, index, balance, now));
// 		// }

// 		// /// Dissolve an entire crowdCampaign after its retirement period has expired.
// 		// /// Anyone can call this function, and they are incentivized to do so because
// 		// /// they inherit the deposit.
// 		// #[weight = 10_000]
// 		// fn dissolve(origin, index: CampaignIndex) {
// 		// 	let reporter = ensure_signed(origin)?;

// 		// 	let campaign = Self::campaigns(index).ok_or(Error::<T>::InvalidIndex)?;

// 		// 	// Check that enough time has passed to remove from storage
// 		// 	let now = <system::Module<T>>::block_number();
// 		// 	ensure!(now >= campaign.end + T::RetirementPeriod::get(), Error::<T>::CampaignNotRetired);

// 		// 	let account = Self::campaign_account_id(index);

// 		// 	// Dissolver collects the deposit and any remaining Campaigns
// 		// 	let _ = T::Currency::resolve_creating(&reporter, T::Currency::withdraw(
// 		// 		&account,
// 		// 		campaign.deposit + campaign.raised,
// 		// 		WithdrawReasons::from(WithdrawReason::Transfer),
// 		// 		ExistenceRequirement::AllowDeath,
// 		// 	)?);

// 		// 	// Remove the campaign info from storage
// 		// 	<Campaigns<T>>::remove(index);
// 		// 	// Remove all the contributor info from storage in a single write.
// 		// 	// This is possible thanks to the use of a child tree.
// 		// 	Self::campaign_kill(index);

// 		// 	Self::deposit_event(RawEvent::Dissolved(index, now, reporter));
// 		// }

// 		// /// Dispense a payment to the beneficiary of a successful crowdCampaign.
// 		// /// The beneficiary receives the contributed Campaigns and the caller receives
// 		// /// the deposit as a reward to incentivize clearing settled crowdCampaigns out of storage.
// 		// #[weight = 10_000]
// 		// fn dispense(origin, index: CampaignIndex) {
// 		// 	let caller = ensure_signed(origin)?;

// 		// 	let campaign = Self::campaigns(index).ok_or(Error::<T>::InvalidIndex)?;

// 		// 	// Check that enough time has passed to remove from storage
// 		// 	let now = <system::Module<T>>::block_number();

// 		// 	ensure!(now >= campaign.end, Error::<T>::CampaignStillActive);

// 		// 	// Check that the Campaign was actually successful
// 		// 	ensure!(campaign.raised >= campaign.goal, Error::<T>::UnsuccessfulCampaign);

// 		// 	let account = Self::campaign_account_id(index);

// 		// 	// Beneficiary collects the contributed Campaigns
// 		// 	let _ = T::Currency::resolve_creating(&campaign.beneficiary, T::Currency::withdraw(
// 		// 		&account,
// 		// 		campaign.raised,
// 		// 		WithdrawReasons::from(WithdrawReason::Transfer),
// 		// 		ExistenceRequirement::AllowDeath,
// 		// 	)?);

// 		// 	// Caller collects the deposit
// 		// 	let _ = T::Currency::resolve_creating(&caller, T::Currency::withdraw(
// 		// 		&account,
// 		// 		campaign.deposit,
// 		// 		WithdrawReasons::from(WithdrawReason::Transfer),
// 		// 		ExistenceRequirement::AllowDeath,
// 		// 	)?);

// 		// 	// Remove the campaign info from storage
// 		// 	<Campaigns<T>>::remove(index);
// 		// 	// Remove all the contributor info from storage in a single write.
// 		// 	// This is possible thanks to the use of a child tree.
// 		// 	Self::campaign_kill(index);

// 		// 	Self::deposit_event(RawEvent::Dispensed(index, now, caller));
// 		// }
// 	}
// }

impl<T: Trait + balances::Trait> Module<T> {

	fn mint(
		sender: T::AccountId,
		id: T::Hash,
		expiry: T::BlockNumber,
		deposit: BalanceOf<T>,
		new_campaign: Campaign<T::Hash, T::AccountId, BalanceOf<T>, T::BlockNumber>
	) -> DispatchResult {


		// QUESTION: is this contributions or campaigns?
		let all_contributions_count = Self::all_contributions_count();
		let new_all_contributions_count = all_contributions_count.checked_add(1).ok_or("Overflow adding a new funding to total fundings")?;

		// QUESTION: is this contributions or campaigns?
		let own_contributions_count = Self::own_contributions_count(&sender);
		let new_own_contributions_count = own_contributions_count.checked_add(1).ok_or("Overflow adding a new funding to account balance")?;

		// change the global states
		<Campaigns<T>>::insert(id.clone(), new_campaign.clone());
		<CampaignOwner<T>>::insert(id.clone(), sender.clone());
		<CampaignsByBlockNumber<T>>::mutate(expiry.clone(), |campaigns| campaigns.push(id.clone()));

		// update global contributions
		<AllContributionsArray<T>>::insert(&all_contributions_count, id.clone());
		<AllContributionsCount>::put(new_all_contributions_count);
		<AllContributionsIndex<T>>::insert(id.clone(), all_contributions_count);

		// update individual contributions
		<OwnContributionsArray<T>>::insert((sender.clone(), own_contributions_count.clone()), id.clone());
		<OwnContributionsCount<T>>::insert(&sender, new_own_contributions_count);
		<OwnContributionsIndex<T>>::insert((sender.clone(), id.clone()), own_contributions_count);

		// when deposit exceeds available balance,
		// revert campaign creation

		// if deposit > BalanceOf::sa(0) {
		// 	match Self::not_invest_before(sender.clone(), id.clone(), deposit.clone()){
		// 		// If the invest function meets error then revert the storage
		// 		Err(_e) => {
		// 			<Campaigns<T>>::remove(id.clone());
		// 			<CampaignOwner<T>>::remove(id.clone());
		// 			<CampaignsByBlockNumber<T>>::mutate(expiry,|campaigns| campaigns.pop());
		// 			<AllContributionsArray<T>>::remove(&all_contributions_count);
		// 			<AllContributionsCount>::put(all_contributions_count.clone());
		// 			<AllContributionsIndex<T>>::remove(id.clone());
		// 			<OwnContributionsArray<T>>::remove((sender.clone(), own_contributions_count.clone()));
		// 			<OwnContributionsCount<T>>::remove(&sender);
		// 			<OwnContributionsIndex<T>>::remove((sender.clone(), id.clone()));
		// 		},
		// 		Ok(_v) => {}
		// 	}
		// }

		// add the nonce
		Nonce::mutate(|n| *n += 1);

		Ok(())
	}

	// The investor has contributed to the project before
	// Now he wants to increase the contribution

	// fn invest_before(
	// 	sender: T::AccountId,
	// 	campaign_id: T::Hash,
	// 	invest_amount: BalanceOf<T>
	// ) -> DispatchResult {

	// 	// ensure the funding exists
	// 	ensure!(<Campaigns<T>>::exists(campaign_id), "The funding does not exist");

	// 	// ensure the investor has enough money
	// 	// ensure!(<balances::Module<T>>::free_balance(sender.clone()) >= investment_amount, "You don't have enough free balance for investing for the funding");

	// 	// get the campaign
	// 	let campaign = Self::campaign_by_id(&campaign_id);
	// 	// ensure that the campaign is valid to invest
	// 	ensure!(<system::Module<T>>::block_number() < campaign.expiry, "This campaign has expired.");

	// 	// reserve the amount of money
	// 	<balances::Module<T>>::reserve(&sender, invest_amount)?;

	// 	let amount_of_investment_on_campaign = Self::investment_amount_of((campaign_id.clone(), sender.clone()));
	// 	let new_amount_of_investment_on_campaign = amount_of_investment_on_campaign + invest_amount.clone();

	// 	//change the amount of the investor has invested
	// 	<InvestmentAmount<T>>::insert((campaign_id, sender), new_amount_of_investment_on_campaign.clone());

	// 	// get the total amount of the project and add invest_amount
	// 	let amount_of_funding = Self::total_amount_of_funding(&campaign_id);
	// 	let new_amount_of_funding = amount_of_funding + invest_amount;

	// 	// change the total amount of the project has collected
	// 	<CampaignContributionsAmount<T>>::insert(&campaign_id, new_amount_of_funding);

	// 	Ok(())
	// }

	// An investor wants to contribute to a new campaign
	// he did not contribute to before

	// fn not_invest_before(
	// 	sender: T::AccountId,
	// 	campaign_id: T::Hash,
	// 	invest_amount: BalanceOf<T>
	// ) -> DispatchResult {

	// 	// ensure campaign exists
	// 	ensure!(<Campaigns<T>>::exists(campaign_id), "The funding does not exist");

	// 	// ensure investor has enough money
	// 	ensure!(<balances::Module<T>>::free_balance(sender.clone()) >= investment_amount, "You don't have enough free balance for investing for the funding");

	// 	// increase the number of campaigns investor invested in
	// 	let invested_campaigns_count = Self::invested_campaigns_count(&sender);
	// 	let new_invested_campaigns_count = invested_campaigns_count.checked_add(1).ok_or("Overflow adding a new invested funding")?;

	// 	// increase the number of investors into the campaign
	// 	let investor_count = <InvestAccountsCount<T>>::get(&campaign_id);
	// 	let new_investor_count = investor_count.checked_add(1).ok_or("Overflow adding the total number of investors of a campaign")?;

	// 	// get the funding
	// 	let campaign = Self::campaign_by_id(&campaign_id);
	// 	// ensure that the project is valid to invest
	// 	ensure!(<system::Module<T>>::block_number() < campaign.expiry, "This funding is expired.");

	// 	// reserve the amount of money
	// 	<balances::Module<T>>::reserve(&sender, invest_amount)?;

	// 	// change the state of invest related fields
	// 	<InvestmentAmount<T>>::insert((campaign_id.clone(), sender.clone()), invest_amount.clone());
	// 	<InvestAccounts<T>>::mutate(&campaign_id, |accounts| accounts.push(sender.clone()));

	// 	// add total support count
	// 	<InvestAccountsCount<T>>::insert(campaign_id.clone(), new_investor_count);

	// 	// change the state of invest related fields
	// 	<InvestedCampaignsArray<T>>::insert((sender.clone(), invested_funding_count), campaign_id.clone());
	// 	<InvestedCampaignsCount<T>>::insert(&sender, new_invested_funding_count);
	// 	<InvestedCampaignsIndex<T>>::insert((sender.clone(), campaign_id.clone()), invested_funding_count);

	// 	// get the total amount of the project and add invest_amount
	// 	let amount_of_funding = Self::total_amount_of_funding(&campaign_id);
	// 	let new_amount_of_funding = amount_of_funding + invest_amount;

	// 	// change the total amount of the project has collected
	// 	<CampaignContributionsAmount<T>>::insert(&campaign_id, new_amount_of_funding);

	// 	Ok(())
	// }

	// pub fn is_campaign_exists(campaign_id: T::Hash) -> bool{
	// 	<Campaigns<T>>::exists(campaign_id)
	// }

	// pub fn is_campaign_success(campaign_id: T::Hash) -> u64{
	// 	<Campaigns<T>>::get(campaign_id).status
	// }

	// pub fn get_campaign_owner(campaign_id: T::Hash) -> Option<T::AccountId> {
	// 	<CampaignOwner<T>>::get(campaign_id)
	// }

	// pub fn get_campaign_total_balance(campaign_id: T::Hash) -> BalanceOf<T>{
	// 	<CampaignContributionsAmount<T>>::get(campaign_id)
	// }

	// pub fn is_investor(campaign_id: T::Hash, from: T::AccountId) -> bool{
	// 	<InvestmentAmount<T>>::exists((campaign_id, from))
	// }

	// pub fn get_invested_number(campaign_id: T::Hash) -> u64{
	// 	<InvestAccountsCount<T>>::get(campaign_id)
	// }


	// /// The account ID of the campaign pot.
	// ///
	// /// This actually does computation. If you need to keep using it, then make sure you cache the
	// /// value and only call this once.
	// pub fn campaign_account_id(hash: T::Hash) -> T::AccountId {
	// 	PALLET_ID.into_sub_account(hash)
	// }

	// /// Find the ID associated with the campaign
	// ///
	// /// Each campaign stores information about its contributors and their contributions in a child trie
	// /// This helper function calculates the id of the associated child trie.
	// pub fn id_from_index(hash: Hash) -> child::ChildInfo {
	// 	let mut buf = Vec::new();
	// 	buf.extend_from_slice(b"crowdfnd");
	// 	buf.extend_from_slice(&index.to_le_bytes()[..]);

	// 	child::ChildInfo::new_default(T::Hashing::hash(&buf[..]).as_ref())
	// }

	// /// Record a contribution in the associated child trie.
	// pub fn contribution_put(hash: Hash, who: &T::AccountId, balance: &T::Balance) {
	// 	let id = Self::id_from_index(index);
	// 	who.using_encoded(|b| child::put(&id, b, &balance));
	// }

	// /// Lookup a contribution in the associated child trie.
	// pub fn contribution_get(hash: Hash, who: &T::AccountId) -> T::Balance {
	// 	let id = Self::id_from_index(index);
	// 	who.using_encoded(|b| child::get_or_default::<T::Balance>(&id, b))
	// }

	// /// Remove a contribution from an associated child trie.
	// pub fn contribution_kill(hash: Hash, who: &T::AccountId) {
	// 	let id = Self::id_from_index(index);
	// 	who.using_encoded(|b| child::kill(&id, b));
	// }

	// /// Remove the entire record of contributions in the associated child trie in a single
	// /// storage write.
	// pub fn campaign_kill(index: CampaignIndex) {
	// 	let id = Self::id_from_index(index);
	// 	child::kill_storage(&id);
	// }
}
