use sp_std::{prelude::*, convert::{TryInto, TryFrom}, result, marker::PhantomData, borrow::Borrow};
use xcm::v0::{Error as XcmError, MultiAsset, MultiLocation, Junction};
use frame_support::{ensure, traits::{Get, tokens::fungibles, Contains}};
use xcm_executor::traits::{Convert, MatchesFungibles, Error as MatchError};
use xcm_executor::traits::FilterAssetLocation;

pub struct SimpleAssetIdConverter<AssetId>(PhantomData<AssetId>);
impl<AssetId: Clone + TryFrom<u128>> Convert<u128, AssetId> for SimpleAssetIdConverter<AssetId> {
	fn convert_ref(id: impl Borrow<u128>) -> Result<AssetId, ()>{
		(*id.borrow()).try_into().map_err(|_e| ())
	}
}


// asset id conversion
pub struct AsPrefixedGeneralIndex<Prefix, AssetId, ConvertAssetId>(PhantomData<(Prefix, AssetId, ConvertAssetId)>);
impl<
	Prefix: Get<MultiLocation>,
	AssetId: Clone,
	ConvertAssetId: Convert<u128, AssetId>,
> Convert<MultiLocation, AssetId> for AsPrefixedGeneralIndex<Prefix, AssetId, ConvertAssetId> {
	fn convert_ref(id: impl Borrow<MultiLocation>) -> result::Result<AssetId, ()> {
		let prefix = Prefix::get();
		let id = id.borrow();

		if id.is_interior() {
			if !prefix.iter().enumerate().all(|(index, item)| id.at(index) == Some(item)) {
				return Err(())
			}
			match id.at(prefix.len()) {
				Some(Junction::GeneralIndex { id }) => return ConvertAssetId::convert_ref(id),
				_ => return Err(()),
			}
		} else {
			if let MultiLocation::X4(Junction::Parent, Junction::Parachain(para_id),
			prefix, Junction::GeneralIndex{id: index}) = id {
				ensure!(MultiLocation::from(prefix.clone()) == Prefix::get(), ());
				let index_length: u32 = 128 - index.leading_zeros();
				let asset_id: u128 = (para_id.clone() as u128).checked_shl(index_length).unwrap_or(0) + (index.clone() as u128);
				return ConvertAssetId::convert_ref(asset_id);
			} else {
				return Err(())
			}
		}

		
	}


	fn reverse_ref(what: impl Borrow<AssetId>) -> result::Result<MultiLocation, ()> {
		let mut location = Prefix::get();
		let id = ConvertAssetId::reverse_ref(what)?;
		location.push(Junction::GeneralIndex { id }).map_err(|_| ())?;
		Ok(location)
	}
}



pub struct SimpleBalanceConverter<Balance>(PhantomData<Balance>);
impl<Balance: Clone + TryFrom<u128>> Convert<u128, Balance> for SimpleBalanceConverter<Balance> {
	fn convert_ref(amount: impl Borrow<u128>) -> Result<Balance, ()> {
		Balance::try_from(*amount.borrow()).map_err(|_e| ())
	}
}



pub struct ConvertedConcreteAssetId<AssetId, Balance, ConvertAssetId, ConvertBalance>(
	PhantomData<(AssetId, Balance, ConvertAssetId, ConvertBalance)>
);
impl<
	AssetId: Clone,
	Balance: Clone,
	ConvertAssetId: Convert<MultiLocation, AssetId>,
	ConvertBalance: Convert<u128, Balance>,
> MatchesFungibles<AssetId, Balance> for
	ConvertedConcreteAssetId<AssetId, Balance, ConvertAssetId, ConvertBalance>
{
	fn matches_fungibles(a: &MultiAsset) -> result::Result<(AssetId, Balance), MatchError> {
		let (id, amount) = match a {
			MultiAsset::ConcreteFungible { id, amount } => (id, amount),
			_ => return Err(MatchError::AssetNotFound),
		};

		let what = ConvertAssetId::convert_ref(id).map_err(|_| MatchError::AssetIdConversionFailed)?;
		let amount = ConvertBalance::convert_ref(amount).map_err(|_| MatchError::AmountToBalanceConversionFailed)?;
		Ok((what, amount))
	}
}

pub struct TrustedReserve;

impl FilterAssetLocation for TrustedReserve {
	fn filter_asset_location(_asset: &MultiAsset, _origin: &MultiLocation) -> bool {
		true
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use frame_support::parameter_types;
	use xcm::v0::{Junction::*, MultiAsset, MultiLocation::*};
	use core::convert::From;
	
	#[test]
	fn match_fungibles_should_work() {

		parameter_types! {
			pub const Prefix: MultiLocation = MultiLocation::X1(Junction::PalletInstance(50));
		}

		type InternalAssetConverter = super::AsPrefixedGeneralIndex<Prefix, u32, super::SimpleAssetIdConverter<u32>>;
		type Balance = u128;
		type AssetConverter = super::ConvertedConcreteAssetId<u32, Balance, InternalAssetConverter, super::SimpleBalanceConverter<Balance>>;

		let sender_chain = MultiLocation::X4(Junction::Parent, Junction::Parachain(2010),  Junction::PalletInstance(50), Junction::GeneralIndex{id: 2010});
		let asset = MultiAsset::ConcreteFungible{id: sender_chain, amount: 200};
		let maybe_asset_id = AssetConverter::matches_fungibles(&asset).unwrap_or_else(|_e| Default::default());
		println!("maybe asset id is {:?}", maybe_asset_id);
	}
}