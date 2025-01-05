# Avail SDK documentation / examples

This directory contains documentation and examples related to the opinionated SDK and additional functionalities provided by the `avail-js-sdk`.

## Requirements

- Node.js: v20.11.1 or higher
- npm: v10.2.4 or higher
- ts-node: v10.9.2 or higher

## Running Examples in the Docs Directory

To run the examples provided in the `docs/extrinsics` directory, follow these steps:

1. Install `ts-node` globally if you haven't already:

```bash
npm i -g ts-node
```

2. From the avail-js/docs folder, install all necessary dependencies:

```bash
npm install
```

3. Start a local Avail node. You have two options:

   a. Build and run from source:
   ```bash
   cargo build --release
   ./target/release/avail-node --dev
   ```

   b. Download and run the latest release:
   - Download from [Github Releases](https://github.com/availproject/avail/releases)
   - Extract and run the binary:
   ```bash
   ./avail-node --dev
   ```

4. Run example scripts:

```bash
# General format
ts-node ./docs/extrinsics/NAME_OF_THE_FILE.ts

# Example
ts-node ./docs/extrinsics/staking_nominate.ts
```

## Available Examples

The following examples are available in the `docs/extrinsics` directory:

- `staking_nominate.ts` - Demonstrates staking and nomination
- `session.ts` - Session management examples
- `multisig.ts` - Multi-signature transaction examples
- `nomination_pools.ts` - Nomination pool interactions
- `da.ts` - Data Availability examples

Each example demonstrates how to interact with different aspects of the Avail network using the avail-js-sdk.

## Troubleshooting

If you encounter any issues:

1. Ensure your Node.js version matches the requirements
2. Clear the node_modules folder and run `npm install` again
3. Check that your local Avail node is running and accessible
4. Verify your network connectivity

For more detailed information, visit the [Avail Documentation](https://docs.availproject.org/)
