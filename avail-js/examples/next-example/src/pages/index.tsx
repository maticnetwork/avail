import { MouseEventHandler, useState } from "react";
import { ApiPromise, initialize, signedExtensions, types } from "avail-js-sdk";
import { isNumber } from "@polkadot/util";
import { SignerOptions } from "@polkadot/api/types";
import Head from "next/head";

// Define the structure of an extension
interface ExtensionInfo {
  version: string;
  enable: () => Promise<void>;
}

// Define the structure of found extensions
interface FoundExtensions {
  [extensionName: string]: ExtensionInfo;
}

// Extend the Window interface to include `injectedWeb3`
interface Web3Window extends Window {
  injectedWeb3?: FoundExtensions;
}

export default function Home() {
  // State to store found extensions
  const [foundExtensions, setFoundExtensions] = useState<FoundExtensions>({});

  // State to track which extensions have been initialized
  const [extensionsInitialized, setExtensionsInitialized] = useState<Record<string, boolean>>({});

  // State to store the Avail API instance
  const [availApi, setAvailApi] = useState<ApiPromise | undefined>();

  // State to store logs for UI display
  const [logs, setLogs] = useState<{ message: string; severity: "info" | "error" }[]>([]);

  // Function to detect and enable extensions
  const findExtension = async () => {
    try {
      // Dynamically import the `web3Enable` function from the Polkadot extension library
      const { web3Enable } = await import("@polkadot/extension-dapp");
      await web3Enable("Example with extension");

      // Cast the global `window` object to our custom `Web3Window` interface
      const web3Window = window as Web3Window;

      // Check if extensions are available and update the state
      if (web3Window.injectedWeb3) {
        setFoundExtensions(web3Window.injectedWeb3);
      } else {
        addLogs("No extensions found", "info");
      }
    } catch (err) {
      // Handle errors and log them
      addLogs(`Failed to find extensions: ${err.message}`, "error");
    }
  };

  // Function to generate metadata for the injector
  const getInjectorMetadata = (api: ApiPromise) => {
    return {
      chain: api.runtimeChain.toString(),
      specVersion: api.runtimeVersion.specVersion.toNumber(),
      tokenDecimals: api.registry.chainDecimals[0] || 18,
      tokenSymbol: api.registry.chainTokens[0] || "AVAIL",
      genesisHash: api.genesisHash.toHex(),
      ss58Format: isNumber(api.registry.chainSS58) ? api.registry.chainSS58 : 0,
      chainType: "substrate" as "substrate",
      icon: "substrate",
      types: types as any,
      userExtensions: signedExtensions,
    };
  };

  // Function to send a transaction using a specific extension
  const sendTx = async (extension: string) => {
    try {
      // Dynamically import functions from the Polkadot extension library
      const { web3Accounts, web3FromSource } = await import("@polkadot/extension-dapp");

      // Initialize the API if it's not already connected
      let api = availApi;
      if (!(api && api.isConnected)) {
        api = await initialize("ws://127.0.0.1:9944");
        setAvailApi(api);
      }

      // Fetch accounts and filter by the selected extension
      const accounts = await web3Accounts();
      const filteredAccounts = accounts.filter((x) => x.meta.source === extension);
      if (filteredAccounts.length === 0) throw new Error("No account found");

      // Select the first account that starts with "5CDG" or fallback to the first account
      const account = filteredAccounts.find((x) => x.address.startsWith("5CDG")) || filteredAccounts[0];
      const injector = await web3FromSource(account.meta.source);

      // Provide metadata to the injector if it hasn't been initialized
      if (injector.metadata && !extensionsInitialized[extension]) {
        const metadata = getInjectorMetadata(api);
        await injector.metadata.provide(metadata);
        setExtensionsInitialized((prev) => ({ ...prev, [injector.name]: true }));
      }

      // Create and send the transaction
      const tx = api.tx.dataAvailability.submitData("0x123456");
      addLogs(`Sending tx with account ${account.address} and wallet ${extension}`, "info");

      await tx.signAndSend(
        account.address,
        { signer: injector.signer, app_id: 1 } as Partial<SignerOptions>,
        ({ status, isError, events }) => {
          if (isError) {
            addLogs("An error has occurred, open console to view logs", "error");
            console.log(events);
          }
          if (status.isInBlock) {
            addLogs(`Transaction included in block: ${status.asInBlock}`, "info");
          }
        }
      );
    } catch (err) {
      // Handle errors and log them
      addLogs(err.message ? err.message : "An unexpected error occurred", "error");
    }
  };

  // Function to add logs to the state
  const addLogs = (message: string, severity: "info" | "error") => {
    setLogs((prevLogs) => {
      const newLogs = [...prevLogs, { message, severity }];
      return newLogs.slice(-50); // Keep only the last 50 logs
    });
  };

  // Component to display logs in a table
  const LogsDisplay = () => {
    return (
      <div style={{ marginTop: "24px", maxHeight: "300px", overflowY: "auto" }}>
        <table style={{ width: "100%", borderCollapse: "collapse" }}>
          <tbody>
            {logs.map((log, index) => (
              <tr key={index}>
                <td
                  style={{
                    border: "1px solid black",
                    padding: "8px",
                    color: log.severity === "error" ? "red" : "white",
                    backgroundColor: log.severity === "error" ? "#ffebee" : "#e8f5e9",
                  }}
                >
                  {log.message}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    );
  };

  // Reusable Button component
  interface ButtonProps {
    onClick: MouseEventHandler<HTMLButtonElement>;
    label: string;
    style?: React.CSSProperties;
  }

  const Button = ({ onClick, label, style }: ButtonProps) => {
    return (
      <button
        style={{
          marginTop: "12px",
          border: "1px solid white",
          padding: "6px",
          borderRadius: "8px",
          width: "240px",
          ...style,
        }}
        onClick={onClick}
      >
        {label}
      </button>
    );
  };

  return (
    <>
      <Head>
        <title>Avail test app</title>
        <meta name="description" content="A simple app to test connection with polkadot js" />
      </Head>
      <div style={{ display: "flex", alignItems: "center", justifyContent: "center", flexDirection: "column" }}>
        <h1 style={{ fontSize: "32px" }}>Use extension</h1>

        {/* Button to detect extensions */}
        <Button onClick={() => findExtension()} label={"Detect extensions"} />

        {/* Buttons to send transactions with detected extensions */}
        {Object.keys(foundExtensions).map((extension, i) => {
          return <Button key={i} onClick={() => sendTx(extension)} label={`Send TX with ${extension}`} />;
        })}

        {/* Display logs if there are any */}
        {logs.length > 0 && <LogsDisplay />}
      </div>
    </>
  );
}
