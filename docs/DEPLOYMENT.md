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

Deploy a contract:

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

## Safety Checklist

- Verify USDC/SAC address before enabling any real market.
- Keep market creation curated in v1.
- Use timelock for critical module, fee, collateral, treasury, council, and upgrade changes.
- Keep `CollateralVault` and `DikeConditionalTokens` non-upgradeable unless a separately audited migration plan exists.
- Simulate Soroban transactions before submission.
