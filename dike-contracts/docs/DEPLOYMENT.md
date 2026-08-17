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

## Contract TTL Operations

Soroban contract instances, executable WASM, and contract-data entries have ledger TTLs. Dike contract calls refresh instance storage and any persistent records they access to `518400` ledgers (about 30 days), but dormant deployments and records still need an operator keeper. See Stellar's [state archival](https://developers.stellar.org/docs/learn/fundamentals/contract-development/storage/state-archival) and [CLI instance-extension](https://developers.stellar.org/docs/tools/cli/cookbook/extend-contract-instance) guides.

The keeper is dry-run-only unless `--execute` is supplied. It reads only Dike-owned contract IDs from the deployment manifest; it deliberately does not maintain the external USDC SAC.

Preview mainnet renewal commands:

```bash
./scripts/maintain-contract-ttl.sh --network mainnet
```

After reviewing the manifest, network, source, fees, and generated commands, submit them with a funded Stellar identity:

```bash
./scripts/maintain-contract-ttl.sh \
  --network mainnet \
  --source ttl-keeper \
  --execute
```

Each deployed module uses one transaction. Stellar's instance-extension footprint includes the contract instance, all `env.storage().instance()` entries, and its executable WASM. Keep the exact deployed WASM artifacts in release storage as an independent deployment-recovery measure.

Run the dry-run in monitoring at least daily and execute renewal at least weekly. Alert on any manifest validation, simulation, submission, or missing-contract failure. Record the transaction hashes and resulting ledger targets in the operator log. `518400` ledgers is the default target; the current Stellar CLI maximum is `535679`. Do not assume wall-clock timing if ledger cadence changes.

### Dormant persistent records

The instance/WASM pass cannot discover or extend arbitrary enum-encoded persistent keys. Active Dike flows refresh records when read or written. For records that must survive longer inactivity, provide an exact key inventory as whitespace-separated fields:

```text
# contract_name durability key_xdr
market_registry persistent <base64-encoded-ScVal-DataKey>
collateral_vault persistent <base64-encoded-ScVal-DataKey>
```

Preview and then execute the inventory:

```bash
./scripts/maintain-contract-ttl.sh \
  --network mainnet \
  --keys-file ops/mainnet-ttl-keys.tsv

./scripts/maintain-contract-ttl.sh \
  --network mainnet \
  --keys-file ops/mainnet-ttl-keys.tsv \
  --source ttl-keeper \
  --execute
```

`key_xdr` must be the base64 XDR of the contract's exact `DataKey` `ScVal`, not application data. Keep the inventory synchronized when markets, pools, resolution requests, council cases, balances, LP positions, fees, or bonds are created. Temporary entries may be extended while active but cannot be recovered after expiry.

### Recovery

Use restore mode only after confirming entries are archived. It generates/submits Stellar CLI `contract restore` operations for each contract instance and optional persistent keys:

```bash
# Preview first.
./scripts/maintain-contract-ttl.sh \
  --network mainnet \
  --keys-file ops/mainnet-ttl-keys.tsv \
  --restore

# Submit after verifying the deployment manifest and key inventory.
./scripts/maintain-contract-ttl.sh \
  --network mainnet \
  --keys-file ops/mainnet-ttl-keys.tsv \
  --source ttl-keeper \
  --restore \
  --execute
```

Restore persistent entries before invoking business functions that depend on them. Never attempt to restore expired temporary entries.

Validate keeper behavior without network access:

```bash
./scripts/test-maintain-contract-ttl.sh
```

## Safety Checklist

- Verify USDC/SAC address before enabling any real market.
- Never wire `mock_usdc` outside local/dev-only runs.
- Keep market creation curated in v1.
- Use timelock for critical module, fee, collateral, treasury, council, and upgrade changes.
- Keep `CollateralVault` and `DikeConditionalTokens` non-upgradeable unless a separately audited migration plan exists.
- Simulate Soroban transactions before submission.
- Run the TTL keeper on schedule; preserve exact deployed WASM and persistent-key inventories for recovery.
