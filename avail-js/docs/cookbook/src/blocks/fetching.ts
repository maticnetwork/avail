import { SDK, Account, Block, WaitFor } from "./../../../../src/index"

const main = async () => {
  const sdk = await SDK.New(SDK.localEndpoint())
  const api = sdk.api
  const alice = SDK.alice()

  // Fetching latest block via rpc call
  const block = await api.rpc.chain.getBlock()
  console.log(block.block.header.hash.toHex()) // `0xce61bfcb24ae953ec22d810520eb9b0c9d093507621bea74fc759fc981df5dbd`

  // Fetching finalized block (or any specific block)
  const finalizedHash = await api.rpc.chain.getFinalizedHead()
  const block2 = await api.rpc.chain.getBlock(finalizedHash)
  console.log(block2.block.header.hash.toHex()) // `0x3d3983fb1d931fb8ec17a623d331fe6a4310f0761c8d6c91da048c1a8eb7aa9f`

  // Fetching block that contains our transaction via SDK
  const data = "MyData"
  const mtx = await sdk.tx.dataAvailability.submitData(data, WaitFor.BlockInclusion, alice)
  const tx = mtx._unsafeUnwrap()
  const block3 = await api.rpc.chain.getBlock(tx.details.blockHash)
  console.log(block3.block.header.hash.toHex()) // `0xb41d3d37ac7449954956d0a7c5f607a0e10b1a30e280e5a05500b10ded5501ce`

  // Fetching block that contains our transaction via Account instance
  const account = new Account(sdk, alice)
  const tx2 = (await account.submitData(data))._unsafeUnwrap()
  const block4 = await api.rpc.chain.getBlock(tx2.details.blockHash) // `0xdf8413c48952204bab1c81371d58c1ef17da2fd1680d6d94a0ffab9492b58519`
  console.log(block4.block.header.hash.toHex())

  // Fetching block that contains our transaction via Block instance
  const block5 = await Block.New(api, tx2.details.blockHash)
  console.log(block5.signedBlock.block.header.hash.toHex()) // `0xdf8413c48952204bab1c81371d58c1ef17da2fd1680d6d94a0ffab9492b58519`

  process.exit()
}
main()