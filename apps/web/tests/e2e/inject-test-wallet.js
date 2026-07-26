// Injected into the Playwright browser context via `page.addInitScript`.
// A real Wallet Standard implementation — not a mock of the app's wallet
// logic — backed by a genuine Ed25519 keypair generated with the
// browser's own WebCrypto (`crypto.subtle`, available because Playwright
// serves the app over http://localhost, a secure context). It signs real
// message/transaction bytes and submits real transactions to the local
// Surfnet RPC; the only thing "faked" is the human clicking "approve" in
// an extension popup, which no CI environment can do anyway. This is the
// standard approach for automated Solana dApp E2E tests.
//
// No bundler, no imports: everything this needs (base58, Solana's
// shortvec/transaction-wire-format, Ed25519 via WebCrypto) is small enough
// to hand-roll in plain JS, so this file ships as-is with no build step.
(function () {
  const CHAIN = "solana:devnet";
  const RPC_URL = "http://127.0.0.1:8899";
  const BASE58_ALPHABET = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

  function base58Encode(bytes) {
    const digits = [0];
    for (let i = 0; i < bytes.length; i++) {
      let carry = bytes[i];
      for (let j = 0; j < digits.length; j++) {
        carry += digits[j] << 8;
        digits[j] = carry % 58;
        carry = (carry / 58) | 0;
      }
      while (carry > 0) {
        digits.push(carry % 58);
        carry = (carry / 58) | 0;
      }
    }
    let leadingZeros = 0;
    for (let i = 0; i < bytes.length && bytes[i] === 0; i++) leadingZeros++;
    let result = "";
    for (let i = 0; i < leadingZeros; i++) result += "1";
    for (let i = digits.length - 1; i >= 0; i--) result += BASE58_ALPHABET[digits[i]];
    return result;
  }

  function base58Decode(str) {
    const bytes = [0];
    for (let i = 0; i < str.length; i++) {
      const value = BASE58_ALPHABET.indexOf(str[i]);
      if (value === -1) throw new Error("invalid base58 character in: " + str);
      let carry = value;
      for (let j = 0; j < bytes.length; j++) {
        carry += bytes[j] * 58;
        bytes[j] = carry & 0xff;
        carry >>= 8;
      }
      while (carry > 0) {
        bytes.push(carry & 0xff);
        carry >>= 8;
      }
    }
    for (let i = 0; i < str.length && str[i] === "1"; i++) bytes.push(0);
    return new Uint8Array(bytes.reverse());
  }

  function decodeShortU16(bytes, offset) {
    let len = 0;
    let size = 0;
    for (;;) {
      const elem = bytes[offset + size];
      len |= (elem & 0x7f) << (size * 7);
      size += 1;
      if ((elem & 0x80) === 0) break;
    }
    return [len, size];
  }

  function bytesEqual(a, b) {
    if (a.length !== b.length) return false;
    for (let i = 0; i < a.length; i++) if (a[i] !== b[i]) return false;
    return true;
  }

  const STORAGE_KEY = "__e2e_test_wallet_jwk__";

  let cryptoKeyPair = null;
  let publicKeyBytes = null;
  let address = null;

  // `addInitScript` re-runs this whole IIFE on every navigation, which
  // would otherwise mint a fresh keypair (and thus a "different wallet")
  // on every page load — persisting the private key material to
  // `localStorage` (per-origin, per-browser-context — so two Playwright
  // contexts naturally get two independent wallets) keeps identity
  // stable across navigations within one test, exactly like a real
  // browser-extension wallet does.
  async function ensureKeyPair() {
    if (cryptoKeyPair) return;

    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored) {
      const jwk = JSON.parse(stored);
      const privateKey = await crypto.subtle.importKey("jwk", jwk, { name: "Ed25519" }, true, ["sign"]);
      const publicJwk = Object.assign({}, jwk);
      delete publicJwk.d;
      publicJwk.key_ops = [];
      const publicKey = await crypto.subtle.importKey("jwk", publicJwk, { name: "Ed25519" }, true, []);
      cryptoKeyPair = { privateKey: privateKey, publicKey: publicKey };
    } else {
      cryptoKeyPair = await crypto.subtle.generateKey({ name: "Ed25519" }, true, ["sign"]);
      const jwk = await crypto.subtle.exportKey("jwk", cryptoKeyPair.privateKey);
      localStorage.setItem(STORAGE_KEY, JSON.stringify(jwk));
    }

    const raw = await crypto.subtle.exportKey("raw", cryptoKeyPair.publicKey);
    publicKeyBytes = new Uint8Array(raw);
    address = base58Encode(publicKeyBytes);
    window.__E2E_WALLET_ADDRESS__ = address;
  }

  async function signRawBytes(bytes) {
    const signature = await crypto.subtle.sign("Ed25519", cryptoKeyPair.privateKey, bytes);
    return new Uint8Array(signature);
  }

  // The first `numRequiredSignatures` account keys correspond 1:1, in
  // order, with the transaction's signature slots (Solana wire format).
  function findSignerIndex(message) {
    const numRequiredSignatures = message[0];
    let offset = 3; // header: numRequiredSignatures, numReadonlySigned, numReadonlyUnsigned
    const [, keyCountSize] = decodeShortU16(message, offset);
    offset += keyCountSize;
    for (let i = 0; i < numRequiredSignatures; i++) {
      const key = message.slice(offset + i * 32, offset + i * 32 + 32);
      if (bytesEqual(key, publicKeyBytes)) return i;
    }
    return -1;
  }

  async function signTransactionBytes(txBytes) {
    const bytes = new Uint8Array(txBytes);
    const [numSignatures, sigCountSize] = decodeShortU16(bytes, 0);
    const signaturesStart = sigCountSize;
    const messageStart = signaturesStart + numSignatures * 64;
    const message = bytes.slice(messageStart);
    const signerIndex = findSignerIndex(message);
    if (signerIndex === -1) {
      throw new Error("E2E test wallet is not a required signer of this transaction");
    }
    const signature = await signRawBytes(message);
    bytes.set(signature, signaturesStart + signerIndex * 64);
    return bytes;
  }

  function bytesToBase64(bytes) {
    let binary = "";
    for (let i = 0; i < bytes.length; i++) binary += String.fromCharCode(bytes[i]);
    return btoa(binary);
  }

  const changeListeners = [];

  const wallet = {
    version: "1.0.0",
    name: "E2E Test Wallet",
    icon: "data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciLz4=",
    chains: [CHAIN],
    accounts: [],
    features: {
      "standard:connect": {
        version: "1.0.0",
        connect: async () => {
          await ensureKeyPair();
          wallet.accounts = [buildAccount()];
          // `@wallet-standard/react`'s wallet registry caches each
          // wallet's `accounts` and only refreshes that cache in
          // response to a "change" event — without emitting one here,
          // `useSelectedWalletAccount()` resolves the just-selected
          // account against a stale (empty) cached `accounts` array and
          // silently comes back `undefined`.
          for (const listener of changeListeners) listener({ accounts: wallet.accounts });
          return { accounts: wallet.accounts };
        },
      },
      "standard:disconnect": {
        version: "1.0.0",
        disconnect: async () => {},
      },
      "standard:events": {
        version: "1.0.0",
        on: (event, listener) => {
          if (event !== "change") return () => {};
          changeListeners.push(listener);
          return () => {
            const index = changeListeners.indexOf(listener);
            if (index !== -1) changeListeners.splice(index, 1);
          };
        },
      },
      "solana:signMessage": {
        version: "1.0.0",
        signMessage: async (...inputs) => {
          const outputs = [];
          for (const input of inputs) {
            const signature = await signRawBytes(input.message);
            outputs.push({ signedMessage: input.message, signature: signature });
          }
          return outputs;
        },
      },
      "solana:signTransaction": {
        version: "1.0.0",
        signTransaction: async (...inputs) => {
          const outputs = [];
          for (const input of inputs) {
            outputs.push({ signedTransaction: await signTransactionBytes(input.transaction) });
          }
          return outputs;
        },
      },
      "solana:signAndSendTransaction": {
        version: "1.0.0",
        signAndSendTransaction: async (...inputs) => {
          const outputs = [];
          for (const input of inputs) {
            const signed = await signTransactionBytes(input.transaction);
            const response = await fetch(RPC_URL, {
              method: "POST",
              headers: { "Content-Type": "application/json" },
              body: JSON.stringify({
                jsonrpc: "2.0",
                id: 1,
                method: "sendTransaction",
                params: [
                  bytesToBase64(signed),
                  { encoding: "base64", skipPreflight: false, preflightCommitment: "confirmed" },
                ],
              }),
            });
            const json = await response.json();
            if (json.error) {
              throw new Error(json.error.message || "sendTransaction failed");
            }
            outputs.push({ signature: base58Decode(json.result) });
          }
          return outputs;
        },
      },
    },
  };

  function registerWallet(walletToRegister) {
    const callback = (api) => api.register(walletToRegister);
    try {
      window.dispatchEvent(
        new CustomEvent("wallet-standard:register-wallet", { detail: callback }),
      );
    } catch {
      // no-op: some environments disallow constructing CustomEvent early.
    }
    window.addEventListener("wallet-standard:app-ready", (event) => callback(event.detail));
  }

  function buildAccount() {
    return {
      address: address,
      publicKey: publicKeyBytes,
      chains: [CHAIN],
      features: ["solana:signMessage", "solana:signTransaction", "solana:signAndSendTransaction"],
    };
  }

  // `addInitScript` re-runs this whole file on every navigation (see the
  // comment on `ensureKeyPair` above), which resets `wallet.accounts` to
  // `[]` on every fresh page load — unlike a real browser-extension
  // wallet, which stays alive across navigations and silently reports
  // "already connected" for a site it previously authorized. To match
  // that behavior (and let `SelectedWalletAccountContextProvider`'s own
  // restore-from-localStorage effect find this wallet's account again),
  // eagerly restore the keypair from storage *before* registering,
  // whenever this wallet has connected before.
  async function init() {
    if (localStorage.getItem(STORAGE_KEY)) {
      await ensureKeyPair();
      wallet.accounts = [buildAccount()];
    }
    registerWallet(wallet);
  }

  init();
})();
