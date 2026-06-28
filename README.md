# Dike Soroban Contracts

Greenfield Soroban implementation of the Dike Protocol prediction-market contract plan in `PLAN.md`.

## Modules

- `DikeMarketFactory`: curated market creation gate.
- `DikeMarketRegistry`: canonical market metadata, resolution rules, status machine, final outcomes.
- `DikeConditionalTokens`: internal YES/NO position balances.
- `CollateralVault`: USDC/SAC custody, market accounting, redemption, bonds, fees.
- `DikeAMM`: fixed-product YES/NO pools, LP shares, buy/sell paths.
- `FeeManager`: protocol fee, reward, and bond calculations.
- `CODOracle`: optimistic request/proposal/dispute/finalization flow.
- `CouncilOfDike`: whitelisted commit-reveal dispute voting.
- `DikeGovernance`: governed parameter/module/member state.
- `DikeTimelock`: queued/cancelled/executed critical actions.
- `mock_usdc`: local SEP-41-style collateral token for tests and local deployment.

## Build

```bash
cargo test
cargo build --release --target wasm32-unknown-unknown
```

The local Stellar CLI template in this environment uses `soroban-sdk = "23"`. If upgrading to Stellar CLI v27+, update the SDK and target to the current official template together.

## Local Deployment Shape

1. Set `USDC_ISSUER` to the real USDC issuer or `COLLATERAL_CONTRACT` to the real USDC Stellar Asset Contract address. Testnet defaults to issuer `GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5`, which derives to SAC `CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA`.
2. Deploy `dike_timelock`.
3. Deploy `dike_governance`.
4. Deploy `fee_manager`.
5. Deploy `market_registry`.
6. Deploy `conditional_tokens`.
7. Deploy `collateral_vault`.
8. Deploy `amm`.
9. Deploy `cod_oracle`.
10. Deploy `council_of_dike`.
11. Deploy `market_factory`.
12. Wire roles:
    - `factory` on registry and AMM.
    - `vault` on conditional tokens.
    - `tokens`, `amm`, `oracle`, `gov` on vault as needed.
    - `oracle`, `council`, `gov` across oracle/council/governance.
13. Add supported collateral and approved creators through governance/timelock for production-like flows.

For local/dev-only simulations, `scripts/deploy-testnet.sh` can deploy `mock_usdc` when `ALLOW_MOCK_USDC=true` and no real collateral issuer/contract is set. Testnet/mainnet-style deployments should pass a real issuer or SAC contract; the script now fails closed instead of silently wiring a mock.

## Network Notes

- Local passphrase: `Standalone Network ; February 2017`
- Testnet passphrase: `Test SDF Network ; September 2015`
- Production collateral should be the real USDC Stellar Asset Contract address.
- `mock_usdc` is only for local simulation and must not be wired into production-like deployments.
