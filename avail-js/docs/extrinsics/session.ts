import { SDK, Keyring, sdkUtil } from "../../src/sdk"

export async function run() {
  console.log("SetKeys")
  await SetKeys.run()
}

namespace SetKeys {
  export async function run() {
    const sdk = await SDK.New(SDK.localEndpoint())

    const account = new Keyring({ type: "sr25519" }).addFromUri("//Alice")
    const keysBytes = await sdk.api.rpc.author.rotateKeys()
    const keys = sdkUtil.deconstruct_session_keys(keysBytes.toString())

    const tx = sdk.tx.session.setKeys(keys)
    const result = await tx.execute_wait_for_inclusion(account)
    if (result.isErr()) throw Error(result.error.reason)
    const details = result.value

    details.printDebug()
  }
}