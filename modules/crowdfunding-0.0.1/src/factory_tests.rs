
#[cfg(test)]
mod tests{
	use super::*;

	use support::{impl_outer_origin, assert_ok, assert_noop};
	use runtime_io::{with_externalities, TestExternalities};
	use primitives::{H256, Blake2Hasher};
	use runtime_primitives::{
		BuildStorage,
		traits::{BlakeTwo256, IdentityLookup},
		testing::{Digest, DigestItem, Header}
	};

	impl_outer_origin! {
		pub enum Origin for FundingTest {}
	}

	#[derive(Clone, Eq, PartialEq)]
	pub struct FundingTest;

	impl system::Trait for FundingTest {
		type Origin = Origin;
		type Index = u64;
		type BlockNumber = u64;
		type Hash = H256;
		type Hashing = BlakeTwo256;
		type Digest = Digest;
		type AccountId = u64;
		type Lookup = IdentityLookup<Self::AccountId>;
		type Header = Header;
		type Event = ();
		type Log = DigestItem;
	}

	impl balances::Trait for FundingTest {
		type Balance = u64;
		type OnFreeBalanceZero = ();
		type OnNewAccount = ();
		type Event = ();
		type TransactionPayment = ();
		type TransferPayment = ();
		type DustRemoval = ();
	}

	impl super::Trait for FundingTest {
		type Event = ();
	}

	type Fundings = super::Module<FundingTest>;

	fn build_ext() -> TestExternalities<Blake2Hasher> {
		let mut t = system::GenesisConfig::<FundingTest>::default().build_storage().unwrap().0;
		t.extend(balances::GenesisConfig::<FundingTest>::default().build_storage().unwrap().0);
		t.into()
	}

	#[test]
	fn create_funding_should_work() {
		with_externalities(&mut build_ext(), || {
			// create a funding with account #6.
			assert_ok!(Fundings::create_funding(Origin::signed(6), vec![12,56], 20000, 0, 1000));

			// check that there are now 3 fundings in storage
			assert_eq!(Fundings::all_funding_count(), 1);

			// check that account #6 owns 1 funding
			assert_eq!(Fundings::owned_funding_count(6), 1);

			// check that some random account #5 does not own a funding
			assert_eq!(Fundings::owned_funding_count(5), 0);

			// check that this funding is specifically owned by account #6
			let hash = Fundings::funding_by_index(0);
			assert_eq!(Fundings::owner_of(hash), Some(6));

			let other_hash = Fundings::funding_of_owner_by_index((6, 0));
			assert_eq!(hash, other_hash);
		})
	}
}
