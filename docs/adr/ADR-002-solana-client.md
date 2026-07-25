# ADR-002: Solana client strategy — `@solana/kit` + Codama-generated client

## Status

Accepted

## Context

Spec §15.1 requires the frontend to use `@solana/kit` (web3.js 2.0) rather
than `@solana/web3.js` v1, and explicitly forbids adding v1 "without proven
necessity." The Anchor program (`programs/eventquest`) produces an IDL via
`anchor build`, and Anchor's default TypeScript client (`@coral-xyz/anchor`'s
`Program` class) is built around `@solana/web3.js` v1 types
(`Connection`, `PublicKey`, `Transaction`), which are not directly compatible
with `@solana/kit`'s address/transaction/signer types.

Mixing representations of public keys, transactions, signers, and RPC clients
across the codebase without a single adapter layer is explicitly disallowed
by spec §15.1 ("não misturar representações... sem adapters centralizados e
testados").

## Decision

1. Generate the TypeScript client from the Anchor IDL using **Codama**
   (`/generate-idl-client`, per the kit's own playbook and spec §15.1),
   targeting `@solana/kit`-compatible output, into a single shared package:
   `packages/chain-client`.
2. `apps/web` and any TypeScript test/tooling code import exclusively from
   `packages/chain-client` for account layouts, instruction builders, and PDA
   derivation — no hand-written duplicate account decoders anywhere else in
   the TypeScript codebase.
3. On the Rust side, `apps/api` and `apps/indexer` share `crates/eventquest-chain`
   for the equivalent account layouts, instruction building (for the backend's
   attestor co-signing step), and event decoding — again, a single shared
   crate rather than duplicated struct definitions per service.
4. `@coral-xyz/anchor`'s TS client is used only as an internal implementation
   detail during IDL generation tooling if needed; it is never imported by
   application code in `apps/web`.
5. `@solana/web3.js` v1 is not added to `apps/web`'s dependencies. If a
   specific library dependency later forces a v1 type at some integration
   boundary, that boundary must go through an explicit, tested adapter
   function in `packages/chain-client`, not ad hoc casts scattered through
   feature code.

## Consequences

- Every program change (new instruction, changed account layout) requires
  regenerating `packages/chain-client` (`/generate-idl-client`) and
  `crates/eventquest-chain` before frontend/backend code depending on it will
  compile — this is intended: it makes IDL drift a compile-time error instead
  of a runtime bug.
- The frontend gets tree-shakable, modern types and does not pull in the
  legacy `Buffer`-heavy `@solana/web3.js` v1 dependency tree.
- Slightly more setup cost up front (Codama pipeline configuration) in
  exchange for a single, testable adapter boundary instead of scattered
  manual type conversions.
