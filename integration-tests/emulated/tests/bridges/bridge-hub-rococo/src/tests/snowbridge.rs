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
use crate::*;
use bridge_hub_rococo_runtime::{EthereumBeaconClient, EthereumInboundQueue, RuntimeOrigin};
use codec::{Decode, Encode};
use emulated_integration_tests_common::xcm_emulator::ConvertLocation;
use frame_support::pallet_prelude::TypeInfo;
use hex_literal::hex;
use orml_traits::MultiCurrency;
use parachains_common::rococo::snowbridge::EthereumNetwork;
use rococo_system_emulated_network::{
	BridgeHubRococoParaSender as BridgeHubRococoSender, OrmlTemplatePara,
};
use snowbridge_core::outbound::OperatingMode;
use snowbridge_pallet_inbound_queue_fixtures::{
	register_token::make_register_token_message,
	register_token_with_insufficient_fee::make_register_token_with_infufficient_fee_message,
	send_token::make_send_token_message, send_token_to_penpal::make_send_token_to_penpal_message,
	InboundQueueFixture,
};
use snowbridge_pallet_system;
use snowbridge_router_primitives::inbound::{
	Command, Destination, GlobalConsensusEthereumConvertsFor, MessageV1, VersionedMessage,
};
use sp_core::H256;
use sp_runtime::{ArithmeticError::Underflow, DispatchError::Arithmetic};

const INITIAL_FUND: u128 = 5_000_000_000 * ROCOCO_ED;
const CHAIN_ID: u64 = 11155111;
const TREASURY_ACCOUNT: [u8; 32] =
	hex!("6d6f646c70792f74727372790000000000000000000000000000000000000000");
const WETH: [u8; 20] = hex!("87d1f7fdfEe7f651FaBc8bFCB6E086C278b77A7d");
const ETHEREUM_DESTINATION_ADDRESS: [u8; 20] = hex!("44a57ee2f2FCcb85FDa2B0B18EBD0D8D2333700e");
const XCM_FEE: u128 = 4_000_000_000;

#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone, TypeInfo)]
pub enum ControlCall {
	#[codec(index = 3)]
	CreateAgent,
	#[codec(index = 4)]
	CreateChannel { mode: OperatingMode },
}

#[allow(clippy::large_enum_variant)]
#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone, TypeInfo)]
pub enum SnowbridgeControl {
	#[codec(index = 83)]
	Control(ControlCall),
}

pub fn send_inbound_message(fixture: InboundQueueFixture) -> DispatchResult {
	EthereumBeaconClient::store_execution_header(
		fixture.message.proof.block_hash,
		fixture.execution_header,
		0,
		H256::default(),
	);

	EthereumInboundQueue::submit(
		RuntimeOrigin::signed(BridgeHubRococoSender::get()),
		fixture.message,
	)
}

/// Create an agent on Ethereum. An agent is a representation of an entity in the Polkadot
/// ecosystem (like a parachain) on Ethereum.
#[test]
#[ignore]
fn create_agent() {
	let origin_para: u32 = 1001;
	// Fund the origin parachain sovereign account so that it can pay execution fees.
	BridgeHubRococo::fund_para_sovereign(origin_para.into(), INITIAL_FUND);

	let sudo_origin = <Rococo as Chain>::RuntimeOrigin::root();
	let destination = Rococo::child_location_of(BridgeHubRococo::para_id()).into();

	let create_agent_call = SnowbridgeControl::Control(ControlCall::CreateAgent {});
	// Construct XCM to create an agent for para 1001
	let remote_xcm = VersionedXcm::from(Xcm(vec![
		UnpaidExecution { weight_limit: Unlimited, check_origin: None },
		DescendOrigin(Parachain(origin_para).into()),
		Transact {
			require_weight_at_most: 3000000000.into(),
			origin_kind: OriginKind::Xcm,
			call: create_agent_call.encode().into(),
		},
	]));

	// Rococo Global Consensus
	// Send XCM message from Relay Chain to Bridge Hub source Parachain
	Rococo::execute_with(|| {
		assert_ok!(<Rococo as RococoPallet>::XcmPallet::send(
			sudo_origin,
			bx!(destination),
			bx!(remote_xcm),
		));

		type RuntimeEvent = <Rococo as Chain>::RuntimeEvent;
		// Check that the Transact message was sent
		assert_expected_events!(
			Rococo,
			vec![
				RuntimeEvent::XcmPallet(pallet_xcm::Event::Sent { .. }) => {},
			]
		);
	});

	BridgeHubRococo::execute_with(|| {
		type RuntimeEvent = <BridgeHubRococo as Chain>::RuntimeEvent;
		// Check that a message was sent to Ethereum to create the agent
		assert_expected_events!(
			BridgeHubRococo,
			vec![
				RuntimeEvent::EthereumSystem(snowbridge_pallet_system::Event::CreateAgent {
					..
				}) => {},
			]
		);
	});
}

/// Create a channel for a consensus system. A channel is a bidirectional messaging channel
/// between BridgeHub and Ethereum.
#[test]
#[ignore]
fn create_channel() {
	let origin_para: u32 = 1001;
	// Fund AssetHub sovereign account so that it can pay execution fees.
	BridgeHubRococo::fund_para_sovereign(origin_para.into(), INITIAL_FUND);

	let sudo_origin = <Rococo as Chain>::RuntimeOrigin::root();
	let destination: VersionedLocation =
		Rococo::child_location_of(BridgeHubRococo::para_id()).into();

	let create_agent_call = SnowbridgeControl::Control(ControlCall::CreateAgent {});
	// Construct XCM to create an agent for para 1001
	let create_agent_xcm = VersionedXcm::from(Xcm(vec![
		UnpaidExecution { weight_limit: Unlimited, check_origin: None },
		DescendOrigin(Parachain(origin_para).into()),
		Transact {
			require_weight_at_most: 3000000000.into(),
			origin_kind: OriginKind::Xcm,
			call: create_agent_call.encode().into(),
		},
	]));

	let create_channel_call =
		SnowbridgeControl::Control(ControlCall::CreateChannel { mode: OperatingMode::Normal });
	// Construct XCM to create a channel for para 1001
	let create_channel_xcm = VersionedXcm::from(Xcm(vec![
		UnpaidExecution { weight_limit: Unlimited, check_origin: None },
		DescendOrigin(Parachain(origin_para).into()),
		Transact {
			require_weight_at_most: 3000000000.into(),
			origin_kind: OriginKind::Xcm,
			call: create_channel_call.encode().into(),
		},
	]));

	// Rococo Global Consensus
	// Send XCM message from Relay Chain to Bridge Hub source Parachain
	Rococo::execute_with(|| {
		assert_ok!(<Rococo as RococoPallet>::XcmPallet::send(
			sudo_origin.clone(),
			bx!(destination.clone()),
			bx!(create_agent_xcm),
		));

		assert_ok!(<Rococo as RococoPallet>::XcmPallet::send(
			sudo_origin,
			bx!(destination),
			bx!(create_channel_xcm),
		));

		type RuntimeEvent = <Rococo as Chain>::RuntimeEvent;

		assert_expected_events!(
			Rococo,
			vec![
				RuntimeEvent::XcmPallet(pallet_xcm::Event::Sent { .. }) => {},
			]
		);
	});

	BridgeHubRococo::execute_with(|| {
		type RuntimeEvent = <BridgeHubRococo as Chain>::RuntimeEvent;

		// Check that the Channel was created
		assert_expected_events!(
			BridgeHubRococo,
			vec![
				RuntimeEvent::EthereumSystem(snowbridge_pallet_system::Event::CreateChannel {
					..
				}) => {},
			]
		);
	});
}

/// Tests the registering of a token as an asset on AssetHub.
#[test]
fn register_weth_token_from_ethereum_to_asset_hub() {
	// Fund AssetHub sovereign account so that it can pay execution fees.
	BridgeHubRococo::fund_para_sovereign(AssetHubRococo::para_id().into(), INITIAL_FUND);

	BridgeHubRococo::execute_with(|| {
		type RuntimeEvent = <BridgeHubRococo as Chain>::RuntimeEvent;

		// Construct RegisterToken message and sent to inbound queue
		let register_token_message = make_register_token_message();
		send_inbound_message(register_token_message.clone()).unwrap();

		assert_expected_events!(
			BridgeHubRococo,
			vec![
				RuntimeEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::XcmpMessageSent { .. }) => {},
			]
		);
	});

	AssetHubRococo::execute_with(|| {
		type RuntimeEvent = <AssetHubRococo as Chain>::RuntimeEvent;

		assert_expected_events!(
			AssetHubRococo,
			vec![
				RuntimeEvent::ForeignAssets(pallet_assets::Event::Created { .. }) => {},
			]
		);
	});
}

/// Tests sending a token to a 3rd party parachain, called PenPal. The token reserve is
/// still located on AssetHub.
#[test]
fn send_token_from_ethereum_to_penpal() {
	let asset_hub_sovereign = BridgeHubRococo::sovereign_account_id_of(Location::new(
		1,
		[Parachain(AssetHubRococo::para_id().into())],
	));
	// Fund AssetHub sovereign account so it can pay execution fees for the asset transfer
	BridgeHubRococo::fund_accounts(vec![(asset_hub_sovereign.clone(), INITIAL_FUND)]);

	// Fund PenPal sender and receiver
	PenpalA::fund_accounts(vec![
		(PenpalAReceiver::get(), INITIAL_FUND),
		(PenpalASender::get(), INITIAL_FUND),
	]);

	// The Weth asset location, identified by the contract address on Ethereum
	let weth_asset_location: Location =
		(Parent, Parent, EthereumNetwork::get(), AccountKey20 { network: None, key: WETH }).into();
	// Converts the Weth asset location into an asset ID
	let weth_asset_id: v3::Location = weth_asset_location.try_into().unwrap();

	let origin_location = (Parent, Parent, EthereumNetwork::get()).into();

	// Fund ethereum sovereign on AssetHub
	let ethereum_sovereign: AccountId =
		GlobalConsensusEthereumConvertsFor::<AccountId>::convert_location(&origin_location)
			.unwrap();
	AssetHubRococo::fund_accounts(vec![(ethereum_sovereign.clone(), INITIAL_FUND)]);

	// Create asset on the Penpal parachain.
	PenpalA::execute_with(|| {
		assert_ok!(<PenpalA as PenpalAPallet>::ForeignAssets::create(
			<PenpalA as Chain>::RuntimeOrigin::signed(PenpalASender::get()),
			weth_asset_id,
			asset_hub_sovereign.into(),
			1000,
		));

		assert!(<PenpalA as PenpalAPallet>::ForeignAssets::asset_exists(weth_asset_id));
	});

	BridgeHubRococo::execute_with(|| {
		type RuntimeEvent = <BridgeHubRococo as Chain>::RuntimeEvent;

		// Construct RegisterToken message and sent to inbound queue
		send_inbound_message(make_register_token_message()).unwrap();

		// Construct SendToken message and sent to inbound queue
		send_inbound_message(make_send_token_to_penpal_message()).unwrap();

		assert_expected_events!(
			BridgeHubRococo,
			vec![
				RuntimeEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::XcmpMessageSent { .. }) => {},
			]
		);
	});

	AssetHubRococo::execute_with(|| {
		type RuntimeEvent = <AssetHubRococo as Chain>::RuntimeEvent;
		// Check that the assets were issued on AssetHub
		assert_expected_events!(
			AssetHubRococo,
			vec![
				RuntimeEvent::ForeignAssets(pallet_assets::Event::Issued { .. }) => {},
				RuntimeEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::XcmpMessageSent { .. }) => {},
			]
		);
	});

	PenpalA::execute_with(|| {
		type RuntimeEvent = <PenpalA as Chain>::RuntimeEvent;
		// Check that the assets were issued on PenPal
		assert_expected_events!(
			PenpalA,
			vec![
				RuntimeEvent::ForeignAssets(pallet_assets::Event::Issued { .. }) => {},
			]
		);
	});
}

/// Tests the registering of a token as an asset on AssetHub, and then subsequently sending
/// a token from Ethereum to AssetHub.
#[test]
fn send_token_from_ethereum_to_asset_hub() {
	BridgeHubRococo::fund_para_sovereign(AssetHubRococo::para_id().into(), INITIAL_FUND);

	// Fund ethereum sovereign on AssetHub
	AssetHubRococo::fund_accounts(vec![(AssetHubRococoReceiver::get(), INITIAL_FUND)]);

	BridgeHubRococo::execute_with(|| {
		type RuntimeEvent = <BridgeHubRococo as Chain>::RuntimeEvent;

		// Construct RegisterToken message and sent to inbound queue
		send_inbound_message(make_register_token_message()).unwrap();

		// Construct SendToken message and sent to inbound queue
		send_inbound_message(make_send_token_message()).unwrap();

		// Check that the message was sent
		assert_expected_events!(
			BridgeHubRococo,
			vec![
				RuntimeEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::XcmpMessageSent { .. }) => {},
			]
		);
	});

	AssetHubRococo::execute_with(|| {
		type RuntimeEvent = <AssetHubRococo as Chain>::RuntimeEvent;

		// Check that the token was received and issued as a foreign asset on AssetHub
		assert_expected_events!(
			AssetHubRococo,
			vec![
				RuntimeEvent::ForeignAssets(pallet_assets::Event::Issued { .. }) => {},
			]
		);
	});
}

/// Tests the full cycle of token transfers:
/// - registering a token on AssetHub
/// - sending a token to AssetHub
/// - returning the token to Ethereum
#[test]
fn send_weth_asset_from_asset_hub_to_ethereum() {
	use asset_hub_rococo_runtime::xcm_config::bridging::to_ethereum::DefaultBridgeHubEthereumBaseFee;
	let assethub_sovereign = BridgeHubRococo::sovereign_account_id_of(Location::new(
		1,
		[Parachain(AssetHubRococo::para_id().into())],
	));

	AssetHubRococo::force_default_xcm_version(Some(XCM_VERSION));
	BridgeHubRococo::force_default_xcm_version(Some(XCM_VERSION));
	AssetHubRococo::force_xcm_version(
		Location::new(2, [GlobalConsensus(Ethereum { chain_id: CHAIN_ID })]),
		XCM_VERSION,
	);

	BridgeHubRococo::fund_accounts(vec![(assethub_sovereign.clone(), INITIAL_FUND)]);
	AssetHubRococo::fund_accounts(vec![(AssetHubRococoReceiver::get(), INITIAL_FUND)]);

	const WETH_AMOUNT: u128 = 1_000_000_000;

	BridgeHubRococo::execute_with(|| {
		type RuntimeEvent = <BridgeHubRococo as Chain>::RuntimeEvent;

		// Construct RegisterToken message and sent to inbound queue
		send_inbound_message(make_register_token_message()).unwrap();

		// Check that the register token message was sent using xcm
		assert_expected_events!(
			BridgeHubRococo,
			vec![
				RuntimeEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::XcmpMessageSent { .. }) => {},
			]
		);

		// Construct SendToken message and sent to inbound queue
		send_inbound_message(make_send_token_message()).unwrap();

		// Check that the send token message was sent using xcm
		assert_expected_events!(
			BridgeHubRococo,
			vec![
				RuntimeEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::XcmpMessageSent { .. }) => {},
			]
		);
	});

	AssetHubRococo::execute_with(|| {
		type RuntimeEvent = <AssetHubRococo as Chain>::RuntimeEvent;
		type RuntimeOrigin = <AssetHubRococo as Chain>::RuntimeOrigin;

		// Check that AssetHub has issued the foreign asset
		assert_expected_events!(
			AssetHubRococo,
			vec![
				RuntimeEvent::ForeignAssets(pallet_assets::Event::Issued { .. }) => {},
			]
		);
		let assets = vec![Asset {
			id: AssetId(Location::new(
				2,
				[
					GlobalConsensus(Ethereum { chain_id: CHAIN_ID }),
					AccountKey20 { network: None, key: WETH },
				],
			)),
			fun: Fungible(WETH_AMOUNT),
		}];
		let multi_assets = VersionedAssets::V4(Assets::from(assets));

		let destination = VersionedLocation::V4(Location::new(
			2,
			[GlobalConsensus(Ethereum { chain_id: CHAIN_ID })],
		));

		let beneficiary = VersionedLocation::V4(Location::new(
			0,
			[AccountKey20 { network: None, key: ETHEREUM_DESTINATION_ADDRESS.into() }],
		));

		let free_balance_before = <AssetHubRococo as AssetHubRococoPallet>::Balances::free_balance(
			AssetHubRococoReceiver::get(),
		);
		// Send the Weth back to Ethereum
		<AssetHubRococo as AssetHubRococoPallet>::PolkadotXcm::reserve_transfer_assets(
			RuntimeOrigin::signed(AssetHubRococoReceiver::get()),
			Box::new(destination),
			Box::new(beneficiary),
			Box::new(multi_assets),
			0,
		)
		.unwrap();
		let free_balance_after = <AssetHubRococo as AssetHubRococoPallet>::Balances::free_balance(
			AssetHubRococoReceiver::get(),
		);
		// Assert at least DefaultBridgeHubEthereumBaseFee charged from the sender
		let free_balance_diff = free_balance_before - free_balance_after;
		assert!(free_balance_diff > DefaultBridgeHubEthereumBaseFee::get());
	});

	BridgeHubRococo::execute_with(|| {
		type RuntimeEvent = <BridgeHubRococo as Chain>::RuntimeEvent;
		// Check that the transfer token back to Ethereum message was queue in the Ethereum
		// Outbound Queue
		assert_expected_events!(
			BridgeHubRococo,
			vec![
				RuntimeEvent::EthereumOutboundQueue(snowbridge_pallet_outbound_queue::Event::MessageQueued {..}) => {},
			]
		);
		let events = BridgeHubRococo::events();
		// Check that the local fee was credited to the Snowbridge sovereign account
		assert!(
			events.iter().any(|event| matches!(
				event,
				RuntimeEvent::Balances(pallet_balances::Event::Deposit{ who, amount })
					if *who == TREASURY_ACCOUNT.into() && *amount == 16903333
			)),
			"Snowbridge sovereign takes local fee."
		);
		// Check that the remote fee was credited to the AssetHub sovereign account
		assert!(
			events.iter().any(|event| matches!(
				event,
				RuntimeEvent::Balances(pallet_balances::Event::Deposit{ who, amount })
					if *who == assethub_sovereign && *amount == 2680000000000,
			)),
			"AssetHub sovereign takes remote fee."
		);
	});
}

#[test]
fn register_weth_token_in_asset_hub_fail_for_insufficient_fee() {
	BridgeHubRococo::fund_para_sovereign(AssetHubRococo::para_id().into(), INITIAL_FUND);

	BridgeHubRococo::execute_with(|| {
		type RuntimeEvent = <BridgeHubRococo as Chain>::RuntimeEvent;

		// Construct RegisterToken message and sent to inbound queue
		let message = make_register_token_with_infufficient_fee_message();
		send_inbound_message(message).unwrap();

		assert_expected_events!(
			BridgeHubRococo,
			vec![
				RuntimeEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::XcmpMessageSent { .. }) => {},
			]
		);
	});

	AssetHubRococo::execute_with(|| {
		type RuntimeEvent = <AssetHubRococo as Chain>::RuntimeEvent;

		assert_expected_events!(
			AssetHubRococo,
			vec![
				RuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success:false, .. }) => {},
			]
		);
	});
}

#[test]
fn send_token_from_ethereum_to_asset_hub_fail_for_insufficient_fund() {
	// Insufficient fund
	BridgeHubRococo::fund_para_sovereign(AssetHubRococo::para_id().into(), 1_000);

	BridgeHubRococo::execute_with(|| {
		assert_err!(send_inbound_message(make_register_token_message()), Arithmetic(Underflow));
	});
}

#[test]
fn send_token_from_ethereum_to_orml_chain() {
	AssetHubRococo::force_default_xcm_version(Some(XCM_VERSION));
	BridgeHubRococo::force_default_xcm_version(Some(XCM_VERSION));
	OrmlTemplatePara::force_default_xcm_version(Some(XCM_VERSION));
	AssetHubRococo::force_xcm_version(
		Location::new(2, [GlobalConsensus(Ethereum { chain_id: CHAIN_ID })]),
		XCM_VERSION,
	);
	let asset_hub_sovereign = BridgeHubRococo::sovereign_account_id_of(Location::new(
		1,
		[Parachain(AssetHubRococo::para_id().into())],
	));
	// Fund AssetHub sovereign account so it can pay execution fees for the asset transfer
	BridgeHubRococo::fund_accounts(vec![(asset_hub_sovereign.clone(), INITIAL_FUND)]);

	// Fund sender and receiver on OrmlTemplate chain
	OrmlTemplatePara::fund_accounts(vec![
		(OrmlReceiver::get(), INITIAL_FUND),
		(OrmlSender::get(), INITIAL_FUND),
	]);

	// The Weth asset location, identified by the contract address on Ethereum
	let weth_asset_location: Location =
		(Parent, Parent, EthereumNetwork::get(), AccountKey20 { network: None, key: WETH }).into();

	let origin_location = (Parent, Parent, EthereumNetwork::get()).into();

	// Fund ethereum sovereign on AssetHub
	let ethereum_sovereign: AccountId =
		GlobalConsensusEthereumConvertsFor::<AccountId>::convert_location(&origin_location)
			.unwrap();
	// Fund ORML sovereign on AssetHub
	let orml_sovereign_on_asset_hub = AssetHubRococo::sovereign_account_id_of(Location::new(
		1,
		[Parachain(OrmlTemplatePara::para_id().into())],
	));
	AssetHubRococo::fund_accounts(vec![
		(ethereum_sovereign.clone(), INITIAL_FUND),
		(orml_sovereign_on_asset_hub.clone(), INITIAL_FUND),
	]);

	// Register WETH and ROC(as fee asset) on OrmlTemplate chain.
	OrmlTemplatePara::execute_with(|| {
		use parachain_orml_template_runtime::{AssetRegistry, RuntimeOrigin};
		use primitives::{ROC, WETH};
		assert_ok!(AssetRegistry::set_location(
			RuntimeOrigin::root(),
			WETH,
			parachain_orml_template_runtime::AssetLocation(weth_asset_location),
		));
		assert_ok!(AssetRegistry::set_location(
			RuntimeOrigin::root(),
			ROC,
			parachain_orml_template_runtime::AssetLocation(Parent.into()),
		));
	});

	const WETH_AMOUNT: u128 = 1_000_000_000;

	BridgeHubRococo::execute_with(|| {
		type RuntimeEvent = <BridgeHubRococo as Chain>::RuntimeEvent;

		// Construct RegisterToken message and sent to inbound queue
		send_inbound_message(make_register_token_message()).unwrap();

		// Construct SendToken message and sent to inbound queue
		let message_id: H256 = [1; 32].into();
		let message = VersionedMessage::V1(MessageV1 {
			chain_id: CHAIN_ID,
			command: Command::SendToken {
				token: WETH.into(),
				destination: Destination::ForeignAccountId32 {
					para_id: OrmlParaId,
					id: OrmlReceiver::get().into(),
					fee: XCM_FEE,
				},
				amount: WETH_AMOUNT,
				fee: XCM_FEE,
			},
		});
		// Convert the message to XCM
		let (xcm, _) = EthereumInboundQueue::do_convert(message_id, message).unwrap();
		// Send the XCM
		let _ = EthereumInboundQueue::send_xcm(xcm, AssetHubRococo::para_id().into()).unwrap();

		assert_expected_events!(
			BridgeHubRococo,
			vec![
				RuntimeEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::XcmpMessageSent { .. }) => {},
			]
		);
	});

	AssetHubRococo::execute_with(|| {
		type RuntimeEvent = <AssetHubRococo as Chain>::RuntimeEvent;
		// Check that the assets were issued on AssetHub
		assert_expected_events!(
			AssetHubRococo,
			vec![
				RuntimeEvent::ForeignAssets(pallet_assets::Event::Issued { .. }) => {},
				RuntimeEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::XcmpMessageSent { .. }) => {},
			]
		);
	});

	OrmlTemplatePara::execute_with(|| {
		type RuntimeEvent = <OrmlTemplatePara as Chain>::RuntimeEvent;
		type RuntimeOrigin = <OrmlTemplatePara as Chain>::RuntimeOrigin;
		// Check that the assets were issued on OrmlTemplate
		assert_expected_events!(
			OrmlTemplatePara,
			vec![
				RuntimeEvent::Currencies(pallet_currencies::Event::Deposited { currency_id: primitives::WETH,.. }) => {},
			]
		);
		let free_balance = <OrmlTemplatePara as OrmlPallet>::Currencies::free_balance(
			primitives::WETH,
			&OrmlReceiver::get(),
		);
		assert_eq!(free_balance, WETH_AMOUNT);
		// Send the Weth back to AssetHub, with ROC as fee asset
		let fee_asset = Asset { id: AssetId(Parent.into()), fun: Fungible(XCM_FEE) };
		let weth_asset = Asset {
			id: AssetId(Location::new(
				2,
				[
					GlobalConsensus(Ethereum { chain_id: CHAIN_ID }),
					AccountKey20 { network: None, key: WETH },
				],
			)),
			fun: Fungible(WETH_AMOUNT),
		};
		let assets = vec![fee_asset, weth_asset];
		let multi_assets = VersionedAssets::V4(Assets::from(assets));

		let destination = VersionedLocation::V4(Location::new(1, [Parachain(AssetHubParaId)]));

		let beneficiary = VersionedLocation::V4(Location::new(
			0,
			[AccountId32 { network: None, id: AssetHubRococoReceiver::get().into() }],
		));
		assert_ok!(<OrmlTemplatePara as OrmlPallet>::PolkadotXcm::reserve_transfer_assets(
			RuntimeOrigin::signed(OrmlReceiver::get()),
			Box::new(destination),
			Box::new(beneficiary),
			Box::new(multi_assets),
			0,
		));
		assert_expected_events!(
			OrmlTemplatePara,
			vec![
				RuntimeEvent::Currencies(pallet_currencies::Event::Withdrawn { currency_id:primitives::WETH,.. }) => {},
			]
		);
		let free_balance_after = <OrmlTemplatePara as OrmlPallet>::Currencies::free_balance(
			primitives::WETH,
			&OrmlReceiver::get(),
		);
		assert_eq!(free_balance_after, 0);
	});

	AssetHubRococo::execute_with(|| {
		type RuntimeEvent = <AssetHubRococo as Chain>::RuntimeEvent;
		type RuntimeOrigin = <AssetHubRococo as Chain>::RuntimeOrigin;
		// Check that weth was burned from orml-sovereign account on AssetHub
		// and issued into the destination account
		assert_expected_events!(
			AssetHubRococo,
			vec![
				RuntimeEvent::ForeignAssets(pallet_assets::Event::Burned { owner ,.. }) => {
					owner: *owner == orml_sovereign_on_asset_hub,
				},
				RuntimeEvent::ForeignAssets(pallet_assets::Event::Issued { owner ,.. }) => {
					owner: *owner == AssetHubRococoReceiver::get(),
				},
			]
		);
		// Send the WETH back from AssetHub to Ethereum
		let assets = vec![Asset {
			id: AssetId(Location::new(
				2,
				[
					GlobalConsensus(Ethereum { chain_id: CHAIN_ID }),
					AccountKey20 { network: None, key: WETH },
				],
			)),
			fun: Fungible(WETH_AMOUNT),
		}];
		let multi_assets = VersionedAssets::V4(Assets::from(assets));
		let destination = VersionedLocation::V4(Location::new(
			2,
			[GlobalConsensus(Ethereum { chain_id: CHAIN_ID })],
		));
		let beneficiary = VersionedLocation::V4(Location::new(
			0,
			[AccountKey20 { network: None, key: ETHEREUM_DESTINATION_ADDRESS.into() }],
		));
		<AssetHubRococo as AssetHubRococoPallet>::PolkadotXcm::reserve_transfer_assets(
			RuntimeOrigin::signed(AssetHubRococoReceiver::get()),
			Box::new(destination),
			Box::new(beneficiary),
			Box::new(multi_assets),
			0,
		)
		.unwrap();
	});

	BridgeHubRococo::execute_with(|| {
		type RuntimeEvent = <BridgeHubRococo as Chain>::RuntimeEvent;
		// Check that the transfer token back to Ethereum message was queue in the Ethereum
		// Outbound Queue
		assert_expected_events!(
			BridgeHubRococo,
			vec![
				RuntimeEvent::EthereumOutboundQueue(snowbridge_pallet_outbound_queue::Event::MessageQueued {..}) => {},
			]
		);
	});
}
