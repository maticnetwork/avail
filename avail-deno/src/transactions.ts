import type { ApiPromise } from "@polkadot/api";
import type { KeyringPair } from "@polkadot/keyring/types";
import type { ISubmittableResult } from "@polkadot/types/types/extrinsic";
import type { SignerOptions } from "@polkadot/api/submittable/types";
import type { EventRecord, H256 } from "@polkadot/types/interfaces/types";
import type { BN } from "@polkadot/util";
import { err, ok } from "@neverthrow";
import type { Result } from "@neverthrow";

import * as TransactionData from "./transaction_data.ts";
import * as Events from "./events.ts";
import { commissionNumberToPerbill, decodeError } from "./common.ts";

export type DispatchFeeModifier = { weightMaximumFee: BN | null; weightFeeDivider: number | null; weightFeeMultiplier: number | null };
export enum WaitFor {
	BlockInclusion,
	BlockFinalization,
}
export type StakingRewardDestination = "Staked" | "Stash" | "None" | { account: string };
type ValidatorPerfs = { commission: string; blocked: boolean };

type GenericFailure = { isErr: true; reason: string };

// DA Struct
type SubmitDataTxSuccess = {
	isErr: false;
	txData: TransactionData.DataAvailability.SubmitData;
	event: Events.DataAvailability.DataSubmittedEvent;
	events: EventRecord[];
	txHash: H256;
	txIndex: number;
	blockHash: H256;
	blockNumber: number;
};
type CreateApplicationKeyTxSuccess = {
	isErr: false;
	event: Events.DataAvailability.ApplicationKeyCreatedEvent;
	events: EventRecord[];
	txHash: H256;
	txIndex: number;
	blockHash: H256;
	blockNumber: number;
};
type SetApplicationKeyTxSuccess = {
	isErr: false;
	event: Events.DataAvailability.ApplicationKeySetEvent;
	events: EventRecord[];
	txHash: H256;
	txIndex: number;
	blockHash: H256;
	blockNumber: number;
};
type SubmitBlockLengthProposalTxSuccess = {
	isErr: false;
	event: Events.DataAvailability.BlockLengthProposalSubmittedEvent;
	events: EventRecord[];
	txHash: H256;
	txIndex: number;
	blockHash: H256;
	blockNumber: number;
};
type SetSubmitDataFeeModifierTxSuccess = {
	isErr: false;
	event: Events.DataAvailability.SubmitDataFeeModifierSetEvent;
	events: EventRecord[];
	txHash: H256;
	txIndex: number;
	blockHash: H256;
	blockNumber: number;
};

// Balances struct
type TransferKeepAliveTxSuccess = {
	isErr: false;
	event: Events.Balances.TransferEvent;
	events: EventRecord[];
	txHash: H256;
	txIndex: number;
	blockHash: H256;
	blockNumber: number;
};
type TransferAllowDeathTxSuccess = {
	isErr: false;
	event: Events.Balances.TransferEvent;
	event2?: Events.System.KilledAccount;
	events: EventRecord[];
	txHash: H256;
	txIndex: number;
	blockHash: H256;
	blockNumber: number;
};
type TransferAllTxSuccess = {
	isErr: false;
	event: Events.Balances.TransferEvent;
	event2?: Events.System.KilledAccount;
	events: EventRecord[];
	txHash: H256;
	txIndex: number;
	blockHash: H256;
	blockNumber: number;
};

// Staking struct
type BondTxSuccess = {
	isErr: false;
	event: Events.Staking.Bonded;
	events: EventRecord[];
	txHash: H256;
	txIndex: number;
	blockHash: H256;
	blockNumber: number;
};
type BondExtraTxSuccess = {
	isErr: false;
	event: Events.Staking.Bonded;
	events: EventRecord[];
	txHash: H256;
	txIndex: number;
	blockHash: H256;
	blockNumber: number;
};
type ChillTxSuccess = {
	isErr: false;
	event: Events.Staking.Chilled;
	events: EventRecord[];
	txHash: H256;
	txIndex: number;
	blockHash: H256;
	blockNumber: number;
};
type ChillOtherTxSuccess = {
	isErr: false;
	event: Events.Staking.Chilled;
	events: EventRecord[];
	txHash: H256;
	txIndex: number;
	blockHash: H256;
	blockNumber: number;
};
type UnbondTxSuccess = {
	isErr: false;
	event: Events.Staking.Unbonded;
	events: EventRecord[];
	txHash: H256;
	txIndex: number;
	blockHash: H256;
	blockNumber: number;
};
type ValidateTxSuccess = {
	isErr: false;
	event: Events.Staking.ValidatorPrefsSet;
	events: EventRecord[];
	txHash: H256;
	txIndex: number;
	blockHash: H256;
	blockNumber: number;
};
type NominateTxSuccess = {
	isErr: false;
	txData: TransactionData.Staking.Nominate;
	events: EventRecord[];
	txHash: H256;
	txIndex: number;
	blockHash: H256;
	blockNumber: number;
};

// Session struct
// todo

// Nomination Pools struct
type PoolCreateTxSuccess = {
	isErr: false;
	event: Events.NominationPools.Created;
	event2: Events.NominationPools.Bonded;
	events: EventRecord[];
	txHash: H256;
	txIndex: number;
	blockHash: H256;
	blockNumber: number;
};

type PoolCreateWithPoolIdTxSuccess = {
	isErr: false;
	event: Events.NominationPools.Created;
	event2: Events.NominationPools.Bonded;
	events: EventRecord[];
	txHash: H256;
	txIndex: number;
	blockHash: H256;
	blockNumber: number;
};

type PoolJoinTxSuccess = {
	isErr: false;
	event: Events.NominationPools.Bonded;
	events: EventRecord[];
	txHash: H256;
	txIndex: number;
	blockHash: H256;
	blockNumber: number;
};

type PoolNominateTxSuccess = {
	isErr: false;
	events: EventRecord[];
	txHash: H256;
	txIndex: number;
	blockHash: H256;
	blockNumber: number;
};

export class Transactions {
	private api: ApiPromise;
	dataAvailability: DataAvailability;
	balances: Balances;
	staking: Staking;
	nominationPools: NominationPools;

	constructor(api: ApiPromise) {
		this.api = api;
		this.dataAvailability = new DataAvailability(api);
		this.balances = new Balances(api);
		this.staking = new Staking(api);
		this.nominationPools = new NominationPools(api);
	}
}

function standardCallback(
	result_in: ISubmittableResult,
	result_out: (value: Result<ISubmittableResult, string>) => void,
	waitFor: WaitFor,
) {
	if (result_in.isError) {
		result_out(ok(result_in));
	}

	if (result_in.isInBlock && waitFor == WaitFor.BlockInclusion) {
		result_out(ok(result_in));
	}

	if (result_in.isFinalized) {
		result_out(ok(result_in));
	}
}

async function getBlockHashAndTxHash(result: ISubmittableResult, waitFor: WaitFor, api: ApiPromise): Promise<[H256, number, H256, number]> {
	const txHash = result.txHash as H256;
	const txIndex: number = result.txIndex || 22;
	let blockHash = txHash;
	if (waitFor == WaitFor.BlockFinalization) {
		blockHash = result.status.asFinalized as H256;
	} else {
		blockHash = result.status.asInBlock as H256;
	}

	const header = await api.rpc.chain.getHeader(blockHash);
	const blockNumber: number = header.number.toNumber();

	return [txHash, txIndex, blockHash, blockNumber];
}

export class NominationPools {
	private api: ApiPromise;

	constructor(api: ApiPromise) {
		this.api = api;
	}

	async create(
		amount: BN,
		root: string,
		nominator: string,
		bouncer: string,
		waitFor: WaitFor,
		account: KeyringPair,
		options?: Partial<SignerOptions>,
	): Promise<PoolCreateTxSuccess | GenericFailure> {
		const optionWrapper = options || {};
		const maybeTxResult = await new Promise<Result<ISubmittableResult, string>>((res, _) => {
			this.api.tx.nominationPools.create(amount, root, nominator, bouncer).signAndSend(
				account,
				optionWrapper,
				(result: ISubmittableResult) => {
					standardCallback(result, res, waitFor);
				},
			).catch((reason) => {
				res(err(reason));
			});
		});

		if (maybeTxResult.isErr()) {
			return { isErr: true, reason: maybeTxResult.error } as GenericFailure;
		}
		const txResult = maybeTxResult.value;

		if (txResult.isError) {
			return { isErr: true, reason: "The transaction was dropped or something." } as GenericFailure;
		}

		const events = txResult.events;
		const failed = events.find((e) => this.api.events.system.ExtrinsicFailed.is(e.event));
		if (failed != undefined) {
			return { isErr: true, reason: decodeError(this.api, failed.event.data[0]) } as GenericFailure;
		}

		const event = Events.NominationPools.Created.New(txResult.events);
		if (event == undefined) {
			return { isErr: true, reason: "Failed to find Created event." } as GenericFailure;
		}

		const event2 = Events.NominationPools.Bonded.New(txResult.events);
		if (event2 == undefined) {
			return { isErr: true, reason: "Failed to find Bonded event." } as GenericFailure;
		}

		const [txHash, txIndex, blockHash, blockNumber] = await getBlockHashAndTxHash(txResult, waitFor, this.api);

		return { isErr: false, event, event2, events, txHash, txIndex, blockHash, blockNumber } as PoolCreateTxSuccess;
	}

	async createWithPoolId(
		amount: BN,
		root: string,
		nominator: string,
		bouncer: string,
		poolId: number,
		waitFor: WaitFor,
		account: KeyringPair,
		options?: Partial<SignerOptions>,
	): Promise<PoolCreateWithPoolIdTxSuccess | GenericFailure> {
		const optionWrapper = options || {};
		const maybeTxResult = await new Promise<Result<ISubmittableResult, string>>((res, _) => {
			this.api.tx.nominationPools.createWithPoolId(amount, root, nominator, bouncer, poolId).signAndSend(
				account,
				optionWrapper,
				(result: ISubmittableResult) => {
					standardCallback(result, res, waitFor);
				},
			).catch((reason) => {
				res(err(reason));
			});
		});

		if (maybeTxResult.isErr()) {
			return { isErr: true, reason: maybeTxResult.error } as GenericFailure;
		}
		const txResult = maybeTxResult.value;

		if (txResult.isError) {
			return { isErr: true, reason: "The transaction was dropped or something." } as GenericFailure;
		}

		const events = txResult.events;
		const failed = events.find((e) => this.api.events.system.ExtrinsicFailed.is(e.event));
		if (failed != undefined) {
			return { isErr: true, reason: decodeError(this.api, failed.event.data[0]) } as GenericFailure;
		}

		const event = Events.NominationPools.Created.New(txResult.events);
		if (event == undefined) {
			return { isErr: true, reason: "Failed to find Created event." } as GenericFailure;
		}

		const event2 = Events.NominationPools.Bonded.New(txResult.events);
		if (event2 == undefined) {
			return { isErr: true, reason: "Failed to find Bonded event." } as GenericFailure;
		}

		const [txHash, txIndex, blockHash, blockNumber] = await getBlockHashAndTxHash(txResult, waitFor, this.api);

		return { isErr: false, event, event2, events, txHash, txIndex, blockHash, blockNumber } as PoolCreateWithPoolIdTxSuccess;
	}

	async join(
		amount: BN,
		poolId: number,
		waitFor: WaitFor,
		account: KeyringPair,
		options?: Partial<SignerOptions>,
	): Promise<PoolJoinTxSuccess | GenericFailure> {
		const optionWrapper = options || {};
		const maybeTxResult = await new Promise<Result<ISubmittableResult, string>>((res, _) => {
			this.api.tx.nominationPools.join(amount, poolId).signAndSend(account, optionWrapper, (result: ISubmittableResult) => {
				standardCallback(result, res, waitFor);
			}).catch((reason) => {
				res(err(reason));
			});
		});

		if (maybeTxResult.isErr()) {
			return { isErr: true, reason: maybeTxResult.error } as GenericFailure;
		}
		const txResult = maybeTxResult.value;

		if (txResult.isError) {
			return { isErr: true, reason: "The transaction was dropped or something." } as GenericFailure;
		}

		const events = txResult.events;
		const failed = events.find((e) => this.api.events.system.ExtrinsicFailed.is(e.event));
		if (failed != undefined) {
			return { isErr: true, reason: decodeError(this.api, failed.event.data[0]) } as GenericFailure;
		}

		const event = Events.NominationPools.Bonded.New(txResult.events);
		if (event == undefined) {
			return { isErr: true, reason: "Failed to find Bonded event." } as GenericFailure;
		}

		const [txHash, txIndex, blockHash, blockNumber] = await getBlockHashAndTxHash(txResult, waitFor, this.api);

		return { isErr: false, event, events, txHash, txIndex, blockHash, blockNumber } as PoolJoinTxSuccess;
	}

	async nominate(
		poolId: number,
		validators: string[],
		waitFor: WaitFor,
		account: KeyringPair,
		options?: Partial<SignerOptions>,
	): Promise<PoolNominateTxSuccess | GenericFailure> {
		const optionWrapper = options || {};
		const maybeTxResult = await new Promise<Result<ISubmittableResult, string>>((res, _) => {
			this.api.tx.nominationPools.nominate(poolId, validators).signAndSend(account, optionWrapper, (result: ISubmittableResult) => {
				standardCallback(result, res, waitFor);
			}).catch((reason) => {
				res(err(reason));
			});
		});

		if (maybeTxResult.isErr()) {
			return { isErr: true, reason: maybeTxResult.error } as GenericFailure;
		}
		const txResult = maybeTxResult.value;

		if (txResult.isError) {
			return { isErr: true, reason: "The transaction was dropped or something." } as GenericFailure;
		}

		const events = txResult.events;
		const failed = events.find((e) => this.api.events.system.ExtrinsicFailed.is(e.event));
		if (failed != undefined) {
			return { isErr: true, reason: decodeError(this.api, failed.event.data[0]) } as GenericFailure;
		}

		const [txHash, txIndex, blockHash, blockNumber] = await getBlockHashAndTxHash(txResult, waitFor, this.api);

		return { isErr: false, events, txHash, txIndex, blockHash, blockNumber } as PoolNominateTxSuccess;
	}
}

export class Staking {
	private api: ApiPromise;

	constructor(api: ApiPromise) {
		this.api = api;
	}

	async bond(
		value: BN,
		payee: StakingRewardDestination,
		waitFor: WaitFor,
		account: KeyringPair,
		options?: Partial<SignerOptions>,
	): Promise<BondTxSuccess | GenericFailure> {
		const optionWrapper = options || {};
		const maybeTxResult = await new Promise<Result<ISubmittableResult, string>>((res, _) => {
			this.api.tx.staking.bond(value, payee).signAndSend(account, optionWrapper, (result: ISubmittableResult) => {
				standardCallback(result, res, waitFor);
			}).catch((reason) => {
				res(err(reason));
			});
		});

		if (maybeTxResult.isErr()) {
			return { isErr: true, reason: maybeTxResult.error } as GenericFailure;
		}
		const txResult = maybeTxResult.value;

		if (txResult.isError) {
			return { isErr: true, reason: "The transaction was dropped or something." } as GenericFailure;
		}

		const events = txResult.events;
		const failed = events.find((e) => this.api.events.system.ExtrinsicFailed.is(e.event));
		if (failed != undefined) {
			return { isErr: true, reason: decodeError(this.api, failed.event.data[0]) } as GenericFailure;
		}

		const event = Events.Staking.Bonded.New(txResult.events);
		if (event == undefined) {
			return { isErr: true, reason: "Failed to find Bonded event." } as GenericFailure;
		}

		const [txHash, txIndex, blockHash, blockNumber] = await getBlockHashAndTxHash(txResult, waitFor, this.api);

		return { isErr: false, event, events, txHash, txIndex, blockHash, blockNumber } as BondTxSuccess;
	}

	async bondExtra(
		maxAdditional: BN,
		waitFor: WaitFor,
		account: KeyringPair,
		options?: Partial<SignerOptions>,
	): Promise<BondExtraTxSuccess | GenericFailure> {
		const optionWrapper = options || {};
		const maybeTxResult = await new Promise<Result<ISubmittableResult, string>>((res, _) => {
			this.api.tx.staking.bondExtra(maxAdditional).signAndSend(account, optionWrapper, (result: ISubmittableResult) => {
				standardCallback(result, res, waitFor);
			}).catch((reason) => {
				res(err(reason));
			});
		});

		if (maybeTxResult.isErr()) {
			return { isErr: true, reason: maybeTxResult.error } as GenericFailure;
		}
		const txResult = maybeTxResult.value;

		if (txResult.isError) {
			return { isErr: true, reason: "The transaction was dropped or something." } as GenericFailure;
		}

		const events = txResult.events;
		const failed = events.find((e) => this.api.events.system.ExtrinsicFailed.is(e.event));
		if (failed != undefined) {
			return { isErr: true, reason: decodeError(this.api, failed.event.data[0]) } as GenericFailure;
		}

		const event = Events.Staking.Bonded.New(txResult.events);
		if (event == undefined) {
			return { isErr: true, reason: "Failed to find Bonded event." } as GenericFailure;
		}

		const [txHash, txIndex, blockHash, blockNumber] = await getBlockHashAndTxHash(txResult, waitFor, this.api);

		return { isErr: false, event, events, txHash, txIndex, blockHash, blockNumber } as BondExtraTxSuccess;
	}

	async chill(waitFor: WaitFor, account: KeyringPair, options?: Partial<SignerOptions>): Promise<ChillTxSuccess | GenericFailure> {
		const optionWrapper = options || {};
		const maybeTxResult = await new Promise<Result<ISubmittableResult, string>>((res, _) => {
			this.api.tx.staking.chill().signAndSend(account, optionWrapper, (result: ISubmittableResult) => {
				standardCallback(result, res, waitFor);
			}).catch((reason) => {
				res(err(reason));
			});
		});

		if (maybeTxResult.isErr()) {
			return { isErr: true, reason: maybeTxResult.error } as GenericFailure;
		}
		const txResult = maybeTxResult.value;

		if (txResult.isError) {
			return { isErr: true, reason: "The transaction was dropped or something." } as GenericFailure;
		}

		const events = txResult.events;
		const failed = events.find((e) => this.api.events.system.ExtrinsicFailed.is(e.event));
		if (failed != undefined) {
			return { isErr: true, reason: decodeError(this.api, failed.event.data[0]) } as GenericFailure;
		}

		const event = Events.Staking.Chilled.New(txResult.events);
		if (event == undefined) {
			return { isErr: true, reason: "Failed to find Chilled event." } as GenericFailure;
		}

		const [txHash, txIndex, blockHash, blockNumber] = await getBlockHashAndTxHash(txResult, waitFor, this.api);

		return { isErr: false, event, events, txHash, txIndex, blockHash, blockNumber } as ChillTxSuccess;
	}

	async chillOther(
		stash: string,
		waitFor: WaitFor,
		account: KeyringPair,
		options?: Partial<SignerOptions>,
	): Promise<ChillOtherTxSuccess | GenericFailure> {
		const optionWrapper = options || {};
		const maybeTxResult = await new Promise<Result<ISubmittableResult, string>>((res, _) => {
			this.api.tx.staking.chillOther(stash).signAndSend(account, optionWrapper, (result: ISubmittableResult) => {
				standardCallback(result, res, waitFor);
			}).catch((reason) => {
				res(err(reason));
			});
		});

		if (maybeTxResult.isErr()) {
			return { isErr: true, reason: maybeTxResult.error } as GenericFailure;
		}
		const txResult = maybeTxResult.value;

		if (txResult.isError) {
			return { isErr: true, reason: "The transaction was dropped or something." } as GenericFailure;
		}

		const events = txResult.events;
		const failed = events.find((e) => this.api.events.system.ExtrinsicFailed.is(e.event));
		if (failed != undefined) {
			return { isErr: true, reason: decodeError(this.api, failed.event.data[0]) } as GenericFailure;
		}

		const event = Events.Staking.Chilled.New(txResult.events);
		if (event == undefined) {
			return { isErr: true, reason: "Failed to find Chilled event." } as GenericFailure;
		}

		const [txHash, txIndex, blockHash, blockNumber] = await getBlockHashAndTxHash(txResult, waitFor, this.api);

		return { isErr: false, event, events, txHash, txIndex, blockHash, blockNumber } as ChillOtherTxSuccess;
	}

	async nominate(
		targets: string[],
		waitFor: WaitFor,
		account: KeyringPair,
		options?: Partial<SignerOptions>,
	): Promise<NominateTxSuccess | GenericFailure> {
		const optionWrapper = options || {};
		const maybeTxResult = await new Promise<Result<ISubmittableResult, string>>((res, _) => {
			this.api.tx.staking.nominate(targets).signAndSend(account, optionWrapper, (result: ISubmittableResult) => {
				standardCallback(result, res, waitFor);
			}).catch((reason) => {
				res(err(reason));
			});
		});

		if (maybeTxResult.isErr()) {
			return { isErr: true, reason: maybeTxResult.error } as GenericFailure;
		}
		const txResult = maybeTxResult.value;

		if (txResult.isError) {
			return { isErr: true, reason: "The transaction was dropped or something." } as GenericFailure;
		}

		const events = txResult.events;
		const failed = events.find((e) => this.api.events.system.ExtrinsicFailed.is(e.event));
		if (failed != undefined) {
			return { isErr: true, reason: decodeError(this.api, failed.event.data[0]) } as GenericFailure;
		}

		const [txHash, txIndex, blockHash, blockNumber] = await getBlockHashAndTxHash(txResult, waitFor, this.api);
		const maybeTxData = await TransactionData.Staking.Nominate.New(this.api, txHash, blockHash);
		if (maybeTxData.isErr()) {
			return { isErr: true, reason: maybeTxData.error } as GenericFailure;
		}

		return { isErr: false, txData: maybeTxData.value, events, txHash, txIndex, blockHash, blockNumber } as NominateTxSuccess;
	}

	async unbond(
		value: BN,
		waitFor: WaitFor,
		account: KeyringPair,
		options?: Partial<SignerOptions>,
	): Promise<UnbondTxSuccess | GenericFailure> {
		const optionWrapper = options || {};
		const maybeTxResult = await new Promise<Result<ISubmittableResult, string>>((res, _) => {
			this.api.tx.staking.unbond(value).signAndSend(account, optionWrapper, (result: ISubmittableResult) => {
				standardCallback(result, res, waitFor);
			}).catch((reason) => {
				res(err(reason));
			});
		});

		if (maybeTxResult.isErr()) {
			return { isErr: true, reason: maybeTxResult.error } as GenericFailure;
		}
		const txResult = maybeTxResult.value;

		if (txResult.isError) {
			return { isErr: true, reason: "The transaction was dropped or something." } as GenericFailure;
		}

		const events = txResult.events;
		const failed = events.find((e) => this.api.events.system.ExtrinsicFailed.is(e.event));
		if (failed != undefined) {
			return { isErr: true, reason: decodeError(this.api, failed.event.data[0]) } as GenericFailure;
		}

		const event = Events.Staking.Unbonded.New(txResult.events);
		if (event == undefined) {
			return { isErr: true, reason: "Failed to find Unbonded event." } as GenericFailure;
		}

		const [txHash, txIndex, blockHash, blockNumber] = await getBlockHashAndTxHash(txResult, waitFor, this.api);

		return { isErr: false, event, events, txHash, txIndex, blockHash, blockNumber } as UnbondTxSuccess;
	}

	async validate(
		commission: number,
		blocked: boolean,
		waitFor: WaitFor,
		account: KeyringPair,
		options?: Partial<SignerOptions>,
	): Promise<ValidateTxSuccess | GenericFailure> {
		const maybeCommission = commissionNumberToPerbill(commission);
		if (maybeCommission.isErr()) {
			return { isErr: true, reason: maybeCommission.error } as GenericFailure;
		}

		const validatorPerfs = { commission: maybeCommission.value, blocked } as ValidatorPerfs;
		const optionWrapper = options || {};
		const maybeTxResult = await new Promise<Result<ISubmittableResult, string>>((res, _) => {
			this.api.tx.staking.validate(validatorPerfs).signAndSend(account, optionWrapper, (result: ISubmittableResult) => {
				standardCallback(result, res, waitFor);
			}).catch((reason) => {
				res(err(reason));
			});
		});

		if (maybeTxResult.isErr()) {
			return { isErr: true, reason: maybeTxResult.error } as GenericFailure;
		}
		const txResult = maybeTxResult.value;

		if (txResult.isError) {
			return { isErr: true, reason: "The transaction was dropped or something." } as GenericFailure;
		}

		const events = txResult.events;
		const failed = events.find((e) => this.api.events.system.ExtrinsicFailed.is(e.event));
		if (failed != undefined) {
			return { isErr: true, reason: decodeError(this.api, failed.event.data[0]) } as GenericFailure;
		}

		const event = Events.Staking.ValidatorPrefsSet.New(txResult.events);
		if (event == undefined) {
			return { isErr: true, reason: "Failed to find ValidatorPrefsSet event." } as GenericFailure;
		}

		const [txHash, txIndex, blockHash, blockNumber] = await getBlockHashAndTxHash(txResult, waitFor, this.api);

		return { isErr: false, event, events, txHash, txIndex, blockHash, blockNumber } as ValidateTxSuccess;
	}
}

export class Balances {
	private api: ApiPromise;

	constructor(api: ApiPromise) {
		this.api = api;
	}

	async transferAll(
		dest: string,
		keepAlive: boolean,
		waitFor: WaitFor,
		account: KeyringPair,
		options?: Partial<SignerOptions>,
	): Promise<TransferAllTxSuccess | GenericFailure> {
		const optionWrapper = options || {};
		const maybeTxResult = await new Promise<Result<ISubmittableResult, string>>((res, _) => {
			this.api.tx.balances.transferAll(dest, keepAlive).signAndSend(account, optionWrapper, (result: ISubmittableResult) => {
				standardCallback(result, res, waitFor);
			}).catch((reason) => {
				res(err(reason));
			});
		});

		if (maybeTxResult.isErr()) {
			return { isErr: true, reason: maybeTxResult.error } as GenericFailure;
		}
		const txResult = maybeTxResult.value;

		if (txResult.isError) {
			return { isErr: true, reason: "The transaction was dropped or something." } as GenericFailure;
		}

		const events = txResult.events;
		const failed = events.find((e) => this.api.events.system.ExtrinsicFailed.is(e.event));
		if (failed != undefined) {
			return { isErr: true, reason: decodeError(this.api, failed.event.data[0]) } as GenericFailure;
		}

		const event = Events.Balances.TransferEvent.New(txResult.events);
		if (event == undefined) {
			return { isErr: true, reason: "Failed to find Transfer event." } as GenericFailure;
		}
		const event2 = Events.System.KilledAccount.New(txResult.events);

		const [txHash, txIndex, blockHash, blockNumber] = await getBlockHashAndTxHash(txResult, waitFor, this.api);

		return { isErr: false, event, event2, events, txHash, txIndex, blockHash, blockNumber } as TransferAllTxSuccess;
	}

	async transferAllowDeath(
		dest: string,
		value: BN,
		waitFor: WaitFor,
		account: KeyringPair,
		options?: Partial<SignerOptions>,
	): Promise<TransferAllowDeathTxSuccess | GenericFailure> {
		const optionWrapper = options || {};
		const maybeTxResult = await new Promise<Result<ISubmittableResult, string>>((res, _) => {
			this.api.tx.balances.transferAllowDeath(dest, value).signAndSend(account, optionWrapper, (result: ISubmittableResult) => {
				standardCallback(result, res, waitFor);
			}).catch((reason) => {
				res(err(reason));
			});
		});

		if (maybeTxResult.isErr()) {
			return { isErr: true, reason: maybeTxResult.error } as GenericFailure;
		}
		const txResult = maybeTxResult.value;

		if (txResult.isError) {
			return { isErr: true, reason: "The transaction was dropped or something." } as GenericFailure;
		}

		const events = txResult.events;
		const failed = events.find((e) => this.api.events.system.ExtrinsicFailed.is(e.event));
		if (failed != undefined) {
			return { isErr: true, reason: decodeError(this.api, failed.event.data[0]) } as GenericFailure;
		}

		const event = Events.Balances.TransferEvent.New(txResult.events);
		if (event == undefined) {
			return { isErr: true, reason: "Failed to find Transfer event." } as GenericFailure;
		}
		const event2 = Events.System.KilledAccount.New(txResult.events);

		const [txHash, txIndex, blockHash, blockNumber] = await getBlockHashAndTxHash(txResult, waitFor, this.api);

		return { isErr: false, event, event2, events, txHash, txIndex, blockHash, blockNumber } as TransferAllowDeathTxSuccess;
	}

	async transferKeepAlive(
		dest: string,
		value: BN,
		waitFor: WaitFor,
		account: KeyringPair,
		options?: Partial<SignerOptions>,
	): Promise<TransferKeepAliveTxSuccess | GenericFailure> {
		const optionWrapper = options || {};
		const maybeTxResult = await new Promise<Result<ISubmittableResult, string>>((res, _) => {
			this.api.tx.balances.transferKeepAlive(dest, value).signAndSend(account, optionWrapper, (result: ISubmittableResult) => {
				standardCallback(result, res, waitFor);
			}).catch((reason) => {
				res(err(reason));
			});
		});

		if (maybeTxResult.isErr()) {
			return { isErr: true, reason: maybeTxResult.error } as GenericFailure;
		}
		const txResult = maybeTxResult.value;

		if (txResult.isError) {
			return { isErr: true, reason: "The transaction was dropped or something." } as GenericFailure;
		}

		const events = txResult.events;
		const failed = events.find((e) => this.api.events.system.ExtrinsicFailed.is(e.event));
		if (failed != undefined) {
			return { isErr: true, reason: decodeError(this.api, failed.event.data[0]) } as GenericFailure;
		}

		const event = Events.Balances.TransferEvent.New(txResult.events);
		if (event == undefined) {
			return { isErr: true, reason: "Failed to find Transfer event." } as GenericFailure;
		}

		const [txHash, txIndex, blockHash, blockNumber] = await getBlockHashAndTxHash(txResult, waitFor, this.api);

		return { isErr: false, event, events, txHash, txIndex, blockHash, blockNumber } as TransferKeepAliveTxSuccess;
	}
}

export class DataAvailability {
	private api: ApiPromise;

	constructor(api: ApiPromise) {
		this.api = api;
	}

	async submitData(
		data: string,
		waitFor: WaitFor,
		account: KeyringPair,
		options?: Partial<SignerOptions>,
	): Promise<SubmitDataTxSuccess | GenericFailure> {
		const optionWrapper = options || {};
		const maybeTxResult = await new Promise<Result<ISubmittableResult, string>>((res, _) => {
			this.api.tx.dataAvailability.submitData(data).signAndSend(account, optionWrapper, (result: ISubmittableResult) => {
				standardCallback(result, res, waitFor);
			}).catch((reason) => {
				res(err(reason));
			});
		});

		if (maybeTxResult.isErr()) {
			return { isErr: true, reason: maybeTxResult.error } as GenericFailure;
		}
		const txResult = maybeTxResult.value;

		if (txResult.isError) {
			return { isErr: true, reason: "The transaction was dropped or something." } as GenericFailure;
		}

		const events = txResult.events;
		const failed = events.find((e) => this.api.events.system.ExtrinsicFailed.is(e.event));
		if (failed != undefined) {
			return { isErr: true, reason: decodeError(this.api, failed.event.data[0]) } as GenericFailure;
		}

		const event = Events.DataAvailability.DataSubmittedEvent.New(txResult.events);
		if (event == undefined) {
			return { isErr: true, reason: "Failed to find DataSubmitted event." } as GenericFailure;
		}

		const [txHash, txIndex, blockHash, blockNumber] = await getBlockHashAndTxHash(txResult, waitFor, this.api);

		const maybeTxData = await TransactionData.DataAvailability.SubmitData.New(this.api, txHash, blockHash);
		if (maybeTxData.isErr()) {
			return { isErr: true, reason: maybeTxData.error } as GenericFailure;
		}

		return { isErr: false, txData: maybeTxData.value, event, events, txHash, txIndex, blockHash, blockNumber } as SubmitDataTxSuccess;
	}

	async createApplicationKey(
		key: string,
		waitFor: WaitFor,
		account: KeyringPair,
		options?: Partial<SignerOptions>,
	): Promise<CreateApplicationKeyTxSuccess | GenericFailure> {
		const optionWrapper = options || {};
		const maybeTxResult = await new Promise<Result<ISubmittableResult, string>>((res, _) => {
			this.api.tx.dataAvailability.createApplicationKey(key).signAndSend(account, optionWrapper, (result: ISubmittableResult) => {
				standardCallback(result, res, waitFor);
			}).catch((reason) => {
				res(err(reason));
			});
		});

		if (maybeTxResult.isErr()) {
			return { isErr: true, reason: maybeTxResult.error } as GenericFailure;
		}
		const txResult = maybeTxResult.value;

		if (txResult.isError) {
			return { isErr: true, reason: "The transaction was dropped or something." } as GenericFailure;
		}

		const events = txResult.events;
		const failed = events.find((e) => this.api.events.system.ExtrinsicFailed.is(e.event));
		if (failed != undefined) {
			return { isErr: true, reason: decodeError(this.api, failed.event.data[0]) } as GenericFailure;
		}

		const event = Events.DataAvailability.ApplicationKeyCreatedEvent.New(txResult.events);
		if (event == undefined) {
			return { isErr: true, reason: "Failed to find ApplicationKeyCreated event." } as GenericFailure;
		}

		const [txHash, txIndex, blockHash, blockNumber] = await getBlockHashAndTxHash(txResult, waitFor, this.api);

		return { isErr: false, event, events, txHash, txIndex, blockHash, blockNumber } as CreateApplicationKeyTxSuccess;
	}

	async setApplicationKey(
		oldKey: string,
		newKey: string,
		waitFor: WaitFor,
		account: KeyringPair,
		options?: Partial<SignerOptions>,
	): Promise<SetApplicationKeyTxSuccess | GenericFailure> {
		const optionWrapper = options || {};
		const maybeTxResult = await new Promise<Result<ISubmittableResult, string>>((res, _) => {
			const call = this.api.tx.dataAvailability.setApplicationKey(oldKey, newKey);
			this.api.tx.sudo.sudo(call).signAndSend(account, optionWrapper, (result: ISubmittableResult) => {
				standardCallback(result, res, waitFor);
			}).catch((reason) => {
				res(err(reason));
			});
		});

		if (maybeTxResult.isErr()) {
			return { isErr: true, reason: maybeTxResult.error } as GenericFailure;
		}
		const txResult = maybeTxResult.value;

		if (txResult.isError) {
			return { isErr: true, reason: "The transaction was dropped or something." } as GenericFailure;
		}

		const events = txResult.events;
		const failed = events.find((e) => this.api.events.system.ExtrinsicFailed.is(e.event));
		if (failed != undefined) {
			return { isErr: true, reason: decodeError(this.api, failed.event.data[0]) } as GenericFailure;
		}

		const sudoEvent = txResult.events.find((e) => e.event.method == "Sudid");
		if (sudoEvent == undefined) {
			return { isErr: true, reason: "Failed to find Sudid event." } as GenericFailure;
		}

		const sudoResult: any = (sudoEvent.event.data as any).sudoResult;
		if (sudoResult.isErr) {
			return { isErr: true, isFailure: true, reason: decodeError(this.api, sudoResult.asErr) } as GenericFailure;
		}

		const event = Events.DataAvailability.ApplicationKeySetEvent.New(txResult.events);
		if (event == undefined) {
			return { isErr: true, reason: "Failed to find ApplicationKeySet event." } as GenericFailure;
		}

		const [txHash, txIndex, blockHash, blockNumber] = await getBlockHashAndTxHash(txResult, waitFor, this.api);

		return { isErr: false, event, events, txHash, txIndex, blockHash, blockNumber } as SetApplicationKeyTxSuccess;
	}

	async submitBlockLengthProposal(
		rows: number,
		cols: number,
		waitFor: WaitFor,
		account: KeyringPair,
		options?: Partial<SignerOptions>,
	): Promise<SubmitBlockLengthProposalTxSuccess | GenericFailure> {
		const optionWrapper = options || {};
		const maybeTxResult = await new Promise<Result<ISubmittableResult, string>>((res, _) => {
			const call = this.api.tx.dataAvailability.submitBlockLengthProposal(rows, cols);
			this.api.tx.sudo.sudo(call).signAndSend(account, optionWrapper, (result: ISubmittableResult) => {
				standardCallback(result, res, waitFor);
			}).catch((reason) => {
				res(err(reason));
			});
		});

		if (maybeTxResult.isErr()) {
			return { isErr: true, reason: maybeTxResult.error } as GenericFailure;
		}
		const txResult = maybeTxResult.value;

		if (txResult.isError) {
			return { isErr: true, reason: "The transaction was dropped or something." } as GenericFailure;
		}

		const events = txResult.events;
		const failed = events.find((e) => this.api.events.system.ExtrinsicFailed.is(e.event));
		if (failed != undefined) {
			return { isErr: true, reason: decodeError(this.api, failed.event.data[0]) } as GenericFailure;
		}

		const sudoEvent = txResult.events.find((e) => e.event.method == "Sudid");
		if (sudoEvent == undefined) {
			return { isErr: true, reason: "Failed to find Sudid event." } as GenericFailure;
		}

		const sudoResult: any = (sudoEvent.event.data as any).sudoResult;
		if (sudoResult.isErr) {
			return { isErr: true, isFailure: true, reason: decodeError(this.api, sudoResult.asErr) } as GenericFailure;
		}

		const event = Events.DataAvailability.BlockLengthProposalSubmittedEvent.New(txResult.events);
		if (event == undefined) {
			return { isErr: true, reason: "Failed to find BlockLengthProposalSubmitted event." } as GenericFailure;
		}

		const [txHash, txIndex, blockHash, blockNumber] = await getBlockHashAndTxHash(txResult, waitFor, this.api);

		return { isErr: false, event, events, txHash, txIndex, blockHash, blockNumber } as SubmitBlockLengthProposalTxSuccess;
	}

	async setSubmitDataFeeModifier(
		modifier: DispatchFeeModifier,
		waitFor: WaitFor,
		account: KeyringPair,
		options?: Partial<SignerOptions>,
	): Promise<SetSubmitDataFeeModifierTxSuccess | GenericFailure> {
		const optionWrapper = options || {};
		const maybeTxResult = await new Promise<Result<ISubmittableResult, string>>((res, _) => {
			const call = this.api.tx.dataAvailability.setSubmitDataFeeModifier(modifier);
			this.api.tx.sudo.sudo(call).signAndSend(account, optionWrapper, (result: ISubmittableResult) => {
				standardCallback(result, res, waitFor);
			}).catch((reason) => {
				res(err(reason));
			});
		});

		if (maybeTxResult.isErr()) {
			return { isErr: true, reason: maybeTxResult.error } as GenericFailure;
		}
		const txResult = maybeTxResult.value;

		if (txResult.isError) {
			return { isErr: true, reason: "The transaction was dropped or something." } as GenericFailure;
		}

		const events = txResult.events;
		const failed = txResult.events.find((e) => this.api.events.system.ExtrinsicFailed.is(e.event));
		if (failed != undefined) {
			return { isErr: true, reason: decodeError(this.api, failed.event.data[0]) } as GenericFailure;
		}

		const sudoEvent = txResult.events.find((e) => e.event.method == "Sudid");
		if (sudoEvent == undefined) {
			return { isErr: true, reason: "Failed to find Sudid event." } as GenericFailure;
		}

		const sudoResult: any = (sudoEvent.event.data as any).sudoResult;
		if (sudoResult.isErr) {
			return { isErr: true, isFailure: true, reason: decodeError(this.api, sudoResult.asErr) } as GenericFailure;
		}

		const event = Events.DataAvailability.SubmitDataFeeModifierSetEvent.New(txResult.events);
		if (event == undefined) {
			return { isErr: true, reason: "Failed to find SubmitDataFeeModifierSet event." } as GenericFailure;
		}

		const [txHash, txIndex, blockHash, blockNumber] = await getBlockHashAndTxHash(txResult, waitFor, this.api);

		return { isErr: false, event, events, txHash, txIndex, blockHash, blockNumber } as SetSubmitDataFeeModifierTxSuccess;
	}
}