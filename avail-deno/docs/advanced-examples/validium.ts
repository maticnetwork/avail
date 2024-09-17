import { ApiPromise, Keyring, WsProvider } from "@polkadot/api/mod.ts";
import { API_EXTENSIONS, API_RPC, API_TYPES } from "./api_options.ts";
import { ISubmittableResult } from "@polkadot/types/types/extrinsic.ts";
import { ethers } from "npm:ethers@5.4";

import ABI from "./abi/availbridge.json" with { type: "json" };

const AVAIL_RPC = "ws://127.0.0.1:9944";
const SURI = "//Alice";
const BRIDGE_ADDRESS = ""; // deployed bridge address
const DATA = ""; // data to send
const BRIDGE_API_URL = ""; // bridge api url
const ETH_PROVIDER_URL = ""; // eth provider url
const availApi = await ApiPromise.create({
	provider: new WsProvider(AVAIL_RPC),
	rpc: API_RPC,
	types: API_TYPES,
	signedExtensions: API_EXTENSIONS,
});
const account = new Keyring({ type: "sr25519" }).addFromUri(SURI);

/**
 *  ProofData represents a response from the api that holds proof for
 *  the blob verification.
 */
class ProofData {
	dataRootProof: Array<string>;
	leafProof: string;
	rangeHash: string;
	dataRootIndex: number;
	blobRoot: string;
	bridgeRoot: string;
	leaf: string;
	leafIndex: number;
}

/**
 * Submitting data to Avail as a transaction.
 *
 * @param availApi api instance
 * @param data payload to send
 * @param account that is sending transaction
 * @returns {Promise<unknown>}
 */
async function submitData(availApi, data, account) {
	return await new Promise<ISubmittableResult>((res) => {
		console.log("Sending transaction...");
		availApi.tx.dataAvailability.submitData(data).signAndSend(account, { nonce: -1 }, (result: ISubmittableResult) => {
			console.log(`Tx status: ${result.status}`);
			if (result.isError) {
				console.log(`Tx failed!`);
				res(result);
			}
			if (result.isInBlock) {
				console.log("Transaction in block, waiting for block finalization...");
			}
			if (result.isFinalized) {
				console.log(`Tx finalized.`);
				res(result);
			}
		});
	});
}

let result = await submitData(availApi, DATA, account);
if (result.isFinalized) {
	console.log(`DA transaction in finalized block: ${result.blockNumber}, transaction index: ${result.txIndex}`);
}

// wait until the chain head on the Ethereum network is updated with the block range
// in which the Avail DA transaction is included.
while (true) {
	let getHeadRsp = await fetch(BRIDGE_API_URL + "/avl/head");
	if (getHeadRsp.status != 200) {
		console.log("Something went wrong fetching the head.");
		break;
	}
	let headRsp = await getHeadRsp.json();
	let blockNumber: number = result.blockNumber.toNumber();
	let lastCommittedBlock: number = headRsp.data.end;
	if (lastCommittedBlock >= blockNumber) {
		console.log("Fetching the blob proof.");
		const proofResponse = await fetch(BRIDGE_API_URL + "/eth/proof/" + result.status.asFinalized + "?index=" + result.txIndex);
		if (proofResponse.status != 200) {
			console.log("Something went wrong fetching the proof.");
			console.log(proofResponse);
			break;
		}
		let proof: ProofData = await proofResponse.json();
		console.log("Proof fetched:");
		console.log(proof);
		// call the deployed contract verification function with the inclusion proof.
		const provider = new ethers.providers.JsonRpcProvider(ETH_PROVIDER_URL);
		const contractInstance = new ethers.Contract(BRIDGE_ADDRESS, ABI, provider);
		const isVerified = await contractInstance.verifyBlobLeaf([
			proof.dataRootProof,
			proof.leafProof,
			proof.rangeHash,
			proof.dataRootIndex,
			proof.blobRoot,
			proof.bridgeRoot,
			proof.leaf,
			proof.leafIndex,
		]);
		console.log(`Blob validation is: ${isVerified}`);
		break;
	}

	console.log("Waiting to bridge inclusion commitment. This can take a while...");
	// wait for 1 minute to check again
	await new Promise((f) => setTimeout(f, 60 * 1000));
}

Deno.exit(0);
