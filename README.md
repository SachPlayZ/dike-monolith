# Dike Soroban Contracts

![Rust](https://img.shields.io/badge/Rust-2021-b7410e?style=flat-square&logo=rust)
![Soroban SDK](https://img.shields.io/badge/Soroban%20SDK-23-7d5fff?style=flat-square)
![Stellar](https://img.shields.io/badge/Stellar-Soroban-111827?style=flat-square)
![Status](https://img.shields.io/badge/status-testnet%20ready-f59e0b?style=flat-square)

Smart contracts for **Dike Protocol**, a modular binary prediction-market system on Stellar Soroban.

Dike markets use USDC/SAC collateral, YES/NO conditional positions, AMM trading, optimistic resolution, and a whitelisted Council of Dike dispute process. The contracts are written as a Rust workspace and designed to be tested, deployed, and audited module by module.

[Overview](#overview) - [Architecture](#architecture) - [Contracts](#contracts) - [Getting Started](#getting-started) - [Deployment](#deployment) - [Safety Notes](#safety-notes)

> [!IMPORTANT]
> This repository contains smart contracts for financial market infrastructure. Treat every deployment as experimental until it has been independently reviewed and tested with the exact role wiring, collateral contract, and network configuration you intend to use.

## Overview

Dike Protocol supports a curated v1 prediction-market flow:

1. An approved creator creates a binary YES/NO market.
2. USDC collateral backs complete YES/NO sets.
3. Traders buy or sell positions through the AMM.
4. After expiry, anyone can request resolution.
5. A proposer posts an outcome with a bond.
6. If undisputed, the outcome finalizes after the dispute window.
7. If disputed, the Council of Dike resolves by commit-reveal vote.
8. Winning or invalid-refund positions redeem through the vault.

Core design goals:

- **Solvency first**: no unbacked outcome positions.
- **Explicit state machines**: market and oracle status transitions are separated.
- **Modular custody**: collateral, positions, trading, oracle, council, and governance are distinct contracts.
- **Curated v1 scope**: market creation, collateral support, modules, fees, and council members are permissioned.

## Architecture

```mermaid
flowchart TD
    Creator["Approved Creator"] --> Factory["DikeMarketFactory"]
    Factory --> Registry["DikeMarketRegistry"]
    Factory --> AMM["DikeAMM"]
    AMM --> Tokens["DikeConditionalTokens"]
    AMM --> Vault["CollateralVault"]
    Trader["Trader / LP"] --> AMM
    Oracle["CODOracle"] --> Registry
    Oracle --> Vault
    Oracle --> Council["CouncilOfDike"]
    Council --> Oracle
    Vault --> Tokens
    Governance["DikeGovernance"] --> Timelock["DikeTimelock"]
    Governance --> Factory
    Governance --> Registry
    Governance --> Vault
    Governance --> AMM
    Governance --> Council
```

## Contracts

| Contract | Responsibility |
| --- | --- |
| `DikeMarketFactory` | Curated market creation, module coordination, initial AMM liquidity. |
| `DikeMarketRegistry` | Canonical market metadata, status machine, collateral support, final outcomes. |
| `DikeConditionalTokens` | Internal YES/NO position balances, complete-set minting, transfer checks, redemption burns. |
| `CollateralVault` | USDC/SAC custody, market accounting, redemptions, child-market credit, fees, oracle bonds. |
| `DikeAMM` | Fixed-product YES/NO pools, LP shares, buys, sells, liquidity removal. |
| `FeeManager` | Fee, reward, and bond calculation helpers. |
| `CODOracle` | Optimistic resolution requests, proposals, disputes, council escalation, finalization. |
| `CouncilOfDike` | Whitelisted commit-reveal voting for disputed markets. |
| `DikeGovernance` | Governed parameters, modules, supported collateral, council membership, upgrade hashes. |
| `DikeTimelock` | Queued, delayed, cancellable execution records for critical actions. |
| `mock_usdc` | Local/dev-only SEP-41-style collateral token. |

Shared crates:

| Crate | Purpose |
| --- | --- |
| `dike-types` | Shared contract types, statuses, config structs, errors, constants. |
| `dike-math` | Checked arithmetic, fee math, collateral limits, AMM quote helpers. |

## Repository Layout

```text
.
├── contracts/              # Soroban contract crates
├── crates/                 # Shared Rust crates
├── docs/DEPLOYMENT.md      # Deployment runbook
├── scripts/deploy-testnet.sh
├── scripts/local-flow.md   # Manual operator flow
├── deployments/            # Deployment manifests
├── PLAN.md                 # Protocol design plan
└── Cargo.toml              # Workspace manifest
```

## Getting Started

### Prerequisites

- Rust stable with `wasm32-unknown-unknown`
- Stellar CLI with Soroban support
- A local Stellar container or Testnet identity

```bash
rustup target add wasm32-unknown-unknown
cargo test
cargo build --release --target wasm32-unknown-unknown
```

> [!NOTE]
> This workspace currently uses `soroban-sdk = "23"`. If you upgrade the Stellar CLI template or SDK major version, update the workspace together and rerun the full contract suite.

### Run Tests

```bash
cargo test
cargo clippy --all-targets --all-features
cargo build --release --target wasm32-unknown-unknown
```

The test suite includes unit and integration-style Soroban tests for:

- Full production graph market creation, trading, resolution, and redemption.
- AMM tradeability, liquidity, cancellation, and child-market buy flows.
- Vault accounting, redemptions, child credit/debt repayment, and invalid refunds.
- Oracle proposal, dispute, council escalation, and finalization paths.
- Registry transition invariants and fee config validation.
- Timelock overflow and execution windows.

Soroban test snapshots live under each contract's `test_snapshots/` directory and should be reviewed when behavior changes.

## Deployment

For production-like deployments, use a real Stellar Asset Contract for USDC:

```bash
export NETWORK=testnet
export SOURCE=alice
export USDC_ISSUER=GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5
./scripts/deploy-testnet.sh
```

If you already know the SAC contract address:

```bash
export COLLATERAL_CONTRACT=<real-usdc-sac-contract-id>
./scripts/deploy-testnet.sh
```

For local-only simulations without a real SAC:

```bash
export NETWORK=local
export ALLOW_MOCK_USDC=true
./scripts/deploy-testnet.sh
```

The deployment script:

- Builds optimized Soroban WASM into `target/stellar`.
- Deploys or reuses contract aliases.
- Wires module roles across the registry, tokens, vault, AMM, oracle, council, and factory.
- Registers supported collateral and an approved market creator.
- Writes a manifest to `deployments/<network>.json`.

See [docs/DEPLOYMENT.md](docs/DEPLOYMENT.md) for the full runbook.

> [!WARNING]
> `mock_usdc` is for local/dev simulations only. Testnet and production-style deployments should use a real issuer-derived or explicitly supplied SAC contract.

## Market Lifecycle

```text
Created
  -> Live
  -> TradingClosed
  -> ResolutionRequested
  -> Proposed
  -> Resolved
```

Disputed markets follow:

```text
Proposed
  -> Disputed
  -> CouncilVoting
  -> Resolved
```

Cancelled markets allow LP position recovery and invalid-style redemption paths, but should not allow new trading.

## Safety Notes

- Verify the USDC/SAC contract address before enabling any market.
- Keep market creation curated in v1.
- Use timelock-controlled governance for critical module, fee, collateral, treasury, council, and upgrade changes.
- Keep `CollateralVault` under close size review; it is the largest contract because it owns custody, redemption, child credit, fees, and bonds.
- Consider splitting oracle bond custody into a separate `BondVault` before expanding the vault feature set further.
- Simulate Soroban transactions before signing and submitting.
- Run an independent security review before putting real liquidity at risk.

## Useful Commands

```bash
# Full local verification
cargo test
cargo clippy --all-targets --all-features
cargo build --release --target wasm32-unknown-unknown

# Start a local Stellar network
stellar container start local

# Fund a local identity
stellar keys generate alice --network local --fund

# Deploy to testnet with default testnet USDC issuer
NETWORK=testnet SOURCE=alice ./scripts/deploy-testnet.sh
```

## Further Reading

- [PLAN.md](PLAN.md) - protocol design and module responsibilities.
- [docs/DEPLOYMENT.md](docs/DEPLOYMENT.md) - deployment runbook and role wiring.
- [scripts/local-flow.md](scripts/local-flow.md) - manual local operator flow.
