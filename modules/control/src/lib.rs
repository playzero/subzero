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

//! CONTROL
//!
//! This pallet invokes the initial control body for various functions.

#![cfg_attr(not(feature = "std"), no_std)]
#![feature(derive_default_enum)]

// TODO: harden checks on completion
#![allow(clippy::unused_unit)]
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

// TODO:
// mod default_weight;
// mod mock;
// mod tests;

pub use module::*;

pub mod module {

	use frame_support::{
		decl_error, decl_event, decl_module, decl_storage,
		ensure,
		dispatch::DispatchResult,
		traits::{ Get, Currency, EnsureOrigin, Randomness, ReservableCurrency, ExistenceRequirement },
	};
	use frame_system::{ self as system, ensure_root, ensure_signed };
	use codec::{ Encode, Decode };
	use sp_runtime::{
		traits::{ Hash, Zero },
		ModuleId
	};
	use sp_std::prelude::*;
	use pallet_balances::{ self as balances };
	use primitives:: {
		Balance,
		// AccountId,
	};

	// use hex_literal;

	use tangram;
	use tangram::Module as Tangram;
	// use tangram::nft::Items;
	use tangram::{
		Call::create_realm,
		Call::create_class,
		NextRealmIndex
	};

	use scale_info::TypeInfo;
	#[cfg(feature = "std")]
	use serde::{Deserialize, Deserializer, Serialize, Serializer};

	// use tangram::Call::create_class;

	#[derive(Encode, Decode, PartialEq, Clone, Eq, Default, PartialOrd, Ord, TypeInfo)]
	#[repr(u8)]
	#[derive(Debug)]
	pub enum ControlType {
	#[default]
		Individual = 0, // individual
		Company = 1,    // offchain body
		Dao = 2,        // dao
		Hybrid = 3,
	}

	#[derive(Encode, Decode, PartialEq, Clone, Eq, PartialOrd, Ord, TypeInfo)]
	#[repr(u8)]
	pub enum ControlState {
		Inactive = 0,
		Active = 1,
		Locked = 2,
	}

	#[derive(Encode, Decode, PartialEq, Clone, Eq, PartialOrd, Ord, TypeInfo)]
	#[repr(u8)]
	pub enum ControlMemberState {
		Inactive = 0, // eg inactive after threshold period
		Active = 1,   // active
		Pending = 2,  // application voting pending
		Kicked = 3,
		Banned = 4,
		Exited = 5,
	}

	#[derive(Encode, Decode, Clone, PartialEq, Default, Eq, PartialOrd, Ord, TypeInfo)]
	#[repr(u8)]
	#[derive(Debug)]
	pub enum ControlFeeModel {
	#[default]
		NoFees = 0,		// feeless
		Reserve = 1,	// amount is reserved in user account
		Transfer = 2,	// amount is transfered to DAO treasury
	}

	#[derive(Encode, Decode, Clone, PartialEq, Default, Eq, PartialOrd, Ord, TypeInfo)]
	#[repr(u8)]
	#[derive(Debug)]
	pub enum ControlAccessModel {
	#[default]
		Open = 0,		// anybody can join
		Voting = 1,		// application creates membership voting
		Controller = 2,	// controller invites
	}

	//
	//
	//

	pub const MODULE_ID: ModuleId = ModuleId(*b"dao/ctrl");
	pub const MODULE_VERSION: &str = "1.0";

	//
	//
	//

	pub trait Config: system::Config + balances::Config + tangram::Config {

		type GameDAOTreasury: Get<<Self as frame_system::Config>::AccountId>;

		// supervision
		type ForceOrigin: EnsureOrigin<Self::Origin>;

		// treasury, fees
		// connect to individual gamedao treasury
		// to separate fee collection from net
		// type Treasury: Get<Self::AccountId>;
		type Currency: ReservableCurrency<Self::AccountId>;
		type CreationFee: Get<Self::Balance>;

		// bounds
		type MaxBodiesPerAccount: Get<usize>;
		type MaxMembersPerBody: Get<usize>;
		type MaxCreationsPerBlock: Get<usize>;

		//
		type Event: From<Event<Self>> + Into<<Self as system::Config>::Event>;
		type Randomness: Randomness<Self::Hash>;

	}

	//
	//
	//

	/// Body
	#[derive(Encode, Decode, Default, PartialEq, Eq)]
	#[cfg_attr(feature = "std", derive(Debug))]
	pub struct Body<Hash, AccountId, BlockNumber, ControlType> {
		id: Hash,                // general hash
		index: u128,             // nonce
		creator: AccountId,      // creator
		name: Vec<u8>,           // body name
		cid: Vec<u8>,            // cid -> ipfs
		body: ControlType,       // individual | legal body | dao
		created: BlockNumber,
		mutated: BlockNumber,
	}

	/// Body Config
	// TODO: refactor to bits
	#[derive(Encode, Decode, Default, PartialEq, Eq)]
	#[cfg_attr(feature = "std", derive(Debug))]
	pub struct BConfig<Balance, ControlFeeModel, ControlAccessModel> {
		fee_model: ControlFeeModel, // only TX by OS | fees are reserved | fees are moved to treasury
		fee: Balance,        // plain fee amount
		gov_asset: u8,       // gov
		pay_asset: u8,       // pay
		access: ControlAccessModel, // 0 open, 1 voting by members, 2 invite by controller
		member_limit: u64,   // max members allowed
	}

	//
	//	storage should be optimized to offload these maps to a graph / where more efficient
	//

	decl_storage! {
		trait Store for Module<T: Config> as Control50 {

			/// DAO by hash
			Bodies get(fn body_by_hash): map hasher(blake2_128_concat) T::Hash => Body<T::Hash, T::AccountId, T::BlockNumber, ControlType>;
			/// DAO by nonce
			BodyByNonce get(fn body_by_nonce): map hasher(blake2_128_concat) u128 => T::Hash;
			/// DAO mutable settings
			BodyConfig get(fn body_config): map hasher(blake2_128_concat) T::Hash => BConfig<T::Balance, ControlFeeModel, ControlAccessModel>;

			/// Global DAO State
			BodyState get(fn body_state): map hasher(blake2_128_concat) T::Hash => ControlState = ControlState::Inactive;

			/// Access model
			BodyAccess get(fn body_access): map hasher(blake2_128_concat) T::Hash => ControlAccessModel = ControlAccessModel::Open;

			/// Members of a DAO
			BodyMembers get(fn body_members): map hasher(blake2_128_concat) T::Hash => Vec<T::AccountId>;
			/// Membercount of a DAO
			BodyMemberCount get(fn body_member_count): map hasher(blake2_128_concat) T::Hash => u64 = 0;

			/// Member state for a DAO
			BodyMemberState get(fn body_member_state): map hasher(blake2_128_concat) (T::Hash, T::AccountId) => ControlMemberState = ControlMemberState::Inactive;
			/// Memberships by AccountId
			Memberships get(fn memberships): map hasher(blake2_128_concat) T::AccountId => Vec<T::Hash>;

			// significant accounts

			/// Creator of a body
			BodyCreator get(fn body_creator): map hasher(blake2_128_concat) T::Hash => T::AccountId;
			/// Admin Controller of a body
			BodyController get(fn body_controller): map hasher(blake2_128_concat) T::Hash => T::AccountId;
			/// Treasury of a body
			BodyTreasury get(fn body_treasury): map hasher(blake2_128_concat) T::Hash => T::AccountId;

			//

			/// DAOs created by account
			CreatedBodies get(fn by_creator): map hasher(blake2_128_concat) T::AccountId => Vec<T::Hash>;
			/// Number of DAOs created by account
			CreatedBodiesCount get(fn by_creator_count): map hasher(blake2_128_concat) T::AccountId => u64 = 0;

			/// DAOs controlled by account
			ControlledBodies get(fn by_controller): map hasher(blake2_128_concat) T::AccountId => Vec<T::Hash>;
			/// Number of DAOs controlled by account
			ControlledBodiesCount get(fn by_controller_count): map hasher(blake2_128_concat) T::AccountId => u64 = 0;

			/// the goode olde nonce
			Nonce: u128;

		}

		// add_extra_genesis {
		// 	config(balances): Vec<(T::AccountId, Vec<T::CommodityInfo>)>;
		// 	build(|config: &GenesisConfig<T, I>| {
		// 		for (who, assets) in config.balances.iter() {
		// 			for asset in assets {
		// 			match <Module::<T, I> as UniqueAssets::<T::AccountId>>::mint(who, asset.clone()) {
		// 				Ok(_) => {}
		// 				Err(err) => { panic!(err) },
		// 			}
		// 			}
		// 		}
		// 	});
		// }
	}

	//
	//
	//

	decl_module! {
		pub struct Module<T: Config> for enum Call where origin: T::Origin {

			fn deposit_event() = default;
			type Error = Error<T>;

			// Enable DAO
			// currently root, layer supervisor
			// enables an org to be used
			// hash: an organisations hash
			#[weight = 1_000_000]
			fn enable(
				origin,
				hash: T::Hash,
			) -> DispatchResult {
				ensure_root(origin)?;
				<BodyState<T>>::insert( hash.clone(), ControlState::Active );
				let now = <system::Module<T>>::block_number();
				Self::deposit_event( RawEvent::BodyDisabled( hash ) );
				Ok(())
			}

			// Disable DAO
			// currently root, layer supervisor
			// disables an org to be used
			// hash: an organisations hash
			#[weight = 1_000_000]
			fn disable(
				origin,
				hash: T::Hash,
			) -> DispatchResult {
				ensure_root(origin)?;
				<BodyState<T>>::insert( hash.clone(), ControlState::Inactive );
				let now = <system::Module<T>>::block_number();
				Self::deposit_event( RawEvent::BodyDisabled( hash ) );
				Ok(())
			}

			// Create DAO
			// create an on chain organisation
			// creator: T::AccountId,      // creator
			// controller: T::AccountId,   // current controller
			// treasury: T::AccountId,     // treasury
			// name: Vec<u8>,              // body name
			// cid: Vec<u8>,               // cid -> ipfs
			// body: u8,                   // individual | legal body | dao
			// access: u8,                 // anybody can join | only member can add | only controller can add
			// fee_model: u8,              // only TX by OS | fees are reserved | fees are moved to treasury
			// fee: T::Balance,
			// gov_asset: u8,              // control assets to empower actors
			// pay_asset: u8,
			// member_limit: u64,          // max members, if 0 == no limit
			// // mint: T::Balance,		// cost to mint
			// // burn: T::Balance,		// cost to burn
			// // strategy: u16,
			#[weight = 5_000_000]
			fn create(
				origin,
				controller: T::AccountId,
				treasury: T::AccountId,
				name: Vec<u8>,
				cid: Vec<u8>,
				body: ControlType,
				access: ControlAccessModel,
				fee_model: ControlFeeModel,
				fee: T::Balance,
				gov_asset: u8,
				pay_asset: u8,
				member_limit: u64,
				// mint: T::Balance,
				// burn: T::Balance,
				// strategy: u16,
			) -> DispatchResult {

				let sender = ensure_signed(origin.clone())?;

				// set up fee
				let creation_fee = T::CreationFee::get();

				// TODO: fix balance check
				// it seems like the balances coming from polkadotjs app work
				// the ones coming from gamedao beta dont
				// controller and treasury need enough balance
				let free_balance = balances::Module::<T>::free_balance(&sender);
				// ensure!( free_balance > creation_fee, Error::<T>::BalanceTooLow );
				let free_balance_treasury = balances::Module::<T>::free_balance(&treasury);
				// ensure!( free_balance_treasury > creation_fee, Error::<T>::BalanceTooLow );

				// controller and treasury must not be equal
				ensure!(&controller != &treasury, Error::<T>::DuplicateAddress );

				let now   = <system::Module<T>>::block_number();
				let hash = <T as Config>::Randomness::random(&name);
				let index = Nonce::get();
				let state = ControlState::Active; // live
				// TODO: create enums for bonding strategies
				let strategy = 0;

				let body_data = Body {
					id:       hash.clone(),
					index:    index.clone(),
					creator:  sender.clone(),
					name:     name.clone(),
					cid:      cid,
					body:     body.clone(),
					created:  now.clone(),
					mutated:  now.clone(),
				};
				Bodies::<T>::insert( hash.clone(), body_data );

				// membership fees
				let mut _fee = T::Balance::zero();
				match &fee_model {
					ControlFeeModel::Reserve => { _fee = fee },
					ControlFeeModel::Transfer => { _fee = fee },
					_ => { }
				};

				let config_data = BConfig {
					fee_model: fee_model.clone(),
					fee: _fee.clone(),
					gov_asset: gov_asset.clone(),
					pay_asset: pay_asset.clone(),
					member_limit: member_limit.clone(),
					access: access.clone()
				};
				BodyConfig::<T>::insert( hash.clone(), config_data );

				//

				BodyByNonce::<T>::insert( index.clone(), hash.clone() );
				BodyState::<T>::insert( hash.clone(), state );
				BodyAccess::<T>::insert( hash.clone(), access.clone() );
				BodyCreator::<T>::insert( hash.clone(), sender.clone() );
				BodyController::<T>::insert( hash.clone(), controller.clone() );
				BodyTreasury::<T>::insert( hash.clone(), treasury.clone() );

				let mut controlled = Self::by_controller(&controller);
				controlled.push(hash.clone());
				ControlledBodies::<T>::mutate(
					&controller,
					|controlled| controlled.push(hash.clone())
				);

				// TODO: this needs a separate add / removal function
				// whenever the controller of an organisation changes!
				ControlledBodiesCount::<T>::mutate(
					&controller,
					|controlled_count| *controlled_count += 1
				);


				let mut created = Self::by_creator(sender.clone());
				created.push(hash.clone());
				ControlledBodies::<T>::mutate(
					&sender,
					|created| created.push(hash.clone())
				);

				// initiate member registry -> consumes fees
				// creator and controller can be equal
				// controller and treasury cannot be equal
				// match Self::add( &hash, creator.clone() ) {
				// 		Ok(_) => {},
				// 		Err(err) => { panic!("{err}") }
				// };
				match Self::add( hash.clone(), controller.clone() ) {
						Ok(_) => {},
						Err(err) => { panic!("{err}") }
				};
				// match Self::add( hash.clone(), treasury.clone() ) {
				// 		Ok(_) => {},
				// 		Err(err) => { panic!("{err}") }
				// };

				// generate nft realm

				// get the current realm index
				let current_realm_index = tangram::NextRealmIndex::get();

				// every org receives a token realm by default
				let realm = tangram::Module::<T>::create_realm(origin.clone(), hash.clone());
				let realm = match realm {
						Ok(_) => {},
						Err(err) => { return Err(err) }
				};

				// get current class index
				let current_class_index = tangram::NextClassIndex::get(current_realm_index);

				// generate a class name
				let name:Vec<u8> = b"game".to_vec();
				// every org receives a token class for collectables by default
				let max = 1000; // TODO: externalise max
				let class = tangram::Module::<T>::create_class(
					origin.clone(),
					current_realm_index.clone(),
					name,
					max,
					// mint,
					// burn,
					strategy
				);
				let class = match class {
						Ok(_) => {},
						Err(err) => { return Err(err) }
				};

				// get the next realm index...
				let next_realm_index = tangram::NextRealmIndex::get();

				// mint an item for creator
				// let item_name:Vec<u8> = b"creator".to_vec();
				// let item_cid:Vec<u8> = b"0".to_vec();

				// let item = tangram::Module::<T>::create_item(
				// 	origin.clone(),
				// 	current_realm_index,
				// 	current_class_index,
				// 	item_name,
				// 	item_cid,
				// 	creator.clone()
				// );
				// let item = match item {
				// 		Ok(_) => {},
				// 		Err(err) => { return Err(err) }
				// };

				// mint an item for controller
				// let ctrl_item_name:Vec<u8> = b"controller".to_vec();
				// let ctrl_item_cid:Vec<u8> = b"1".to_vec();

				// let ctrl_item = tangram::Module::<T>::create_item(
				// 	origin.clone(),
				// 	current_realm_index,
				// 	current_class_index,
				// 	ctrl_item_name,
				// 	ctrl_item_cid,
				// 	controller.clone()
				// );
				// let ctrl_item = match ctrl_item {
				// 		Ok(_) => {},
				// 		Err(err) => { return Err(err) }
				// };

				// pay tribute
				// let balance = <balances::Module<T>>::free_balance(&sender);
				// let dao_fee = _fee.checked_mul(0.25);

				let transfer = <balances::Module<T> as Currency<_>>::transfer(
					&sender,
					&T::GameDAOTreasury::get(),
					creation_fee,
					ExistenceRequirement::AllowDeath
				);

				// nonce
				Nonce::mutate(|n| *n += 1);

				// dispatch event
				Self::deposit_event(
					RawEvent::BodyCreated(sender, hash, now, next_realm_index)
				);
				Ok(())

			}

			// Add Member to Body
			#[weight = 1_000_000]
			fn add_member(
				origin,
				hash: T::Hash,
				account: T::AccountId
			) -> DispatchResult {

				let caller = ensure_signed(origin)?;
				// TODO: ensure not a member yet
				let add = Self::add( hash.clone(), account.clone() );
				let add = match add {
						Ok(_) => {},
						Err(err) => { return Err(err) }
				};
				Ok(())

			}

			// Remove Member from Body
			#[weight = 1_000_000]
			fn remove_member(
				origin,
				hash: T::Hash,
				account: T::AccountId,
			) -> DispatchResult {

				// TODO:
				// when fees==1 unreserve fees
				let caller = ensure_signed(origin)?;
				let remove = Self::remove( hash.clone(), account.clone());
				let remove = match remove {
						Ok(_) => {},
						Err(err) => { return Err(err) }
				};
				Ok(())

			}

			// Update State of Body
			// #[weight = 5_000]
			// fn update_state(
			// 	origin,
			// 	hash: T::Hash,
			// 	state: u8
			// ) {

			// 	let sender = ensure_root(origin)?;
			// 	Self::set_state(hash.clone(), state.clone());

			// }

			#[weight = 1_000_000]
			fn check_membership(
				origin,
				hash: T::Hash
			) -> DispatchResult {

				let caller = ensure_signed(origin)?;
				let members = BodyMembers::<T>::get(hash);
				ensure!(members.contains(&caller), Error::<T>::MemberUnknown);
				Self::deposit_event(
					RawEvent::IsAMember(hash,caller)
				);
				Ok(())

			}

		// /// Set controller. Must be a current member.
		// ///
		// /// May only be called from `T::PrimeOrigin`.
		// #[weight = 50_000_000]
		// pub fn set_control(origin, who: T::AccountId) {
		// 	T::PrimeOrigin::ensure_origin(origin)?;
		// 	Self::members().binary_search(&who).ok().ok_or(Error::<T, I>::NotMember)?;
		// 	Prime::<T, I>::put(&who);
		// 	T::MembershipChanged::set_prime(Some(who));
		// }

		// /// Remove the prime member if it exists.
		// ///
		// /// May only be called from `T::PrimeOrigin`.
		// #[weight = 50_000_000]
		// pub fn clear_control(origin) {
		// 	T::PrimeOrigin::ensure_origin(origin)?;
		// 	Prime::<T, I>::kill();
		// 	T::MembershipChanged::set_prime(None);
		// }


		}

	}

	impl<T: Config> Module<T> {

		// fn set_state(
		// 	hash: T::Hash,
		// 	state: u8
		// ) -> DispatchResult {

		// 	<BodyState<T>>::insert( hash.clone(), state.clone() );

		// 	let now = <system::Module<T>>::block_number();
		// 	Self::deposit_event( RawEvent::BodyUpdated(hash, state, now ) );
		// 	Ok(())

		// }

		fn set_member_state(
			hash: T::Hash,
			account: T::AccountId,
			member_state: ControlMemberState
		) -> DispatchResult {

			ensure!( <Bodies<T>>::contains_key(&hash), Error::<T>::BodyUnknown );
			let config = Self::body_config(hash);
			let current_state = Self::body_member_state(( &hash, &account ));
			let new_state = match config.access {
				ControlAccessModel::Open => member_state, // when open use desired state
				_ => ControlMemberState::Pending, // else pending
			};

			Ok(())

		}

		fn add(
			hash: T::Hash,
			account: T::AccountId
		) -> DispatchResult {

			// 1. body exists

			ensure!( <Bodies<T>>::contains_key(&hash), Error::<T>::BodyUnknown );

			// 2. member limit

			let mut members = BodyMembers::<T>::get(hash);
			let max_members = T::MaxMembersPerBody::get();
			ensure!(members.len() < max_members, Error::<T>::MembershipLimitReached);

			// 3. initial state

			let config = Self::body_config(hash);
			let state = match config.access {
				ControlAccessModel::Open => ControlMemberState::Active, // active
				_ => ControlMemberState::Pending, // pending
 			};

 			// 4. apply fees

			let fee = config.fee;
			ensure!( <balances::Module<T>>::free_balance(account.clone()) >= fee, Error::<T>::BalanceTooLow );

 			match &config.fee_model {

 				// no fees
 				ControlFeeModel::NoFees => {
 				},
 				// reserve
 				ControlFeeModel::Reserve => {
					<balances::Module<T>>::reserve(&account, config.fee)?;
 				},
 				// transfer to treasury
 				ControlFeeModel::Transfer => {
					let treasury = BodyTreasury::<T>::get(hash);
					let transfer = <balances::Module<T> as Currency<_>>::transfer(
						&account,
						&treasury,
						config.fee,
						ExistenceRequirement::AllowDeath
					);
 				}
 				_ => {}
 			}

			// 5. add

			match members.binary_search(&account) {

				// already a member, return
				Ok(_) => Err(Error::<T>::MemberExists.into()),

				// not a member, insert at index
				Err(index) => {

					members.insert(index, account.clone());
					// BodyMembers::<T>::mutate( &hash, |members| members.push(account.clone()) );
					BodyMembers::<T>::insert( &hash, members.clone() );

					// counter
					let count = members.len();
					BodyMemberCount::<T>::insert( &hash, count as u64 );

					let mut memberships = Self::memberships(&account);
					memberships.push( hash.clone() );
					Memberships::<T>::insert( &account, memberships );

					// state
					BodyMemberState::<T>::insert(( hash.clone(), account.clone() ), state);

					let now = <system::Module<T>>::block_number();
					Self::deposit_event(
						RawEvent::AddMember(hash,account,now)
					);
					Ok(())
				}

			}

		}

		fn remove(
			hash: T::Hash,
			account: T::AccountId,
		) -> DispatchResult {

			// existence
			ensure!( <Bodies<T>>::contains_key(&hash), Error::<T>::BodyUnknown );

			let mut members = BodyMembers::<T>::get(hash);
			match members.binary_search(&account) {

				Ok(index) => {

					// remove member from body
					members.remove(index);
					BodyMembers::<T>::insert(&hash,members.clone());

					// remove body from member's bodies
					let mut memberships = Self::memberships(&account);
					match memberships.binary_search(&hash) {
						Ok(index) => {
							memberships.remove(index);
							Memberships::<T>::insert( &account, memberships );
						},
						Err(_) => {},
					}

					// unreserve?
					let config = Self::body_config(hash);
		 			if config.fee_model == ControlFeeModel::Reserve {
						<balances::Module<T>>::unreserve( &account, config.fee );
	 				}

					// counter --
					let count = members.len();
					BodyMemberCount::<T>::insert( &hash, count as u64 );

					// member state
					BodyMemberState::<T>::insert(( hash.clone(), account.clone() ), ControlMemberState::Inactive);

					let now = <system::Module<T>>::block_number();
					Self::deposit_event(
						RawEvent::RemoveMember(hash,account,now)
					);
					Ok(())
				},

				Err(_) => Err(Error::<T>::MemberUnknown.into()),

			}

		}

		// transfer control of a body
		fn transfer(
			hash: T::Hash,
			account: T::AccountId
		) {

		}

	}

	//
	//
	//

	decl_event! {
		pub enum Event<T> where
			<T as system::Config>::AccountId,
			<T as system::Config>::BlockNumber,
			<T as system::Config>::Hash,
		{
			Message(Vec<u8>),
			BodyCreated( AccountId, Hash, BlockNumber, u64),
			BodyUpdated( AccountId, Hash, BlockNumber),
			BodyDisabled( Hash ),
			BodyTransferred( AccountId, Hash, BlockNumber),
			AddMember( Hash, AccountId, BlockNumber),
			RemoveMember( Hash, AccountId, BlockNumber),
			UpdateMember( Hash, AccountId, BlockNumber),
			IsAMember( Hash, AccountId),
			RealmCreated(),
			ClassCreated(),
		}
	}

	//
	//
	//

	decl_error! {
		pub enum Error for Module<T: Config> {
			/// Body Exists
			BodyExists,
			/// Body Unknown
			BodyUnknown,
			/// Body Inactive
			BodyInactive,
			/// Guru Meditation
			GuruMeditation,
			/// Insufficient Balance to create Body
			BalanceTooLow,
			/// Member Add Overflow
			MemberAddOverflow,
			/// Membership Limit Reached
			MembershipLimitReached,
			/// Member Exists
			MemberExists,
			/// Member Unknonw
			MemberUnknown,
			/// Unknown Error
			UnknownError,
			/// Duplicate Address
			DuplicateAddress,
		}
	}

}
