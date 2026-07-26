## Description
<!-- Describe your changes -->

## Checklist

### Code quality
- [ ] `cargo fmt --check` passes
- [ ] `cargo clippy --workspace --all-targets -- -W clippy::all -D warnings` passes
- [ ] No `unwrap()`/`expect()` added to program code (tests are fine)
- [ ] All arithmetic uses checked operations (`checked_add`/`checked_sub`/etc.)

### Testing
- [ ] `cargo test --workspace` passes
- [ ] `pnpm test:ts` passes
- [ ] New instructions/handlers have Mollusk test coverage (happy path + negative cases)

### Security (if this touches `programs/eventquest`)
- [ ] All accounts validated (owner, signer, PDA with a stored canonical bump)
- [ ] CPI targets validated, accounts reloaded after any CPI that modifies them
- [ ] `cargo audit` passes (or the new advisory is added to `.cargo/audit.toml` with a justification in `docs/SECURITY.md`)

### Docs
- [ ] README / `docs/` updated if behavior, setup, or deployment changed
- [ ] `docs/SECURITY.md` updated if this resolves or adds a deferred finding

## Type of change
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Security fix
- [ ] Docs/CI only
