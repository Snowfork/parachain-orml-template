//! Unit tests for xcm-support implementations.

#![cfg(test)]

use super::*;
use xcm::prelude::{Asset, Parent};

use orml_traits::{
	location::{AbsoluteReserveProvider, RelativeLocations},
	ConcreteFungibleAsset,
};
use xcm::v3::prelude::{Concrete, Fungible, Junction, Parachain, X1};

#[derive(Debug, PartialEq, Eq)]
pub enum TestCurrencyId {
	TokenA,
	TokenB,
	RelayChainToken,
}

pub struct CurrencyIdConvert;
impl Convert<Location, Option<TestCurrencyId>> for CurrencyIdConvert {
	fn convert(l: Location) -> Option<TestCurrencyId> {
		use TestCurrencyId::*;

		if l == Location::parent() {
			return Some(RelayChainToken);
		}
		if l == Location::try_from(MultiLocation::sibling_parachain_general_key(
			1,
			b"TokenA".to_vec().try_into().unwrap(),
		))
		.unwrap()
		{
			return Some(TokenA);
		}
		if l == Location::try_from(MultiLocation::sibling_parachain_general_key(
			2,
			b"TokenB".to_vec().try_into().unwrap(),
		))
		.unwrap()
		{
			return Some(TokenB);
		}
		None
	}
}

type MatchesCurrencyId = IsNativeConcrete<TestCurrencyId, CurrencyIdConvert>;

#[test]
fn is_native_concrete_matches_native_currencies() {
	assert_eq!(
		MatchesCurrencyId::matches_fungible(
			&Asset::try_from(MultiAsset::parent_asset(100)).unwrap()
		),
		Some(100),
	);

	assert_eq!(
		MatchesCurrencyId::matches_fungible(
			&Asset::try_from(MultiAsset::sibling_parachain_asset(
				1,
				b"TokenA".to_vec().try_into().unwrap(),
				100
			))
			.unwrap()
		),
		Some(100),
	);

	assert_eq!(
		MatchesCurrencyId::matches_fungible(
			&Asset::try_from(MultiAsset::sibling_parachain_asset(
				2,
				b"TokenB".to_vec().try_into().unwrap(),
				100
			))
			.unwrap()
		),
		Some(100),
	);
}

#[test]
fn is_native_concrete_does_not_matches_non_native_currencies() {
	assert!(<MatchesCurrencyId as MatchesFungible<u128>>::matches_fungible(
		&Asset::try_from(MultiAsset::sibling_parachain_asset(
			2,
			b"TokenC".to_vec().try_into().unwrap(),
			100
		))
		.unwrap()
	)
	.is_none());
	assert!(<MatchesCurrencyId as MatchesFungible<u128>>::matches_fungible(
		&Asset::try_from(MultiAsset::sibling_parachain_asset(
			1,
			b"TokenB".to_vec().try_into().unwrap(),
			100
		))
		.unwrap()
	)
	.is_none());
	let general_key = Junction::from(sp_runtime::BoundedVec::try_from(b"TokenB".to_vec()).unwrap());
	assert!(<MatchesCurrencyId as MatchesFungible<u128>>::matches_fungible(
		&Asset::try_from(MultiAsset {
			fun: Fungible(100),
			id: Concrete(MultiLocation::new(1, X1(general_key)))
		})
		.unwrap()
	)
	.is_none());
}

#[test]
fn multi_native_asset() {
	assert!(MultiNativeAsset::<AbsoluteReserveProvider>::contains(
		&Asset::try_from(MultiAsset { fun: Fungible(10), id: Concrete(MultiLocation::parent()) })
			.unwrap(),
		&Parent.into()
	));
	assert!(MultiNativeAsset::<AbsoluteReserveProvider>::contains(
		&Asset::try_from(MultiAsset::sibling_parachain_asset(
			1,
			b"TokenA".to_vec().try_into().unwrap(),
			100
		))
		.unwrap(),
		&Location::try_from(MultiLocation::new(1, X1(Parachain(1)))).unwrap(),
	));
	assert!(!MultiNativeAsset::<AbsoluteReserveProvider>::contains(
		&Asset::try_from(MultiAsset::sibling_parachain_asset(
			1,
			b"TokenA".to_vec().try_into().unwrap(),
			100
		))
		.unwrap(),
		&Parent.into(),
	));
}
