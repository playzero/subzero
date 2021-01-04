use frame_system::{ self as system };
use frame_support::{
	decl_error, decl_event, decl_module, decl_storage, ensure, storage::child,
	traits::{ Currency, ExistenceRequirement, Get, ReservableCurrency, WithdrawReason, WithdrawReasons, },
};
/// pallet configuration trait
pub trait Trait: system::Trait {
	/// The ubiquitous Event type
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
	/// The currency in which the crowdCampaigns will be denominated
	type Currency: ReservableCurrency<Self::AccountId>;
	/// The amount to be held on deposit by the owner of a crowdCampaign
	type SubmissionDeposit: Get<BalanceOf<Self>>;
	/// The minimum amount that may be contributed into a crowdCampaign.
	/// Should almost certainly be at least ExistentialDeposit.
	type MinContribution: Get<BalanceOf<Self>>;
	/// The period of time (in blocks) after an unsuccessful crowdCampaign ending during which
	/// contributors are able to withdraw their Campaigns. After this period, their Campaigns are lost.
	type RetirementPeriod: Get<Self::BlockNumber>;
}

