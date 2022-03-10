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

//! SENSE
//!
//! This pallet aggregates datapoints to reflect user experience and behaviour.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

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
		traits::{ EnsureOrigin },
	};
	use frame_system::{ self as system, ensure_root };
	use codec::{ Encode, Decode };
	use sp_runtime::{ ModuleId };
	use sp_std::prelude::*;
	use pallet_balances::{ self as balances };

	//
	//
	//

	pub const MODULE_ID: ModuleId = ModuleId(*b"sensenet");
	pub const MODULE_VERSION: &str = "1.0";

	//
	//
	//

	pub trait Config: system::Config + balances::Config {
		// TODO: decide module name int or ext
		// type ModuleId: Get<ModuleId>;
		/// the goode olde event
		type Event: From<Event<Self>> + Into<<Self as system::Config>::Event>;
		///
		type ForceOrigin: EnsureOrigin<Self::Origin>;
		// TODO:
		// Weight information for extrinsics in this module.
		// type WeightInfo: WeightInfo;
	}

	//
	//
	//

	/// SenseNet Entity
	#[derive(Encode, Decode, Default, PartialEq, Eq)]
	#[cfg_attr(feature = "std", derive(Debug))]
	pub struct Entity<AccountId, BlockNumber> {
		account: AccountId,
		index:   u128,
		cid:     Vec<u8>,
		created: BlockNumber,
		mutated: BlockNumber,
	}

	/// SenseNet Entity Property
	#[derive(Encode, Decode, Default, PartialEq, Eq)]
	#[cfg_attr(feature = "std", derive(Debug))]
	pub struct EntityProperty<BlockNumber> {
		value:   u64,
		mutated: BlockNumber,
	}

	//
	//
	//

	decl_storage! {
		trait Store for Module<T: Config> as Sense48 {
			/// A Sense Entity
			Sense get(fn entity): map hasher(blake2_128_concat) T::AccountId => Entity<T::AccountId,T::BlockNumber>;
			/// XP across the metaverse
			SenseXP get(fn xp): map hasher(blake2_128_concat) T::AccountId => EntityProperty<T::BlockNumber>;
			/// REP across the metaverse
			SenseREP get(fn rep): map hasher(blake2_128_concat) T::AccountId => EntityProperty<T::BlockNumber>;
			/// Trust score across the metaverse
			SenseTrust get(fn trust): map hasher(blake2_128_concat) T::AccountId => EntityProperty<T::BlockNumber>;

			// propose judgements account, value, deposit
			// if judgement successful, value will be applied and deposit returned
			// SenseOpenJudgements get(fn judgements): map hasher(blake2_128_concat) => Vec<Judgement<T::AccountId, u64, T::Balance>>;

			/// the goode olde nonce
			Nonce: u128;
		}
	}

	//
	//
	//

	decl_event! {
		pub enum Event<T> where
			<T as system::Config>::AccountId,
			<T as system::Config>::BlockNumber,
		{
			EntityInit(AccountId, BlockNumber),
			EntityMutateXP(AccountId, BlockNumber),
			EntityMutateREP(AccountId, BlockNumber),
			EntityMutateTrust(AccountId, BlockNumber),
		}
	}

	//
	//
	//

	decl_error! {
		pub enum Error for Module<T: Config> {
			/// Entity Exists
			EntityExists,
			/// Entity Unknown
			EntityUnknown,
			/// Guru Meditation
			GuruMeditation,
		}
	}

	//
	//
	//

	decl_module! {
		pub struct Module<T: Config> for enum Call where origin: T::Origin {

			fn deposit_event() = default;
			type Error = Error<T>;

			// Create Entity
			#[weight = 1_000_000]
			fn create_entity(
				origin,
				account: T::AccountId,
				cid:     Vec<u8>,
			) -> DispatchResult {

				ensure_root(origin)?;
				ensure!( !<Sense<T>>::contains_key(&account), Error::<T>::EntityExists );

				let now   = <system::Module<T>>::block_number();
				let index = Nonce::get();
				let data  = Entity {
					account:  account.clone(),
					cid:      cid,
					index:    index.clone(),
					created:  now.clone(),
					mutated:  now.clone(),
				};

				let xp    = EntityProperty { value: 0, mutated: now.clone() };
				let rep   = EntityProperty { value: 0, mutated: now.clone() };
				let trust = EntityProperty { value: 0, mutated: now.clone() };

				<SenseXP<T>>::insert( account.clone(), xp );
				<SenseREP<T>>::insert( account.clone(), rep );
				<SenseTrust<T>>::insert( account.clone(), trust );
				<Sense<T>>::insert( account.clone(), data );

				Nonce::mutate(|n| *n += 1);

				Self::deposit_event(
					RawEvent::EntityInit(account, now)
				);
				Ok(())

			}

			// TODO:
			// mutation of values should be restricted
			// certain roles are allowed to mutate values
			// xp:    realm
			// rep:   social
			// trust: id
			// all:   governance
			//        sudo ( until its removal )

			#[weight = 1_000_000]
			fn mod_xp(
				origin,
				account: T::AccountId,
				value:   u8, // allow for a maximum of u8 to change
			) -> DispatchResult {

				ensure_root(origin)?;
				ensure!( <Sense<T>>::contains_key(&account), Error::<T>::EntityUnknown );

				let now = <system::Module<T>>::block_number();

				let v = u64::from(value);
				let current = Self::xp(&account);
				let updated = EntityProperty {
					value: current.value.checked_add(v).ok_or(Error::<T>::GuruMeditation)?,
					mutated: now
				};

				<SenseXP<T>>::insert( account.clone(), updated );

				Self::deposit_event(
					RawEvent::EntityMutateXP(account, now)
				);
				Ok(())

			}

			#[weight = 1_000_000]
			fn mod_rep(
				origin,
				account: T::AccountId,
				value:   u8, // allow for a maximum of u8 to change
			) -> DispatchResult {

				ensure_root(origin)?;
				ensure!( <Sense<T>>::contains_key(&account), Error::<T>::EntityUnknown );

				let now = <system::Module<T>>::block_number();
				let v = u64::from(value);
				let current = Self::rep(&account);
				let updated = EntityProperty {
					value: current.value.checked_add(v).ok_or(Error::<T>::GuruMeditation)?,
					mutated: now
				};

				<SenseREP<T>>::insert( account.clone(), updated );

				Self::deposit_event(
					RawEvent::EntityMutateREP(account, now)
				);
				Ok(())

			}

			#[weight = 1_000_000]
			fn mod_trust(
				origin,
				account: T::AccountId,
				value:   u8, // allow for a maximum of u8 to change
			) -> DispatchResult {

				ensure_root(origin)?;
				ensure!( <Sense<T>>::contains_key(&account), Error::<T>::EntityUnknown );

				let now = <system::Module<T>>::block_number();
				let v = u64::from(value);
				let current = Self::trust(&account);
				let updated = EntityProperty {
					value: current.value.checked_add(v).ok_or(Error::<T>::GuruMeditation)?,
					mutated: now
				};

				<SenseTrust<T>>::insert( account.clone(), updated );

				Self::deposit_event(
					RawEvent::EntityMutateTrust(account, now)
				);
				Ok(())

			}

			// TODO:
			// generic mod for all properties

			// #[weight = 1_000]
			// fn mod(
			// 	origin,
			// 	account: T::AccountId,
			// 	property: SenseProps,
			// 	value:   u8, // allow for a maximum of u8 to change
			// ) -> DispatchResult {

			// 	let creator = ensure_signed(origin)?;
			// 	let now = <system::Module<T>>::block_number();

			// 	let v = u64::from(v);
			// 	let current_xp = Self::xp(&account);
			// 	let updated_xp = current_xp.checked_add(v).ok_or(Error::<T>::GuruMeditation)?;
			// 	<SenseXP<T>>::insert( account.clone(), updated_xp );

			// 	Self::deposit_event(
			// 		RawEvent::EntityMutate(account, now)
			// 	);
			// 	Ok(())

			// }

		}
	}

}
