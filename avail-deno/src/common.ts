import type { ApiPromise } from "@polkadot/api";
import { err, ok } from "@neverthrow";
import type { Result } from "@neverthrow";

export function fromHexToAscii(hex: string): string {
	let str = "";
	for (let n = 0; n < hex.length; n += 2) {
		str += String.fromCharCode(parseInt(hex.substring(n, n + 2), 16));
	}

	return `${str}`;
}

export function decodeError(api: ApiPromise, error: any): string {
	if (!error.isModule) {
		return error.toString();
	}

	const { docs, method, section } = api.registry.findMetaError(error.asModule);
	return `${section}.${method}: ${docs.join(" ")}`;
}

export function commissionNumberToPerbill(value: number): Result<string, string> {
	if (!Number.isInteger(value)) {
		return err("Commission cannot have decimal place. It needs to be a whole number.");
	}

	if (value < 0 || value > 100) {
		return err("Commission is limited to the following range: 0 - 100. It cannot be less than 0 or more than 100.");
	}

	let commission = value.toString().concat("0000000");
	// For some reason 0 commission is not defined as "0" but as "1".
	if (commission == "00000000") {
		commission = "1";
	}

	return ok(commission);
}
