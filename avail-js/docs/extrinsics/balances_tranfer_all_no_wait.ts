import { SDK, Keyring } from "../../src/index"

const main = async () => {
  const providerEndpoint = "ws://127.0.0.1:9944"
  const sdk = await SDK.New(providerEndpoint)

  // Input
  const account = new Keyring({ type: "sr25519" }).addFromUri("//Alice")
  const dest = "5HGjWAeFDfFCWPsjFQdVV2Msvz2XtMktvgocEZcCj68kUMaw" // Eve
  const keepAlive = true

  const txHash = await sdk.tx.balances.transferAllNoWait(dest, keepAlive, account)

  console.log(JSON.stringify(txHash, null, 2))
  process.exit()
}
main()