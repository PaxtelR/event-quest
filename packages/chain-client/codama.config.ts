// Regenerates the typed @solana/kit client from the eventquest program's
// IDL. Run via `pnpm generate` after any program change (after `anchor
// build` + copying target/idl/eventquest.json to idls/eventquest.json —
// see crates/eventquest-chain/src/lib.rs for the same regeneration step on
// the Rust side). See docs/adr/ADR-002-solana-client.md.
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { createFromRoot } from "codama";
import { rootNodeFromAnchor } from "@codama/nodes-from-anchor";
import { renderVisitor } from "@codama/renderers-js";

const idlPath = fileURLToPath(new URL("../../idls/eventquest.json", import.meta.url));
const idl = JSON.parse(readFileSync(idlPath, "utf8"));

const codama = createFromRoot(rootNodeFromAnchor(idl));

await codama.accept(renderVisitor("."));

console.log("Generated TypeScript client: packages/chain-client/src/generated");
