/// The example showcases how to programmatically estimate transactions fee.
///

import { ApiPromise, WsProvider } from "@polkadot/api/mod.ts";
import { BN } from "@polkadot/util/mod.ts";
import { API_EXTENSIONS, API_RPC, API_TYPES } from "../src/api_options.ts";

const api = await ApiPromise.create({
	provider: new WsProvider("ws://127.0.0.1:9944"),
	rpc: API_RPC,
	types: API_TYPES,
	signedExtensions: API_EXTENSIONS,
});

const sender = "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY";
const bob_address = "5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty";
const one_AVAIL = new BN("1000000000000000000");

const transfer_info = await api.tx.balances.transferKeepAlive(bob_address, one_AVAIL).paymentInfo(sender);
// log relevant info, partialFee is Balance, estimated for current
console.log(`Transaction Fee for Balance Transfer:
  class=${transfer_info.class.toString()},
  weight=${transfer_info.weight.toString()},
  partialFee=${transfer_info.partialFee.toHuman()}
`);

const data = "Hello World";
const submit_data_info = await api.tx.dataAvailability.submitData(data).paymentInfo(sender);
// log relevant info, partialFee is Balance, estimated for current
console.log(`Transaction Fee for Submit Data:
  class=${submit_data_info.class.toString()},
  weight=${submit_data_info.weight.toString()},
  partialFee=${submit_data_info.partialFee.toHuman()}
`);

Deno.exit(0);
