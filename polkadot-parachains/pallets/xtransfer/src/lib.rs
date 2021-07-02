// Copyright 2020-2021 Parity Technologies (UK) Ltd.
// This file is part of Cumulus.

// Cumulus is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Cumulus is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Cumulus.  If not, see <http://www.gnu.org/licenses/>.

//! Pallet to spam the XCM/UMP.

#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::prelude::*;
use sp_runtime::traits::Saturating;
use frame_system::Config as SystemConfig;
use frame_system::ensure_signed;
use cumulus_primitives_core::ParaId;
use cumulus_pallet_xcm::{Origin as CumulusOrigin, ensure_sibling_para};
use xcm::v0::{prelude::*, Xcm, Error as XcmError, SendXcm, OriginKind, MultiLocation, Junction};
use xcm_executor::traits::Convert;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use xcm_executor::traits::WeightBounds;
	use super::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	/// The module configuration trait.
	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_assets::Config {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// XCM executor.
		type XcmExecutor: ExecuteXcm<Self::Call>;

		/// Required origin for executing XCM messages, including the teleport functionality. If successful,
		/// then it resolves to `MultiLocation` which exists as an interior location within this chain's XCM
		/// context.
		type ExecuteXcmOrigin: EnsureOrigin<Self::Origin, Success=MultiLocation>;

		/// Means of measuring the weight consumed by an XCM message locally.
		type Weigher: WeightBounds<Self::Call>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	#[pallet::metadata(T::BlockNumber = "BlockNumber")]
	pub enum Event<T: Config> {
		Attempted(xcm::v0::Outcome),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The message's weight could not be determined.
		UnweighableMessage,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> { }

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(0)]
		pub fn transfer(
			origin: OriginFor<T>,
			dest: MultiLocation,
			beneficiary: MultiLocation,
			assets: MultiAsset,
			dest_weight: Weight
		) -> DispatchResult {
			let origin_location = T::ExecuteXcmOrigin::ensure_origin(origin)?;

			// let buy_order = Order::BuyExecution {
			// 	fees: All,
			// 	// Zero weight for additional XCM (since there are none to execute)
			// 	weight: 0,
			// 	debt: dest_weight,
			// 	halt_on_error: false,
			// 	xcm: vec![],
			// };

			let mut message = Xcm::WithdrawAsset {
				assets: vec![assets],
				effects: vec![DepositReserveAsset {
					assets: vec![MultiAsset::All],
					dest,
					effects: vec![
						// buy_order, 
						DepositAsset {
							assets: vec![MultiAsset::All],
							dest: beneficiary,
						}
					],
				}],
			};
			let weight = T::Weigher::weight(&mut message).map_err(|()| Error::<T>::UnweighableMessage)?;

			let outcome = T::XcmExecutor::execute_xcm_in_credit(origin_location, message, weight, weight);
			
			Self::deposit_event(Event::<T>::Attempted(outcome));
			Ok(())
			
		}
	}
}