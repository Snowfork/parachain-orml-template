// Copyright (C) Parity Technologies (UK) Ltd.
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

// Substrate
use sp_core::{sr25519, storage::Storage};

// Cumulus
use emulated_integration_tests_common::{
	accounts,
	accounts::{ALICE, BOB},
	build_genesis_storage, collators, get_account_id_from_seed, SAFE_XCM_VERSION,
};
use parachains_common::Balance;
use primitives::{ROC, WETH};

// Penpal
pub const PARA_ID: u32 = 3000;
pub const ED: Balance = parachain_orml_template_runtime::EXISTENTIAL_DEPOSIT;

pub fn genesis(para_id: u32) -> Storage {
	let genesis_config = parachain_orml_template_runtime::RuntimeGenesisConfig {
		system: parachain_orml_template_runtime::SystemConfig::default(),
		balances: parachain_orml_template_runtime::BalancesConfig {
			balances: accounts::init_balances().iter().cloned().map(|k| (k, ED * 4096)).collect(),
		},
		parachain_info: parachain_orml_template_runtime::ParachainInfoConfig {
			parachain_id: para_id.into(),
			..Default::default()
		},
		collator_selection: parachain_orml_template_runtime::CollatorSelectionConfig {
			invulnerables: collators::invulnerables().iter().cloned().map(|(acc, _)| acc).collect(),
			candidacy_bond: ED * 16,
			..Default::default()
		},
		session: parachain_orml_template_runtime::SessionConfig {
			keys: collators::invulnerables()
				.into_iter()
				.map(|(acc, aura)| {
					(
						acc.clone(),                                           // account id
						acc,                                                   // validator id
						parachain_orml_template_runtime::SessionKeys { aura }, // session keys
					)
				})
				.collect(),
		},
		polkadot_xcm: parachain_orml_template_runtime::PolkadotXcmConfig {
			safe_xcm_version: Some(SAFE_XCM_VERSION),
			..Default::default()
		},
		sudo: parachain_orml_template_runtime::SudoConfig {
			key: Some(get_account_id_from_seed::<sr25519::Public>("Alice")),
		},
		asset_registry: parachain_orml_template_runtime::AssetRegistryConfig {
			registered_assets: vec![
				(b"WETH".to_vec(), 1_000u128, Some(WETH)),
				(b"ROC".to_vec(), 1_000u128, Some(ROC)),
			],
			native_asset_name: b"ORML".to_vec(),
			native_existential_deposit: parachain_orml_template_runtime::EXISTENTIAL_DEPOSIT,
		},
		tokens: parachain_orml_template_runtime::TokensConfig {
			balances: vec![
				(get_account_id_from_seed::<sr25519::Public>(ALICE), ROC, 1_000_000_000_000),
				(get_account_id_from_seed::<sr25519::Public>(BOB), ROC, 1_000_000_000_000),
			],
		},
		..Default::default()
	};

	build_genesis_storage(
		&genesis_config,
		parachain_orml_template_runtime::WASM_BINARY
			.expect("WASM binary was not built, please build it!"),
	)
}
