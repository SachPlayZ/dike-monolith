# Dike Soroban Deployment Runbook

## Local Network

```bash
stellar container start local
stellar keys generate alice --network local --fund
```

Build contracts:

```bash
cargo build --release --target wasm32-unknown-unknown
```

For production-like deployments, configure the collateral explicitly. On Stellar testnet, the default USDC issuer is:

```bash
export USDC_ISSUER=GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5
```

The script derives the Stellar Asset Contract for `USDC:$USDC_ISSUER`; with the testnet issuer above, that SAC contract ID is:

```text
CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA
```

If you already know the SAC contract ID, pass it directly:

```bash
export COLLATERAL_CONTRACT=<real USDC/SAC contract address>
./scripts/deploy-testnet.sh
```

The testnet USDC SAC is already deployed, so the script uses the derived contract ID by default. Set `DEPLOY_SAC=true` only if you are wrapping a classic asset whose SAC does not exist yet.

The deployment script refuses to run without a SAC contract, an issuer to derive one from, or `ALLOW_MOCK_USDC=true` for local/dev-only simulations.

Deploy an individual local mock contract only for local testing:

```bash
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/mock_usdc.wasm \
  --source alice \
  --network local \
  -- \
  --admin alice
```

Repeat for every module, passing constructor arguments in the order defined by each contract.

## Required Role Wiring

Use each contract's `set_role` or governance/timelock application functions to configure:

- `factory`: `DikeMarketFactory`
- `registry`: `DikeMarketRegistry`
- `tokens`: `DikeConditionalTokens`
- `vault`: `CollateralVault`
- `amm`: `DikeAMM`
- `oracle`: `CODOracle`
- `council`: `CouncilOfDike`
- `gov`: `DikeGovernance`

The deployment script wires the concrete production call graph:

- Registry: `factory`, `oracle`, `gov`, supported collateral.
- Conditional tokens: `vault`, `amm`.
- Vault: `tokens`, `oracle`, `amm`, `gov`, `registry`.
- AMM: `factory`, `gov`, plus `vault`, `tokens`, real collateral, and `registry` through `set_modules`.
- Oracle: `gov`, `council`, `registry`, `vault`.
- Council: `gov`, `oracle`.
- Factory: `registry`, `tokens`, `vault`, `amm`, `fee_manager`, approved creator, supported collateral.

## Safety Checklist

- Verify USDC/SAC address before enabling any real market.
- Never wire `mock_usdc` outside local/dev-only runs.
- Keep market creation curated in v1.
- Use timelock for critical module, fee, collateral, treasury, council, and upgrade changes.
- Keep `CollateralVault` and `DikeConditionalTokens` non-upgradeable unless a separately audited migration plan exists.
- Simulate Soroban transactions before submission.
