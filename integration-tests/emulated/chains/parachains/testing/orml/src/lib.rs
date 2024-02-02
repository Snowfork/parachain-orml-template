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

mod genesis;
pub use genesis::{genesis, ED, PARA_ID};
pub use parachain_orml_template_runtime::xcm_config::XcmConfig;

// Substrate
use frame_support::traits::OnInitialize;

// Cumulus
use emulated_integration_tests_common::{
	impl_accounts_helpers_for_parachain, impl_assert_events_helpers_for_parachain,
	impl_xcm_helpers_for_parachain, impls::Parachain, xcm_emulator::decl_test_parachains,
};

// Penpal Parachain declaration
decl_test_parachains! {
	pub struct OrmlTemplate {
		genesis = genesis(PARA_ID),
		on_init = {
			parachain_orml_template_runtime::AuraExt::on_initialize(1);
		},
		runtime = parachain_orml_template_runtime,
		core = {
			XcmpMessageHandler: parachain_orml_template_runtime::XcmpQueue,
			LocationToAccountId: parachain_orml_template_runtime::xcm_config::LocationToAccountId,
			ParachainInfo: parachain_orml_template_runtime::ParachainInfo,
			MessageOrigin: cumulus_primitives_core::AggregateMessageOrigin,
		},
		pallets = {
			PolkadotXcm: parachain_orml_template_runtime::PolkadotXcm,
			Balances: parachain_orml_template_runtime::Balances,
			AssetRegistry: parachain_orml_template_runtime::AssetRegistry,
			Currencies: parachain_orml_template_runtime::Currencies,
		}
	},
}

// Penpal implementation
impl_accounts_helpers_for_parachain!(OrmlTemplate);
impl_assert_events_helpers_for_parachain!(OrmlTemplate);
impl_xcm_helpers_for_parachain!(OrmlTemplate);
