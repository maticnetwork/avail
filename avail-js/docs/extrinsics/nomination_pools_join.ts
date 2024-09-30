import { SDK, WaitFor, Keyring, BN } from "avail-js-sdk"

const main = async () => {
  const providerEndpoint = "ws://127.0.0.1:9944"
  const sdk = await SDK.New(providerEndpoint)

  // Input
  const account = new Keyring({ type: "sr25519" }).addFromUri("//Bob")
  const amount = new BN(10).pow(new BN(18)).mul(new BN(10000)) // 10_000 Avail
  const poolId = 1

  const result = await sdk.tx.nominationPools.join(amount, poolId, WaitFor.BlockInclusion, account)
  if (result.isErr) {
    console.log(result.reason)
    process.exit(1)
  }

  console.log(JSON.stringify(result, null, 2))
  process.exit()
}
main()
