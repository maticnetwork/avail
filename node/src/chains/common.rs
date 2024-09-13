use super::{get_account_id_from_seed, AuthorityKeys};
use avail_core::{BLOCK_CHUNK_SIZE, DA_DISPATCH_RATIO_PCT};
use kate::config::{MAX_BLOCK_COLUMNS, MAX_BLOCK_ROWS};

use da_runtime::{
	constants, AccountId, Balance, DataAvailabilityConfig, SessionKeys, StakerStatus,
};
use frame_system::limits::BlockLength;
use pallet_vector::constants::{
	get_poseidon_hash_for_period, BROADCASTER, BROADCASTER_DOMAIN, FINALITY_THRESHOLD,
	GENESIS_TIME, GENESIS_VALIDATOR_ROOT, PERIOD, ROTATE_FUNCTION_ID, ROTATE_VK, SECONDS_PER_SLOT,
	SLOTS_PER_PERIOD, SOURCE_CHAIN_ID, STEP_FUNCTION_ID, STEP_VK,
};
use sc_telemetry::TelemetryEndpoints;
use serde_json::{json, Value};
use sp_core::crypto::AccountId32;
use sp_core::sr25519::Public;
use sp_runtime::Perbill;

pub const PROTOCOL_ID: &str = "Avail";
pub const TESTNET_TELEMETRY_URL: &str = "ws://telemetry.avail.tools:8001/submit";
// pub const TELEMETRY_URL: &str = "wss://telemetry.avail.so:8001/submit";

const DEFAULT_ENDOWED_SEEDS: [&str; 12] = [
	"Alice",
	"Bob",
	"Charlie",
	"Dave",
	"Eve",
	"Ferdie",
	"Alice//stash",
	"Bob//stash",
	"Charlie//stash",
	"Dave//stash",
	"Eve//stash",
	"Ferdie//stash",
];
const INIT_APP_IDS: [(u32, &str); 10] = [
	(0, "Avail"),
	(1, "Reserved-1"),
	(2, "Reserved-2"),
	(3, "Reserved-3"),
	(4, "Reserved-4"),
	(5, "Reserved-5"),
	(6, "Reserved-6"),
	(7, "Reserved-7"),
	(8, "Reserved-8"),
	(9, "Reserved-9"),
];

pub fn to_telemetry_endpoint(s: String) -> TelemetryEndpoints {
	TelemetryEndpoints::new(vec![(s, 0)]).unwrap()
}

/// Generates a default endowed accounts.
fn dev_endowed_accounts() -> Vec<(AccountId, Balance)> {
	DEFAULT_ENDOWED_SEEDS
		.iter()
		.map(|seed| {
			(
				get_account_id_from_seed::<Public>(seed),
				constants::staking::MIN_VALIDATOR_BOND * 100,
			)
		})
		.collect()
}

fn make_data_avail_config(owner: AccountId) -> DataAvailabilityConfig {
	let app_keys = INIT_APP_IDS
		.iter()
		.map(|(id, app)| (app.as_bytes().to_vec(), (owner.clone(), *id)))
		.collect();

	DataAvailabilityConfig { app_keys }
}

pub fn runtime_genesis_config(
	sudo: AccountId32,
	technical_committee: Vec<AccountId32>,
	treasury_committee: Vec<AccountId32>,
	session_keys: Vec<AuthorityKeys>,
) -> Value {
	let balances = dev_endowed_accounts();
	let stakers: Vec<(AccountId, AccountId, Balance, StakerStatus<AccountId>)> = session_keys
		.iter()
		.map(|k| {
			(
				k.stash.clone(),
				k.controller.clone(),
				constants::staking::MIN_VALIDATOR_BOND,
				StakerStatus::Validator,
			)
		})
		.collect();
	let validator_count = session_keys.len() as u32;
	let session_keys: Vec<(AccountId, AccountId, SessionKeys)> =
		session_keys.into_iter().map(|k| k.into()).collect();
	let block_length = BlockLength::with_normal_ratio(
		MAX_BLOCK_ROWS,
		MAX_BLOCK_COLUMNS,
		BLOCK_CHUNK_SIZE,
		Perbill::from_percent(DA_DISPATCH_RATIO_PCT as u32),
	)
	.expect("Valid `BlockLength` genesis definition .qed");

	json!({
		"system": {
			"blockLength": block_length,
		},
		"balances": {
			"balances": balances,
		},
		"staking": {
			"validatorCount": validator_count,
			"minimumValidatorCount": 1,
			"stakers": stakers,
			"minNominatorBond": constants::staking::MIN_NOMINATOR_BOND,
			"minValidatorBond": constants::staking::MIN_VALIDATOR_BOND,
		},
		"babe": {
			"epochConfig": Some(da_runtime::constants::babe::GENESIS_EPOCH_CONFIG),
		},
		"session": {
			"keys": session_keys,
		},
		"sudo": {
			"key": Some(sudo.clone()),
		},
		"technicalCommittee": {
			"members": technical_committee,
		},
		"treasuryCommittee": {
			"members": treasury_committee,
		},
		"vector": {
			"broadcaster": BROADCASTER,
			"broadcasterDomain": BROADCASTER_DOMAIN,
			"finalityThreshold": FINALITY_THRESHOLD,
			"functionIds": (STEP_FUNCTION_ID, ROTATE_FUNCTION_ID),
			"genesisTime": GENESIS_TIME,
			"genesisValidatorRoot": GENESIS_VALIDATOR_ROOT,
			"period": PERIOD,
			"secondsPerSlot": SECONDS_PER_SLOT,
			"slotsPerPeriod": SLOTS_PER_PERIOD,
			"sourceChainId": SOURCE_CHAIN_ID,
			"syncCommitteePoseidon":get_poseidon_hash_for_period(),
			"stepVerificationKey": STEP_VK.as_bytes().to_vec(),
			"rotateVerificationKey": ROTATE_VK.as_bytes().to_vec(),
			"whitelistedDomains": vec![2],
		},
		"nominationPools": {
			"minCreateBond": constants::nomination_pools::MIN_CREATE_BOND,
			"minJoinBond": constants::nomination_pools::MIN_JOIN_BOND,
			"maxPools": Some(constants::nomination_pools::MAX_POOLS),
			"maxMembersPerPool": Some(constants::nomination_pools::MAX_MEMBERS_PER_POOL),
			"maxMembers": Some(constants::nomination_pools::MAX_MEMBERS),
		},
		"dataAvailability": {
			"appKeys": make_data_avail_config(sudo).app_keys
		},
	})
}
