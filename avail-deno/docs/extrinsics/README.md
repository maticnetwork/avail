# Data Availability

Runtime Component: DataAvailability\
Runtime Index: 29\
Interface Module Name: dataAvailability

## Create Application Key

Origin Level: Signed

### Interface

```js
function createApplicationKey(key: string, waitFor: WaitFor, account: KeyringPair, options?: Partial<SignerOptions>): Promise<CreateApplicationKeyTxSuccess | GenericFailure>;
```

#### Parameters

| parameter | type          | optional | description                                     |
| --------- | ------------- | -------- | ----------------------------------------------- |
| key       | string        | false    | name of the application key                     |
| waitFor   | WaitFor       | false    | wait for block inclusion or finalization        |
| account   | KeyringPair   | false    | account that will send and sign the transaction |
| options   | SignerOptions | true     | used to overwrite existing signer options       |

### Minimal Example

```js
import { Keyring, SDK, WaitFor } from "https://raw.githubusercontent.com/availproject/avail/main/avail-deno/src/sdk.ts";

const providerEndpoint = "ws://127.0.0.1:9944";
const sdk = await SDK.New(providerEndpoint);

// Input
const account = new Keyring({ type: "sr25519" }).addFromUri("//Alice");
const key = "MyAwesomeKey";

const result = await sdk.tx.dataAvailability.createApplicationKey(key, WaitFor.BlockInclusion, account);
if (result.isErr) {
	console.log(result.reason);
	Deno.exit(1);
}

console.log(JSON.stringify(result, null, 4));

Deno.exit();
```

### Example Output

#### On Failure

If the operation fails, the function will return an error message indicating the nature of the issue.

#### On Success

If the operation is successful, the function will return a object of type `CreateApplicationKeyTxSuccess`.

```js
{
    "isErr": false,
    "event": {
        "key": "0x4d79417765736f6d654b6579",
        "owner": "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",
        "id": "10"
    },
    "events": [...],
    "txHash": "0x5ae9edbd2a2da96eeffc14cf9050d711082890fa6bfb8749ad2c4947565f3bd2",
    "txIndex": 1,
    "blockHash": "0x152338c1b0696d12664cf3d4c159af3d54beca151ba1ea8b00989a66dc8050b0",
    "blockNumber": 1
}
```

## Submit Data

Origin Level: Signed

### Interface

```js
function submitData(data: string, waitFor: WaitFor, account: KeyringPair, options?: Partial<SignerOptions>): Promise<SubmitDataTxSuccess | GenericFailure>;
```

#### Parameters

| parameter | type          | optional | description                                     |
| --------- | ------------- | -------- | ----------------------------------------------- |
| data      | string        | false    | data to be submitted                            |
| waitFor   | WaitFor       | false    | wait for block inclusion or finalization        |
| account   | KeyringPair   | false    | account that will send and sign the transaction |
| options   | SignerOptions | true     | used to overwrite existing signer options       |

### Minimal Example

```js
import { Keyring, SDK, WaitFor } from "https://raw.githubusercontent.com/availproject/avail/main/avail-deno/src/sdk.ts";

const providerEndpoint = "ws://127.0.0.1:9944";
const sdk = await SDK.New(providerEndpoint);

// Input
const account = new Keyring({ type: "sr25519" }).addFromUri("//Alice");
const data = "My Awesome Data";

const result = await sdk.tx.dataAvailability.submitData(data, WaitFor.BlockInclusion, account);
if (result.isErr) {
	console.log(result.reason);
	Deno.exit(1);
}

console.log(JSON.stringify(result, null, 4));

Deno.exit();
```

### Example Output

#### On Failure

If the operation fails, the function will return an error message indicating the nature of the issue.

#### On Success

If the operation is successful, the function will return a object of type `SubmitDataTxSuccess`.

```js
{
    "isErr": false,
    "txData": {
        "data": "4d7920417765736f6d652044617461"
    },
    "event": {
        "who": "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",
        "dataHash": "0x8846d900ea89aab9bce96402846c0ac74a853acc00cb99ff5ddb1a0f052594bd"
    },
    "events": [...],
    "txHash": "0xec6f9fd5e002c9ddbcd24764380f57a014de7f2007cc0e2ae11a4dda17ab8062",
    "txIndex": 1,
    "blockHash": "0x043c2b88ff960f2f7042521b55a943676938948febefe8684022b524795340d9",
    "blockNumber": 9
}
```

## Submit Block Length Proposal

Origin Level: Root

### Interface

```js
function submitBlockLengthProposal(rows: number, cols: number, waitFor: WaitFor, account: KeyringPair, options?: Partial<SignerOptions>): Promise<SubmitBlockLengthProposalTxSuccess | GenericFailure>;
```

#### Parameters

| parameter | type          | optional | description                                     |
| --------- | ------------- | -------- | ----------------------------------------------- |
| rows      | number        | false    | number of rows in block                         |
| cols      | number        | false    | number of cols in block                         |
| waitFor   | WaitFor       | false    | wait for block inclusion or finalization        |
| account   | KeyringPair   | false    | account that will send and sign the transaction |
| options   | SignerOptions | true     | used to overwrite existing signer options       |

### Minimal Example

```js
import { Keyring, SDK, WaitFor } from "https://raw.githubusercontent.com/availproject/avail/main/avail-deno/src/sdk.ts";

const providerEndpoint = "ws://127.0.0.1:9944";
const sdk = await SDK.New(providerEndpoint);

// Input
const account = new Keyring({ type: "sr25519" }).addFromUri("//Alice");
const rows = 128;
const cols = 128;

const result = await sdk.tx.dataAvailability.submitBlockLengthProposal(rows, cols, WaitFor.BlockInclusion, account);
if (result.isErr) {
	console.log(result.reason);
	Deno.exit(1);
}

console.log(JSON.stringify(result, null, 4));

Deno.exit();
```

### Example Output

#### On Failure

If the operation fails, the function will return an error message indicating the nature of the issue.

#### On Success

If the operation is successful, the function will return a object of type `SubmitBlockLengthProposalTxSuccess`.

```js
```

## Set Application Key

Origin Level: Root

### Interface

```js
function setApplicationKey(oldKey: string, newKey: string, waitFor: WaitFor, account: KeyringPair, options?: Partial<SignerOptions>): Promise<SetApplicationKeyTxSuccess | GenericFailure>;
```

#### Parameters

| parameter | type          | optional | description                                     |
| --------- | ------------- | -------- | ----------------------------------------------- |
| oldKey    | string        | false    | application key to be replaced                  |
| newKey    | string        | false    | application key that will replace the old one   |
| waitFor   | WaitFor       | false    | wait for block inclusion or finalization        |
| account   | KeyringPair   | false    | account that will send and sign the transaction |
| options   | SignerOptions | true     | used to overwrite existing signer options       |

### Minimal Example

```js
import { Keyring, SDK, WaitFor } from "https://raw.githubusercontent.com/availproject/avail/main/avail-deno/src/sdk.ts";

const providerEndpoint = "ws://127.0.0.1:9944";
const sdk = await SDK.New(providerEndpoint);

// Input
const account = new Keyring({ type: "sr25519" }).addFromUri("//Alice");
const oldKey = "MyNewKeyAwesome1";
const newKey = "MyNewKeyAwesome2";

const result = await sdk.tx.dataAvailability.setApplicationKey(oldKey, newKey, WaitFor.BlockInclusion, account);
if (result.isErr) {
	console.log(result.reason);
	Deno.exit(1);
}

console.log(JSON.stringify(result, null, 4));

Deno.exit();
```

### Example Output

#### On Failure

If the operation fails, the function will return an error message indicating the nature of the issue.

#### On Success

If the operation is successful, the function will return a object of type `SetApplicationKeyTxSuccess`.

```js
```

## Set Submit Data Fee Modifier

Origin Level: Root

### Interface

```js
function setSubmitDataFeeModifier(modifier: DispatchFeeModifier, waitFor: WaitFor, account: KeyringPair, options?: Partial<SignerOptions>): Promise<SetSubmitDataFeeModifierTxSuccess | GenericFailure>;
```

#### Parameters

| parameter | type                | optional | description                                     |
| --------- | ------------------- | -------- | ----------------------------------------------- |
| modifier  | DispatchFeeModifier | false    | new fee modifier values                         |
| waitFor   | WaitFor             | false    | wait for block inclusion or finalization        |
| account   | KeyringPair         | false    | account that will send and sign the transaction |
| options   | SignerOptions       | true     | used to overwrite existing signer options       |

### Minimal Example

```js
import { BN, DispatchFeeModifier, Keyring, SDK, WaitFor } from "https://raw.githubusercontent.com/availproject/avail/main/avail-deno/src/sdk.ts";

const providerEndpoint = "ws://127.0.0.1:9944";
const sdk = await SDK.New(providerEndpoint);

// Input
const account = new Keyring({ type: "sr25519" }).addFromUri("//Alice");
const modifier = { weightMaximumFee: new BN("10").pow(new BN("18")), weightFeeDivider: 20 } as DispatchFeeModifier;

const result = await sdk.tx.dataAvailability.setSubmitDataFeeModifier(modifier, WaitFor.BlockInclusion, account);
if (result.isErr) {
	console.log(result.reason);
	Deno.exit(1);
}

console.log(JSON.stringify(result, null, 4))

Deno.exit();
```

### Example Output

#### On Failure

If the operation fails, the function will return an error message indicating the nature of the issue.

#### On Success

If the operation is successful, the function will return a object of type `SetSubmitDataFeeModifierTxSuccess`.

```js
```

## Type Definitions

```js
enum WaitFor {
	BlockInclusion,
	BlockFinalization,
}
```

```js
type DispatchFeeModifier = { weightMaximumFee: BN | null,  weightFeeDivider: number | null, weightFeeMultiplier: number | null };
```

# Balances

Runtime Component: Balances\
Runtime Index: 6\
Interface Module Name: balances

## Transfer Keep Alive

Origin Level: Signed

### Interface

```js
function transferKeepAlive(dest: string, value: BN,  waitFor: WaitFor, account: KeyringPair, options?: Partial<SignerOptions>): Promise<TransferKeepAliveTxSuccess | GenericFailure>;
```

#### Parameters

| parameter | type          | optional | description                                     |
| --------- | ------------- | -------- | ----------------------------------------------- |
| dest      | string        | false    | account that will receive funds                 |
| value     | BN            | false    | amount that is send. 10^18 is equal to 1 AVL    |
| waitFor   | WaitFor       | false    | wait for block inclusion or finalization        |
| account   | KeyringPair   | false    | account that will send and sign the transaction |
| options   | SignerOptions | true     | used to overwrite existing signer options       |

### Minimal Example

```js
import { BN, Keyring, SDK, WaitFor } from "https://raw.githubusercontent.com/availproject/avail/main/avail-deno/src/sdk.ts";

const providerEndpoint = "ws://127.0.0.1:9944";
const sdk = await SDK.New(providerEndpoint);

// Input
const account = new Keyring({ type: "sr25519" }).addFromUri("//Alice");
const dest = "5HGjWAeFDfFCWPsjFQdVV2Msvz2XtMktvgocEZcCj68kUMaw"; // Eve
const amount = new BN(10).pow(new BN(18)); // one Avail

const result = await sdk.tx.balances.transferKeepAlive(dest, amount, WaitFor.BlockInclusion, account);
if (result.isErr) {
	console.log(result.reason);
	Deno.exit(1);
}

console.log(JSON.stringify(result, null, 4));

Deno.exit();
```

### Example Output

#### On Failure

If the operation fails, the function will return an error message indicating the nature of the issue.

#### On Success

If the operation is successful, the function will return a object of type `TransferKeepAliveTxSuccess`.

```js
{
    "isErr": false,
    "event": {
        "from": "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",
        "to": "5HGjWAeFDfFCWPsjFQdVV2Msvz2XtMktvgocEZcCj68kUMaw",
        "amount": "1000000000000000000"
    },
    "events": [...],
    "txHash": "0x812a3f3960afb8df72de0e5b86ff564c8ce7d93c837182c24d1796fb68a7f5f4",
    "txIndex": 1,
    "blockHash": "0xfdee1faced02696d692df1d896fa2822f4eb02f260c95e11041df86b2c229dfb",
    "blockNumber": 1
}
```

## Transfer Allow Death

Origin Level: Signed

### Interface

```js
function transferAllowDeath(dest: string, value: BN, waitFor: WaitFor, account: KeyringPair, options?: Partial<SignerOptions>): Promise<TransferAllowDeathTxSuccess | GenericFailure>;
```

#### Parameters

| parameter | type          | optional | description                                     |
| --------- | ------------- | -------- | ----------------------------------------------- |
| dest      | string        | false    | account that will receive funds                 |
| value     | BN            | false    | amount that is send. 10^18 is equal to 1 AVL    |
| waitFor   | WaitFor       | false    | wait for block inclusion or finalization        |
| account   | KeyringPair   | false    | account that will send and sign the transaction |
| options   | SignerOptions | true     | used to overwrite existing signer options       |

### Minimal Example

```js
import { BN, Keyring, SDK, WaitFor } from "https://raw.githubusercontent.com/availproject/avail/main/avail-deno/src/sdk.ts";

const providerEndpoint = "ws://127.0.0.1:9944";
const sdk = await SDK.New(providerEndpoint);

// Input
const account = new Keyring({ type: "sr25519" }).addFromUri("//Alice");
const dest = "5HGjWAeFDfFCWPsjFQdVV2Msvz2XtMktvgocEZcCj68kUMaw"; // Eve
const amount = new BN(10).pow(new BN(18)); // one Avail

const result = await sdk.tx.balances.transferAllowDeath(dest, amount, WaitFor.BlockInclusion, account);
if (result.isErr) {
	console.log(result.reason);
	Deno.exit(1);
}

console.log(JSON.stringify(result, null, 4));

Deno.exit();
```

### Example Output

#### On Failure

If the operation fails, the function will return an error message indicating the nature of the issue.

#### On Success

If the operation is successful, the function will return a object of type `TransferAllowDeathTxSuccess`.

```js
{
    "isErr": false,
    "event": {
        "from": "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",
        "to": "5HGjWAeFDfFCWPsjFQdVV2Msvz2XtMktvgocEZcCj68kUMaw",
        "amount": "1000000000000000000"
    },
    "events": [...],
    "txHash": "0x63a73d2d1210ab9840341506788cca9592fd968609fecb5106cf0370c611061c",
    "txIndex": 1,
    "blockHash": "0xde2e95b63a4ca5927f9105931e4676b0634d12f524d4fff1048b403393419489",
    "blockNumber": 2
}
```

## Transfer All

Origin Level: Signed

### Interface

```js
function transferAll(dest: string, keepAlive: boolean, waitFor: WaitFor, account: KeyringPair, options?: Partial<SignerOptions>): Promise<TransferAllTxSuccess | GenericFailure>;
```

#### Parameters

| parameter | type          | optional | description                                      |
| --------- | ------------- | -------- | ------------------------------------------------ |
| dest      | string        | false    | account that will receive funds                  |
| keepAlive | boolean       | false    | if set to false it will reap the account as well |
| waitFor   | WaitFor       | false    | wait for block inclusion or finalization         |
| account   | KeyringPair   | false    | account that will send and sign the transaction  |
| options   | SignerOptions | true     | used to overwrite existing signer options        |

### Minimal Example

```js
import { Keyring, SDK, WaitFor } from "https://raw.githubusercontent.com/availproject/avail/main/avail-deno/src/sdk.ts";

const providerEndpoint = "ws://127.0.0.1:9944";
const sdk = await SDK.New(providerEndpoint);

// Input
const account = new Keyring({ type: "sr25519" }).addFromUri("//Alice");
const dest = "5HGjWAeFDfFCWPsjFQdVV2Msvz2XtMktvgocEZcCj68kUMaw"; // Eve
const keepAlive = true;

const result = await sdk.tx.balances.transferAll(dest, keepAlive, WaitFor.BlockInclusion, account);
if (result.isErr) {
	console.log(result.reason);
	Deno.exit(1);
}

console.log(JSON.stringify(result, null, 4));

Deno.exit();
```

### Example Output

#### On Failure

If the operation fails, the function will return an error message indicating the nature of the issue.

#### On Success

If the operation is successful, the function will return a object of type `TransferAllTxSuccess`.

```js
{
    "isErr": false,
    "event": {
        "from": "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",
        "to": "5HGjWAeFDfFCWPsjFQdVV2Msvz2XtMktvgocEZcCj68kUMaw",
        "amount": "9999999873433871068464733"
    },
    "events": [...],
    "txHash": "0x343d3e8890bd479b4619cb7b0f2dfa91b7b91c0cedc0646247215f85baf1f63e",
    "txIndex": 1,
    "blockHash": "0xaec4adfad11f8aa902e1a985abb62737fc02445072b168238a956c3a0d8820f2",
    "blockNumber": 2
}
```

# Staking

Runtime Component: Staking\
Runtime Index: 10\
Interface Module Name: staking

## Bond

Origin Level: Signed

### Interface

```js
function bond(value: BN, payee: StakingRewardDestination, waitFor: WaitFor, account: KeyringPair, options?: Partial<SignerOptions>): Promise<BondTxSuccess | GenericFailure>;
```

#### Parameters

| parameter | type                     | optional | description                                             |
| --------- | ------------------------ | -------- | ------------------------------------------------------- |
| value     | BN                       | false    | amount that is bond. 10^18 is equal to 1 AVL            |
| payee     | StakingRewardDestination | false    | Can be: "Staked", "Stash", "None" or an account address |
| waitFor   | WaitFor                  | false    | wait for block inclusion or finalization                |
| account   | KeyringPair              | false    | account that will send and sign the transaction         |
| options   | SignerOptions            | true     | used to overwrite existing signer options               |

### Minimal Example

```js
import { BN, Keyring, SDK, WaitFor } from "https://raw.githubusercontent.com/availproject/avail/main/avail-deno/src/sdk.ts";

const providerEndpoint = "ws://127.0.0.1:9944";
const sdk = await SDK.New(providerEndpoint);

// Input
const account = new Keyring({ type: "sr25519" }).addFromUri("//Alice");
const value = new BN(100_000).mul(new BN(10).pow(new BN("18"))); // 100 000 Avail
const payee = "Staked";

const result = await sdk.tx.staking.bond(value, payee, WaitFor.BlockInclusion, account);
if (result.isErr) {
	console.log(result.reason);
	Deno.exit(1);
}

console.log(JSON.stringify(result, null, 4));

Deno.exit();
```

### Example Output

#### On Failure

If the operation fails, the function will return an error message indicating the nature of the issue.

#### On Success

If the operation is successful, the function will return a object of type `BondTxSuccess`.

```js
{
    "isErr": false,
    "event": {
        "stash": "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",
        "amount": "100000"
    },
    "events": [...],
    "txHash": "0x3e1cc48207b02ca5d680cf1beeb270ce7cbf0d18a6191844bc963d4081a0ca90",
    "txIndex": 1,
    "blockHash": "0xf854e74cb428d0baf22454cb15007731a84263e57c64d019a304c0ca1bd30276",
    "blockNumber": 2
}
```

## Bond Extra

Origin Level: Signed

### Interface

```js
function bondExtra(maxAdditional: BN, waitFor: WaitFor, account: KeyringPair, options?: Partial<SignerOptions>): Promise<BondExtraTxSuccess | GenericFailure>;
```

#### Parameters

| parameter     | type          | optional | description                                             |
| ------------- | ------------- | -------- | ------------------------------------------------------- |
| maxAdditional | BN            | false    | additional amount that is bond. 10^18 is equal to 1 AVL |
| waitFor       | WaitFor       | false    | wait for block inclusion or finalization                |
| account       | KeyringPair   | false    | account that will send and sign the transaction         |
| options       | SignerOptions | true     | used to overwrite existing signer options               |

### Minimal Example

```js
import { BN, Keyring, SDK, WaitFor } from "https://raw.githubusercontent.com/availproject/avail/main/avail-deno/src/sdk.ts";

const providerEndpoint = "ws://127.0.0.1:9944";
const sdk = await SDK.New(providerEndpoint);

// Input
const account = new Keyring({ type: "sr25519" }).addFromUri("//Alice//stash");
const maxAdditional = new BN(10).pow(new BN(18)); // one Avail

const result = await sdk.tx.staking.bondExtra(maxAdditional, WaitFor.BlockInclusion, account);
if (result.isErr) {
	console.log(result.reason);
	Deno.exit(1);
}

console.log(JSON.stringify(result, null, 4));

Deno.exit();
```

### Example Output

#### On Failure

If the operation fails, the function will return an error message indicating the nature of the issue.

#### On Success

If the operation is successful, the function will return a object of type `BondExtraTxSuccess`.

```js
{
    "isErr": false,
    "event": {
        "stash": "5GNJqTPyNqANBkUVMN1LPPrxXnFouWXoe2wNSmmEoLctxiZY",
        "amount": "1"
    },
    "events": [...],
    "txHash": "0x940df5141925aeef2ab9aa767f6870689426de533f5f1d84b6d7be203e68ee77",
    "txIndex": 1,
    "blockHash": "0xc2a8375be07956586833f497a429ca2e29bafbb78ee5e051d5157df0ad5c8cb6",
    "blockNumber": 7
}
```

## Chill

Origin Level: Signed

### Interface

```js
function chill(waitFor: WaitFor, account: KeyringPair, options?: Partial<SignerOptions>): Promise<ChillTxSuccess | GenericFailure>;
```

#### Parameters

| parameter | type          | optional | description                                     |
| --------- | ------------- | -------- | ----------------------------------------------- |
| waitFor   | WaitFor       | false    | wait for block inclusion or finalization        |
| account   | KeyringPair   | false    | account that will send and sign the transaction |
| options   | SignerOptions | true     | used to overwrite existing signer options       |

### Minimal Example

```js
import { Keyring, SDK, WaitFor } from "https://raw.githubusercontent.com/availproject/avail/main/avail-deno/src/sdk.ts";

const providerEndpoint = "ws://127.0.0.1:9944";
const sdk = await SDK.New(providerEndpoint);

// Input
const account = new Keyring({ type: "sr25519" }).addFromUri("//Alice//stash");

const result = await sdk.tx.staking.chill(WaitFor.BlockInclusion, account);
if (result.isErr) {
	console.log(result.reason);
	Deno.exit(1);
}

console.log(JSON.stringify(result, null, 4));

Deno.exit();
```

### Example Output

#### On Failure

If the operation fails, the function will return an error message indicating the nature of the issue.

#### On Success

If the operation is successful, the function will return a object of type `ChillTxSuccess`.

```js
{
    "isErr": false,
    "event": {
        "stash": "5GNJqTPyNqANBkUVMN1LPPrxXnFouWXoe2wNSmmEoLctxiZY"
    },
    "events": [...],
    "txHash": "0x4572681f19af32fdfb4759c914697697b0e82fde48a5dd7e28c2b3a263772b0d",
    "txIndex": 1,
    "blockHash": "0xad2e5376f53e6257e7bc0c842e5b6952f1d4af6f7499319b2d1ab59bdd742628",
    "blockNumber": 13
}
```

## Chill Other

Origin Level: Signed

### Interface

```js
function chillOther(stash: string, waitFor: WaitFor, account: KeyringPair, options?: Partial<SignerOptions>): Promise<ChillOtherTxSuccess | GenericFailure>;
```

#### Parameters

| parameter | type          | optional | description                                     |
| --------- | ------------- | -------- | ----------------------------------------------- |
| stash     | string        | false    | address of stash account to chill               |
| waitFor   | WaitFor       | false    | wait for block inclusion or finalization        |
| account   | KeyringPair   | false    | account that will send and sign the transaction |
| options   | SignerOptions | true     | used to overwrite existing signer options       |

### Minimal Example

```js
import { Keyring, SDK, WaitFor } from "https://raw.githubusercontent.com/availproject/avail/main/avail-deno/src/sdk.ts";

const providerEndpoint = "ws://127.0.0.1:9944";
const sdk = await SDK.New(providerEndpoint);

// Input
const account = new Keyring({ type: "sr25519" }).addFromUri("//Alice//stash");
const stash = "5GNJqTPyNqANBkUVMN1LPPrxXnFouWXoe2wNSmmEoLctxiZY"; // Alice Stash

const result = await sdk.tx.staking.chillOther(stash, WaitFor.BlockInclusion, account);
if (result.isErr) {
	console.log(result.reason);
	Deno.exit(1);
}

console.log(JSON.stringify(result, null, 4));

Deno.exit();
```

### Example Output

#### On Failure

If the operation fails, the function will return an error message indicating the nature of the issue.

#### On Success

If the operation is successful, the function will return a object of type `ChillOtherTxSuccess`.

```js
```

## Nominate

Origin Level: Signed

### Interface

```js
function nominate(targets: string[], waitFor: WaitFor, account: KeyringPair, options?: Partial<SignerOptions>): Promise<NominateTxSuccess | GenericFailure>;
```

#### Parameters

| parameter | type          | optional | description                                     |
| --------- | ------------- | -------- | ----------------------------------------------- |
| targets   | string[]      | false    | list od addresses to nominate                   |
| waitFor   | WaitFor       | false    | wait for block inclusion or finalization        |
| account   | KeyringPair   | false    | account that will send and sign the transaction |
| options   | SignerOptions | true     | used to overwrite existing signer options       |

### Minimal Example

```js
import { Keyring, SDK, WaitFor } from "https://raw.githubusercontent.com/availproject/avail/main/avail-deno/src/sdk.ts";

const providerEndpoint = "ws://127.0.0.1:9944";
const sdk = await SDK.New(providerEndpoint);

// Input
const account = new Keyring({ type: "sr25519" }).addFromUri("//Alice");
const targets = [
	"5GNJqTPyNqANBkUVMN1LPPrxXnFouWXoe2wNSmmEoLctxiZY", // Alice Stash
	"5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty", // Bob
];

const result = await sdk.tx.staking.nominate(targets, WaitFor.BlockInclusion, account);
if (result.isErr) {
	console.log(result.reason);
	Deno.exit(1);
}

console.log(JSON.stringify(result, null, 4));

Deno.exit();
```

### Example Output

#### On Failure

If the operation fails, the function will return an error message indicating the nature of the issue.

#### On Success

If the operation is successful, the function will return a object of type `NominateTxSuccess`.

```js
{
    "isErr": false,
    "txData": {
        "targets": [
            "5GNJqTPyNqANBkUVMN1LPPrxXnFouWXoe2wNSmmEoLctxiZY",
            "5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty"
        ]
    },
    "events": [...],
    "txHash": "0x2f81a34f59d36eb7ada96ec1070358043026d7bd7cfb6fa5a532cc474190880b",
    "txIndex": 1,
    "blockHash": "0x49a57953aa2b2ba508f1c6991515309a0fe89723a79f3831f9a9263ba8c7baa4",
    "blockNumber": 4
}
```

## Unbond

Origin Level: Signed

### Interface

```js
function unbond(value: BN, waitFor: WaitFor, account: KeyringPair, options?: Partial<SignerOptions>): Promise<UnbondTxSuccess | GenericFailure>;
```

#### Parameters

| parameter | type          | optional | description                                     |
| --------- | ------------- | -------- | ----------------------------------------------- |
| value     | BN            | false    | amount of tokens to unbond                      |
| waitFor   | WaitFor       | false    | wait for block inclusion or finalization        |
| account   | KeyringPair   | false    | account that will send and sign the transaction |
| options   | SignerOptions | true     | used to overwrite existing signer options       |

### Minimal Example

```js
import { BN, Keyring, SDK, WaitFor } from "https://raw.githubusercontent.com/availproject/avail/main/avail-deno/src/sdk.ts";

const providerEndpoint = "ws://127.0.0.1:9944";
const sdk = await SDK.New(providerEndpoint);

// Input
const account = new Keyring({ type: "sr25519" }).addFromUri("//Alice");
const value = new BN(10).pow(new BN(18)); // one Avail

const result = await sdk.tx.staking.unbond(value, WaitFor.BlockInclusion, account);
if (result.isErr) {
	console.log(result.reason);
	Deno.exit(1);
}

console.log(JSON.stringify(result, null, 4));

Deno.exit();
```

### Example Output

#### On Failure

If the operation fails, the function will return an error message indicating the nature of the issue.

#### On Success

If the operation is successful, the function will return a object of type `UnbondTxSuccess`.

```js
{
    "isErr": false,
    "event": {
        "stash": "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",
        "amount": "1000000000000000000"
    },
    "events": [...],
    "txHash": "0xbf264e0e95885fd64a35d5c64bd4e1cc17056a1e6b05fa9207d7c777395dffdf",
    "txIndex": 1,
    "blockHash": "0x9ef43aaca71ba7b91a53976de5170f80d8a1ed4fe3e95fae237f7ed91f953963",
    "blockNumber": 9
}
```

## Validate

Origin Level: Signed

### Interface

```js
function validate(commission: number, blocked: boolean, waitFor: WaitFor, account: KeyringPair, options?: Partial<SignerOptions>): Promise<ValidateTxSuccess | GenericFailure>;
```

#### Parameters

| parameter  | type          | optional | description                                           |
| ---------- | ------------- | -------- | ----------------------------------------------------- |
| commission | number        | false    | how much validator charge nominators in 0 - 100 range |
| blocked    | boolean       | false    | whether or not this validator accepts nominations     |
| waitFor    | WaitFor       | false    | wait for block inclusion or finalization              |
| account    | KeyringPair   | false    | account that will send and sign the transaction       |
| options    | SignerOptions | true     | used to overwrite existing signer options             |

### Minimal Example

```js
import { Keyring, SDK, WaitFor } from "https://raw.githubusercontent.com/availproject/avail/main/avail-deno/src/sdk.ts";

const providerEndpoint = "ws://127.0.0.1:9944";
const sdk = await SDK.New(providerEndpoint);

// Input
const account = new Keyring({ type: "sr25519" }).addFromUri("//Alice");
const commission = 5; // 5%
const blocked = false;

const result = await sdk.tx.staking.validate(commission, blocked, WaitFor.BlockInclusion, account);
if (result.isErr) {
	console.log(result.reason);
	Deno.exit(1);
}

console.log(JSON.stringify(result, null, 4));

Deno.exit();
```

### Example Output

#### On Failure

If the operation fails, the function will return an error message indicating the nature of the issue.

#### On Success

If the operation is successful, the function will return a object of type `ValidateTxSuccess`.

```js
{
    "isErr": false,
    "event": {
        "stash": "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",
        "commission": "50000000",
        "blocked": "false"
    },
    "events": [...],
    "txHash": "0x31f047da16a350e32b832cc73d3351c8d5e5991625fde6e8c36fc45ebb9d2735",
    "txIndex": 1,
    "blockHash": "0xa7735804f52602d4b73e1dd7f718cf0ab5cc00d111c927a9f8a2b3d02b66e09a",
    "blockNumber": 14
}
```

# Nomination Pools

Runtime Component: Nomination Pools\
Runtime Index: 36\
Interface Module Name: nominationPools

## Create

Origin Level: Signed

### Interface

```js
function create(amount: BN, root: string, nominator: string, bouncer: string, waitFor: WaitFor, account: KeyringPair, options?: Partial<SignerOptions>): Promise<PoolCreateTxSuccess | GenericFailure>;
```

#### Parameters

| parameter | type          | optional | description                                        |
| --------- | ------------- | -------- | -------------------------------------------------- |
| amount    | BN            | false    | The amount of funds to delegate to the pool        |
| root      | string        | false    | The account to set as [`PoolRoles::root`]          |
| nominator | string        | false    | The account to set as the [`PoolRoles::nominator`] |
| bouncer   | string        | false    | The account to set as the [`PoolRoles::bouncer`]   |
| waitFor   | WaitFor       | false    | wait for block inclusion or finalization           |
| account   | KeyringPair   | false    | account that will send and sign the transaction    |
| options   | SignerOptions | true     | used to overwrite existing signer options          |

### Minimal Example

```js
import { BN, Keyring, SDK, WaitFor } from "https://raw.githubusercontent.com/availproject/avail/main/avail-deno/src/sdk.ts";

const providerEndpoint = "ws://127.0.0.1:9944";
const sdk = await SDK.New(providerEndpoint);

// Input
const account = new Keyring({ type: "sr25519" }).addFromUri("//Alice");
const amount = new BN(10).pow(new BN(18)).mul(new BN(10000)); // 10_000 Avail

const root: string = "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY"; // Alice
const nominator: string = "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY"; // Alice
const bouncer: string = "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY"; // Alice

const result = await sdk.tx.nominationPools.create(amount, root, nominator, bouncer, WaitFor.BlockInclusion, account);
if (result.isErr) {
	console.log(result.reason);
	Deno.exit(1);
}

console.log(JSON.stringify(result, null, 4));

Deno.exit();
```

### Example Output

#### On Failure

If the operation fails, the function will return an error message indicating the nature of the issue.

#### On Success

If the operation is successful, the function will return a object of type `PoolCreateTxSuccess`.

```js
{
    "isErr": false,
	"event": {
        "depositor": "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",
        "poolId": "1"
    },
    "event2": {
        "member": "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",
        "poolId": "1",
        "bonded": "10000",
        "joined": "true"
    },
    "event": {
        "key": "0x4d79417765736f6d654b6579",
        "owner": "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",
        "id": "10"
    },
    "events": [...],
    "txHash": "0x5ae9edbd2a2da96eeffc14cf9050d711082890fa6bfb8749ad2c4947565f3bd2",
    "txIndex": 1,
    "blockHash": "0x152338c1b0696d12664cf3d4c159af3d54beca151ba1ea8b00989a66dc8050b0",
    "blockNumber": 1
}
```

## Create with Pool Id

Origin Level: Signed

### Interface

```js
function createWithPoolId(amount: BN, root: string, nominator: string, bouncer: string, poolId: number, waitFor: WaitFor, account: KeyringPair, options?: Partial<SignerOptions>): Promise<PoolCreateWithPoolIdTxSuccess | GenericFailure>;
```

#### Parameters

| parameter | type          | optional | description                                        |
| --------- | ------------- | -------- | -------------------------------------------------- |
| amount    | BN            | false    | The amount of funds to delegate to the pool        |
| root      | string        | false    | The account to set as [`PoolRoles::root`]          |
| nominator | string        | false    | The account to set as the [`PoolRoles::nominator`] |
| bouncer   | string        | false    | The account to set as the [`PoolRoles::bouncer`]   |
| poolId    | number        | false    | pool id                                            |
| waitFor   | WaitFor       | false    | wait for block inclusion or finalization           |
| account   | KeyringPair   | false    | account that will send and sign the transaction    |
| options   | SignerOptions | true     | used to overwrite existing signer options          |

### Minimal Example

```js
import { BN, Keyring, SDK, WaitFor } from "https://raw.githubusercontent.com/availproject/avail/main/avail-deno/src/sdk.ts";

const providerEndpoint = "ws://127.0.0.1:9944";
const sdk = await SDK.New(providerEndpoint);

// Input
const account = new Keyring({ type: "sr25519" }).addFromUri("//Bob");
const amount = new BN(10).pow(new BN(18)).mul(new BN(10000)); // 10_000 Avail

const root: string = "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY"; // Alice
const nominator: string = "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY"; // Alice
const bouncer: string = "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY"; // Alice
const poolId = 0;

const result = await sdk.tx.nominationPools.createWithPoolId(amount, root, nominator, bouncer, poolId, WaitFor.BlockInclusion, account);
if (result.isErr) {
	console.log(result.reason);
	Deno.exit(1);
}

console.log(JSON.stringify(result, null, 4));

Deno.exit();
```

### Example Output

#### On Failure

If the operation fails, the function will return an error message indicating the nature of the issue.

#### On Success

If the operation is successful, the function will return a object of type `PoolCreateWithPoolIdTxSuccess`.

```js
{
    "isErr": false,
    "event": {
        "depositor": "5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty",
        "poolId": "0"
    },
    "event2": {
        "member": "5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty",
        "poolId": "0",
        "bonded": "10000",
        "joined": "true"
    },
    "events": [...],
    "txHash": "0x6b50caed7950e67934cabbf88a1f7dc2e7e995ac608402f91a4db19be0da5c41",
    "txIndex": 1,
    "blockHash": "0xc06df7dbb1e404f54499f942479ddcffc92665c021ea07c2798fc2f354f403d3",
    "blockNumber": 6
}
```

## Join

Origin Level: Signed

### Interface

```js
function join(amount: BN, poolId: number, waitFor: WaitFor, account: KeyringPair, options?: Partial<SignerOptions>): Promise<PoolJoinTxSuccess | GenericFailure>;
```

#### Parameters

| parameter | type          | optional | description                                     |
| --------- | ------------- | -------- | ----------------------------------------------- |
| amount    | BN            | false    | The amount of funds to delegate to the pool     |
| poolId    | number        | false    | pool id                                         |
| waitFor   | WaitFor       | false    | wait for block inclusion or finalization        |
| account   | KeyringPair   | false    | account that will send and sign the transaction |
| options   | SignerOptions | true     | used to overwrite existing signer options       |

### Minimal Example

```js
import { BN, Keyring, SDK, WaitFor } from "https://raw.githubusercontent.com/availproject/avail/main/avail-deno/src/sdk.ts";

const providerEndpoint = "ws://127.0.0.1:9944";
const sdk = await SDK.New(providerEndpoint);

// Input
const account = new Keyring({ type: "sr25519" }).addFromUri("//Bob");
const amount = new BN(10).pow(new BN(18)).mul(new BN(10000)); // 10_000 Avail
const poolId = 1;

const result = await sdk.tx.nominationPools.join(amount, poolId, WaitFor.BlockInclusion, account);
if (result.isErr) {
	console.log(result.reason);
	Deno.exit(1);
}

console.log(JSON.stringify(result, null, 4));

Deno.exit();
```

### Example Output

#### On Failure

If the operation fails, the function will return an error message indicating the nature of the issue.

#### On Success

If the operation is successful, the function will return a object of type `PoolJoinTxSuccess`.

```js
{
    "isErr": false,
    "event": {
        "member": "5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty",
        "poolId": "1",
        "bonded": "10000",
        "joined": "true"
    },
    "events": [...],
    "txHash": "0x06baecbb8680e90d025d1fd08044d0d251054a89e82dd460022bdf3796020050",
    "txIndex": 1,
    "blockHash": "0x82078130da46adacf5bdff86618ab6e1c443fda6d883d9fcf967a41a2e29d612",
    "blockNumber": 19
}
```

## Nominate

Origin Level: Signed

### Interface

```js
function nominate(poolId: number, validators: string[], waitFor: WaitFor, account: KeyringPair, options?: Partial<SignerOptions>): Promise<PoolNominateTxSuccess | GenericFailure>;
```

#### Parameters

| parameter  | type          | optional | description                                     |
| ---------- | ------------- | -------- | ----------------------------------------------- |
| poolId     | number        | false    | pool id                                         |
| validators | string[]      | false    | list of validators to nominate                  |
| waitFor    | WaitFor       | false    | wait for block inclusion or finalization        |
| account    | KeyringPair   | false    | account that will send and sign the transaction |
| options    | SignerOptions | true     | used to overwrite existing signer options       |

### Minimal Example

```js
import { Keyring, SDK, WaitFor } from "https://raw.githubusercontent.com/availproject/avail/main/avail-deno/src/sdk.ts";

const providerEndpoint = "ws://127.0.0.1:9944";
const sdk = await SDK.New(providerEndpoint);

// Input
const account = new Keyring({ type: "sr25519" }).addFromUri("//Alice");
const validators: string[] = ["5GNJqTPyNqANBkUVMN1LPPrxXnFouWXoe2wNSmmEoLctxiZY", "5DAAnrj7VHTznn2AWBemMuyBwZWs6FNFjdyVXUeYum3PTXFy"];
const poolId = 1;

const result = await sdk.tx.nominationPools.nominate(poolId, validators, WaitFor.BlockInclusion, account);
if (result.isErr) {
	console.log(result.reason);
	Deno.exit(1);
}

console.log(JSON.stringify(result, null, 4));

Deno.exit();
```

### Example Output

#### On Failure

If the operation fails, the function will return an error message indicating the nature of the issue.

#### On Success

If the operation is successful, the function will return a object of type `PoolNominateTxSuccess`.

```js
{
    "isErr": false,
    "events": [...],
    "txHash": "0x98b993baf90183d85dece9357d3bc32311f4201b015b63845a13dbc22bf22370",
    "txIndex": 1,
    "blockHash": "0x84ef5a0ada4af71358ee701a2500bce7f6688efb554c32ba1a30c459f64d5370",
    "blockNumber": 48
}
```
