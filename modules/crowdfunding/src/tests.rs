// #[cfg(test)]
// mod tests {

// 	use super::*;

// 	use support::{impl_outer_origin, assert_ok, assert_noop};
// 	use runtime_io::{with_externalities, TestExternalities};
// 	use primitives::{H256, Blake2Hasher};
// 	use runtime_primitives::{
// 		BuildStorage,
// 		traits::{BlakeTwo256, IdentityLookup},
// 		testing::{Digest, DigestItem, Header}
// 	};

// 	impl_outer_origin! {
// 		pub enum Origin for CampaignTest {}
// 	}

// 	#[derive(Clone, Eq, PartialEq)]
// 	pub struct CampaignTest;

// 	impl system::Trait for CampaignTest {
// 		type Origin = Origin;
// 		type Index = u64;
// 		type BlockNumber = u64;
// 		type Hash = H256;
// 		type Hashing = BlakeTwo256;
// 		type Digest = Digest;
// 		type AccountId = u64;
// 		type Lookup = IdentityLookup<Self::AccountId>;
// 		type Header = Header;
// 		type Event = ();
// 		type Log = DigestItem;
// 	}

// 	impl balances::Trait for CampaignTest {
// 		type Balance = u64;
// 		type OnFreeBalanceZero = ();
// 		type OnNewAccount = ();
// 		type Event = ();
// 		type TransactionPayment = ();
// 		type TransferPayment = ();
// 		type DustRemoval = ();
// 	}

// 	impl super::Trait for CampaignTest {
// 		type Event = ();
// 	}

// 	type Campaigns = super::Module<CampaignTest>;

// 	fn build_ext() -> TestExternalities<Blake2Hasher> {
// 		let mut t = system::GenesisConfig::<CampaignTest>::default().build_storage().unwrap().0;
// 		t.extend(balances::GenesisConfig::<CampaignTest>::default().build_storage().unwrap().0);
// 		t.into()
// 	}

// 	#[test]
// 	fn create_campaign_should_work() {
// 		with_externalities(&mut build_ext(), {

// 			// create a Campaign with account #5.
// 			assert_ok!(Campaigns::create(Origin::signed(5), vec![12,56], 20000, 0, 1000));

// 			// check that there are now 1 Campaigns in storage
// 			assert_eq!(Campaigns::all_campaigns_count(), 1);

// 			// check that account #6 owns 1 Campaign
// 			assert_eq!(Campaigns::owned_campaigns_count(5), 1);

// 			// check that some random account #3 does not own a Campaign
// 			assert_eq!(Campaigns::owned_campaigns_count(3), 0);

// 			// check that this Campaign is specifically owned by account #6
// 			let hash = Campaigns::campaign_by_id(0);
// 			assert_eq!(Campaigns::owner_of(hash), Some(5));

// 			let other_hash = Campaigns::owned_campaigns_by_index(5, 0);
// 			assert_eq!(hash, other_hash);
// 		})
// 	}

// 	// #[test]
// 	// fn contribute_to_campaign_should_work() {
// 	// 	with_externalities(&mut build_ext(), || {

// 	// 		// create a Campaign with account #5.
// 	// 		assert_ok!(Campaigns::create(Origin::signed(5), vec![12,56], 20000, 0, 1000));

// 	// 		// check that there are now 1 Campaigns in storage
// 	// 		assert_eq!(Campaigns::all_campaigns_count(), 1);

// 	// 		// check that account #6 owns 1 Campaign
// 	// 		assert_eq!(Campaigns::owned_campaigns_count(5), 1);

// 	// 		// check that some random account #3 does not own a Campaign
// 	// 		assert_eq!(Campaigns::owned_campaigns_count(3), 0);

// 	// 		// check that this Campaign is specifically owned by account #6
// 	// 		let hash = Campaigns::campaign_by_id(0);
// 	// 		assert_eq!(Campaigns::owner_of(hash), Some(5));

// 	// 		let other_hash = Campaigns::owned_campaigns_by_index(5, 0));
// 	// 		assert_eq!(hash, other_hash);
// 	// 	})
// 	// }

// }
