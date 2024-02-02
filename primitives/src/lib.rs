// This file is part of HydraDX-node.

// Copyright (C) 2020-2021  Intergalactic, Limited (GIB).
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![cfg_attr(not(feature = "std"), no_std)]

/// Opaque, encoded, unchecked extrinsic.
pub use frame_support::sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

use sp_std::vec::Vec;
pub trait Registry<AssetId, AssetName, Balance, Error> {
	fn exists(name: AssetId) -> bool;

	fn retrieve_asset(name: &AssetName) -> Result<AssetId, Error>;

	fn retrieve_asset_type(asset_id: AssetId) -> Result<AssetKind, Error>;

	fn create_asset(name: &AssetName, existential_deposit: Balance) -> Result<AssetId, Error>;

	fn get_or_create_asset(
		name: AssetName,
		existential_deposit: Balance,
	) -> Result<AssetId, Error> {
		if let Ok(asset_id) = Self::retrieve_asset(&name) {
			Ok(asset_id)
		} else {
			Self::create_asset(&name, existential_deposit)
		}
	}
}

pub trait InspectRegistry<AssetId> {
	fn exists(asset_id: AssetId) -> bool;
	fn decimals(asset_id: AssetId) -> Option<u8>;
	fn asset_name(asset_id: AssetId) -> Option<Vec<u8>>;
	fn asset_symbol(asset_id: AssetId) -> Option<Vec<u8>>;
}

#[derive(Eq, PartialEq, Copy, Clone)]
pub enum AssetKind {
	Token,
	XYK,
	StableSwap,
	Bond,
}

pub trait CreateRegistry<AssetId, Balance> {
	type Error;
	fn create_asset(
		name: &[u8],
		kind: AssetKind,
		existential_deposit: Balance,
	) -> Result<AssetId, Self::Error>;
}

/// Abstraction over account id and account name creation for `Assets`
pub trait AccountIdFor<Assets> {
	type AccountId;

	/// Create account id for given assets and an identifier
	fn from_assets(assets: &Assets, identifier: Option<&[u8]>) -> Self::AccountId;

	/// Create a name to uniquely identify a share token for given assets and an identifier.
	fn name(assets: &Assets, identifier: Option<&[u8]>) -> Vec<u8>;
}

/// Type for storing the id of an asset.
pub type AssetId = u32;

/// Signed version of Balance
pub type Amount = i128;

pub const ROC: AssetId = 1;
pub const WETH: AssetId = 1000;
