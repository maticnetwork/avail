import { SDK, Keyring, Events } from "../../src/sdk"

export async function run() {
  console.log("SubmitData")
  await SubmitData.run()
  console.log("CreateApplicationKey")
  await CreateApplicationKey.run()
}

namespace SubmitData {
  export async function run() {
    const sdk = await SDK.New(SDK.localEndpoint())

    const account = new Keyring({ type: "sr25519" }).addFromUri("//Alice")
    const data = "My Data"
    const options = { app_id: 1 }

    const tx = sdk.tx.dataAvailability.submitData(data)
    const result = await tx.execute_wait_for_inclusion(account, options)
    if (result.isErr()) throw Error(result.error.reason)
    const details = result.value

    details.printDebug()
    let event = details.findFirstEvent(Events.DataAvailability.DataSubmitted)
    if (event != null) {
      console.log(event)
    }
  }
}

namespace CreateApplicationKey {
  export async function run() {
    const sdk = await SDK.New(SDK.localEndpoint())

    const account = new Keyring({ type: "sr25519" }).addFromUri("//Alice")
    const key = "My Key"

    const tx = sdk.tx.dataAvailability.createApplicationKey(key)
    const result = await tx.execute_wait_for_inclusion(account)
    if (result.isErr()) throw Error(result.error.reason)
    const details = result.value

    details.printDebug()
    let event = details.findFirstEvent(Events.DataAvailability.ApplicationKeyCreated)
    if (event != null) {
      console.log(event)
    }
  }
}