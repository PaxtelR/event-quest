// Funds a fresh Devnet test wallet with a direct SOL transfer from the
// already-funded attestor keypair, rather than the public faucet — the
// same reasoning as scripts/devnet-smoke-test.ts's step 3: the faucet is
// rate-limited enough to make repeat automated runs unreliable, while a
// plain system transfer between two real Devnet accounts is itself a real,
// unmocked transaction.
import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  address,
  appendTransactionMessageInstructions,
  createKeyPairSignerFromBytes,
  createSolanaRpc,
  createSolanaRpcSubscriptions,
  createTransactionMessage,
  getSignatureFromTransaction,
  lamports,
  pipe,
  sendAndConfirmTransactionFactory,
  setTransactionMessageFeePayerSigner,
  setTransactionMessageLifetimeUsingBlockhash,
  signTransactionMessageWithSigners,
  type Address,
} from "@solana/kit";
import { getTransferSolInstruction } from "@solana-program/system";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const ROOT_DIR = path.resolve(__dirname, "../../../..");

export async function fundDevnetWallet(recipientAddress: string, lamportsAmount: bigint): Promise<string> {
  const rpcHttpUrl = process.env.E2E_RPC_URL ?? "https://api.devnet.solana.com";
  const rpcWsUrl = process.env.E2E_RPC_WS_URL ?? "wss://api.devnet.solana.com";
  const keypairPath = process.env.ATTESTOR_KEYPAIR_PATH ?? ".secrets/attestor-devnet.json";
  const resolvedPath = path.isAbsolute(keypairPath) ? keypairPath : path.join(ROOT_DIR, keypairPath);

  const raw = JSON.parse(await readFile(resolvedPath, "utf8")) as number[];
  const funder = await createKeyPairSignerFromBytes(new Uint8Array(raw));

  const rpc = createSolanaRpc(rpcHttpUrl);
  const rpcSubscriptions = createSolanaRpcSubscriptions(rpcWsUrl);
  const sendAndConfirm = sendAndConfirmTransactionFactory({ rpc, rpcSubscriptions });

  const { value: latestBlockhash } = await rpc.getLatestBlockhash().send();
  const message = pipe(
    createTransactionMessage({ version: 0 }),
    (tx) => setTransactionMessageFeePayerSigner(funder, tx),
    (tx) => setTransactionMessageLifetimeUsingBlockhash(latestBlockhash, tx),
    (tx) =>
      appendTransactionMessageInstructions(
        [
          getTransferSolInstruction({
            source: funder,
            destination: address(recipientAddress) as Address,
            amount: lamports(lamportsAmount),
          }),
        ],
        tx,
      ),
  );
  const signedTx = await signTransactionMessageWithSigners(message);
  const signature = getSignatureFromTransaction(signedTx);
  // `sendAndConfirmTransactionFactory`'s parameter type only narrows
  // correctly when the whole `pipe(...)` chain's intermediate types are
  // annotated explicitly, which the official @solana/kit examples don't
  // bother with either (their equivalent helper types the transaction
  // message as `any`) — `setTransactionMessageLifetimeUsingBlockhash`
  // above is what actually guarantees this has a blockhash lifetime.
  await sendAndConfirm(signedTx as Parameters<typeof sendAndConfirm>[0], { commitment: "confirmed" });
  return signature;
}
