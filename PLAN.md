# Dike Web Implementation Plan

## Summary

`dike-web` is the user-facing Dike Protocol application. It should be a Next.js TypeScript App Router project that reads indexed protocol state from `dike-services`, refreshes action-critical contract state from Stellar RPC, and submits user-signed Soroban transactions through Freighter or Stellar Wallets Kit.

The frontend must treat `/Users/sachplayz/Projects/Dike_Stellar/dike-contracts` as the canonical source for contract interfaces, deployment manifests, enum names, and market lifecycle rules. The current testnet manifest lives at:

```text
/Users/sachplayz/Projects/Dike_Stellar/dike-contracts/deployments/testnet.json
```

The application must never custody private keys. All user transactions are built, simulated, signed by the connected wallet, submitted to Stellar RPC, and then reconciled through `dike-services`.

## Contract Context

Use these deployed testnet contracts from the manifest:

| Module                | Manifest Key         | Frontend Responsibility                                                     |
| --------------------- | -------------------- | --------------------------------------------------------------------------- |
| DikeMarketFactory     | `market_factory`     | Curated market creation and initial liquidity flow.                         |
| DikeMarketRegistry    | `market_registry`    | Canonical market data, status, expiry, final outcome, request id.           |
| DikeConditionalTokens | `conditional_tokens` | YES/NO balances, transfers, backing, burns through vault flows.             |
| CollateralVault       | `collateral_vault`   | User deposits, root stake, child credit/debt, redemption, bond accounting.  |
| DikeAMM               | `amm`                | Pool state, quotes, buys, sells, liquidity add/remove, LP balances.         |
| FeeManager            | `fee_manager`        | Fee and bond config display for market creation/admin surfaces.             |
| CODOracle             | `cod_oracle`         | Resolution request, proposal, dispute, escalation, finalization.            |
| CouncilOfDike         | `council_of_dike`    | Council case reads, commit/reveal voting, final report, reward claim.       |
| DikeGovernance        | `dike_governance`    | Governance state and privileged apply action visibility.                    |
| DikeTimelock          | `dike_timelock`      | Queued, cancelled, executed admin action tracking.                          |
| MockUSDC              | `mock_usdc`          | Local/dev only. Do not default production or testnet UI to mock collateral. |

Use the exact contract status enums:

```text
MarketStatus: Created, Live, Paused, TradingClosed, ResolutionRequested, Proposed, Disputed, CouncilVoting, Resolved, Cancelled
Outcome: Yes, No, Invalid
OracleStatus: None, Requested, Proposed, Disputed, Escalated, Finalized
CouncilCaseStatus: Opened, CommitPhase, RevealPhase, ReadyToFinalize, Finalized, Cancelled
TimelockActionKind: FeeConfig, Treasury, SupportedCollateral, Creator, CouncilMember, ModuleAddress, Pause, Upgrade
```

## Recommended Project Structure

```text
dike-web/
  package.json
  next.config.ts
  tsconfig.json
  .env.example
  src/
    app/
      layout.tsx
      page.tsx
      markets/
        page.tsx
        [marketId]/page.tsx
      portfolio/page.tsx
      create/page.tsx
      resolve/page.tsx
      council/page.tsx
      admin/page.tsx
    components/
      app-shell/
      data-state/
      forms/
      market/
      trade/
      wallet/
    features/
      admin/
      council/
      liquidity/
      market/
      portfolio/
      resolution/
      trading/
    lib/
      api/
      contracts/
      stellar/
      types/
      utils/
```

## Environment Configuration

Create `.env.example` with:

```text
NEXT_PUBLIC_STELLAR_NETWORK=testnet
NEXT_PUBLIC_STELLAR_RPC_URL=https://soroban-testnet.stellar.org
NEXT_PUBLIC_STELLAR_HORIZON_URL=https://horizon-testnet.stellar.org
NEXT_PUBLIC_STELLAR_NETWORK_PASSPHRASE=Test SDF Network ; September 2015
NEXT_PUBLIC_DIKE_SERVICES_URL=http://localhost:4000
NEXT_PUBLIC_DIKE_MANIFEST_NETWORK=testnet
```

The app should load contract addresses from a checked-in copy or generated import of `dike-contracts/deployments/testnet.json`. Do not hardcode contract IDs directly into components.

## Stellar And Wallet Layer

Implement `src/lib/stellar/` with:

- `config.ts`: network, RPC URL, Horizon URL, passphrase, manifest network validation.
- `rpc.ts`: singleton Stellar RPC server.
- `wallet.ts`: Freighter/Stellar Wallets Kit adapter.
- `transaction.ts`: build, simulate, sign, submit, poll, decode result helpers.
- `scval.ts`: contract argument and result codecs for Dike structs/enums.

Transaction rules:

- Always verify wallet network/passphrase before building a transaction.
- Always run RPC simulation before requesting signature.
- Always show the user the action, market id, amount, collateral, estimated fee, and deadline before signing.
- Always include `deadline` and `min_out` for AMM trades.
- Never sign automatically and never store secret keys.
- Decode and display known `DikeError` values such as `SlippageExceeded`, `DeadlineExpired`, `InvalidStatus`, `UnsupportedCollateral`, and `EncumberedPosition`.

## Pages And Feature Behavior

### `/markets`

Market discovery page backed by `dike-services`.

Show:

- Market question, category, expiry, status, final outcome if resolved.
- Implied YES/NO prices from AMM reserves.
- Liquidity, volume, active resolution state, and whether trading is available.
- Filters for status, category, expiry, and resolution state.

Primary data:

- `GET /markets` from `dike-services`.
- Optional direct refresh using `market_registry.get_market` and `amm.pool` for recently changed markets.

### `/markets/[marketId]`

Primary market detail and trading page.

Show:

- Full market metadata: question, rules URI, hashes, creator, collateral, expiry, dispute window, bond amount.
- AMM reserves, implied probabilities, fee config, and quote preview.
- User YES/NO balances, LP shares, root stake, deposits, child credit availability, child debt, parent debt.
- Resolution status and final redemption panel when applicable.

Actions:

- `amm.quote_buy_yes(pool_id, amount_in)`
- `amm.quote_buy_no(pool_id, amount_in)`
- `amm.buy_yes(trader, pool_id, amount_in, min_out, deadline)`
- `amm.buy_no(trader, pool_id, amount_in, min_out, deadline)`
- `amm.sell_yes(trader, pool_id, amount_in, min_out, deadline)`
- `amm.sell_no(trader, pool_id, amount_in, min_out, deadline)`
- `amm.add_liquidity(lp, pool_id, amount)`
- `amm.remove_liquidity(lp, pool_id, shares)`
- `collateral_vault.redeem_resolved(token, user, market_id, redeemed_outcome, amount)`
- `collateral_vault.redeem_cancelled(token, user, market_id, redeemed_outcome, amount)`

Child market buying:

- Expose `buy_child_yes` and `buy_child_no` only after the UI can clearly show parent market, parent outcome, available credit, and encumbrance rules.
- Child credit must reflect the contract limit of 60% of the user's parent root stake and one-level chaining only.

### `/portfolio`

User account page.

Show:

- All positions grouped by market and outcome.
- YES/NO token balances from indexed token events plus direct `conditional_tokens.balance` refresh.
- LP shares from `amm.lp_balance`.
- Deposits, root stakes, child credit, child debt, parent debt, redeemed amounts from `collateral_vault`.
- Redeemable resolved/cancelled positions.
- Encumbered positions with explanation that transfers/sells can be blocked by child or parent debt.

Actions:

- Redeem resolved winners or invalid refunds.
- Redeem cancelled markets at invalid-refund style payout.
- Remove liquidity where the market allows LP recovery.

### `/create`

Curated market creation page.

Only show the form if `dike-services` or direct contract reads confirm the connected address is an approved creator.

Fields:

- Question
- Question hash
- Rules URI
- Rules hash
- Category
- Expiry timestamp
- Collateral address, defaulting to the manifest USDC SAC
- Bond amount
- Dispute window
- Initial liquidity
- Opening price, fixed at `5000` bps because the contract currently rejects any other opening price
- Fee config, defaulting to the protocol defaults unless admin-provided values are available

Action:

- `market_factory.create_market(config, initial_liquidity, opening_price_bps)`

Validation:

- Question and rules URI cannot be empty.
- Expiry must satisfy the factory minimum expiry duration.
- Initial liquidity must meet the factory minimum liquidity.
- Bond amount and dispute window must be positive.
- Collateral must be supported.
- Creator must be approved.

### `/resolve`

Resolution workbench for expired markets.

Show tabs for:

- Markets ready to request resolution.
- Requested markets awaiting proposals.
- Proposed markets inside dispute window.
- Proposed markets ready for undisputed finalization.
- Disputed markets ready for council escalation.

Actions:

- `cod_oracle.request_resolution(market_id, question_hash, rules_uri, expiry, bond_amount, dispute_window)`
- `cod_oracle.propose_outcome(proposer, request_id, outcome, evidence_uri)`
- `cod_oracle.dispute_outcome(disputer, request_id, counter_outcome, evidence_uri)`
- `cod_oracle.finalize_undisputed(request_id)`
- `cod_oracle.escalate_to_council(request_id)`

UX rules:

- Evidence URI is required for proposals and disputes.
- Display proposal bond amount before signing.
- Display dispute deadline computed from `proposed_at + dispute_window`.
- Do not show finalization before the dispute window closes.

### `/council`

Council member dashboard for disputed cases.

Show:

- Active cases, evidence URIs, proposer/disputer outcomes, commit end, reveal end.
- Commit phase, reveal phase, finalize-ready, finalized states.
- User commitment status and reveal status.

Actions:

- `council_of_dike.vote_commitment(case_id, voter, outcome, salt)` for local commitment calculation.
- `council_of_dike.commit_vote(voter, case_id, commitment)`
- `council_of_dike.reveal_vote(voter, case_id, outcome, salt)`
- `council_of_dike.finalize_and_report_case(case_id)`
- `council_of_dike.claim_reward(voter, case_id)`

Commit-reveal storage:

- Generate a random 32-byte salt in the browser.
- Store pending reveal data locally, keyed by wallet address, network, and case id.
- Warn users that losing the salt prevents reveal.
- Never send salts to `dike-services`.

### `/admin`

Governance and operations dashboard.

Show:

- Current treasury, supported collateral, fee config, module addresses, approved creators, council members.
- Timelock queued/cancelled/executed actions.
- Module paused states when indexed or directly readable.
- Protocol fee sweep opportunities where relevant.

Actions:

- Admin write actions should be shown only to authorized wallets.
- Queue/execute flows should clearly separate `dike_timelock.queue`, `dike_timelock.execute`, and governance `apply_*` calls.
- The first version may be read-only except for explicitly safe operator flows.

## API Usage

Use `dike-services` for list/history/query-heavy state:

```text
GET /markets
GET /markets/:id
GET /users/:address/portfolio
GET /markets/:id/resolution
GET /council/cases
GET /admin/governance
GET /admin/timelock
```

Use direct Stellar RPC reads for action-critical freshness:

- `market_registry.get_market`
- `market_registry.is_tradeable`
- `market_registry.get_final_outcome`
- `amm.pool`
- `amm.quote_buy_yes`
- `amm.quote_buy_no`
- `amm.lp_balance`
- `conditional_tokens.balance`
- `collateral_vault.accounting`
- `collateral_vault.user_deposit`
- `collateral_vault.root_stake`
- `collateral_vault.child_avail_for_outcome`
- `collateral_vault.child_debt`
- `collateral_vault.parent_debt`
- `cod_oracle.request`
- `council_of_dike.case`
- `dike_timelock.action`

## Data Model Types

Define TypeScript domain types matching the Rust structs:

- `FeeConfig`
- `MarketConfig`
- `MarketData`
- `VaultAccounting`
- `PoolData`
- `TradeQuote`
- `ResolutionRequest`
- `CouncilCase`
- `TimelockAction`

Amounts should be represented internally as bigint-compatible integer strings at API boundaries and converted to `bigint` in contract helpers. Avoid floating point math for USDC, shares, reserves, bonds, fees, and payouts.

## UX States

Every transaction component must handle:

- Wallet not installed.
- Wallet not connected.
- Wrong network.
- Insufficient wallet balance.
- Simulation failed.
- Signature rejected.
- Submission pending.
- Submission failed.
- Submission succeeded but indexer has not caught up.

Market components must handle:

- Empty market list.
- Stale indexed data.
- RPC temporarily unavailable.
- Cancelled markets.
- Resolved markets.
- Disputed/council voting markets.
- Encumbered positions.

## Testing Plan

Unit tests:

- Dike enum/string conversions.
- Amount parsing and formatting.
- Deadline and dispute-window calculations.
- Slippage/min-out calculations.
- API response mapping to UI domain types.

Integration tests:

- Wallet adapter mock: connect, wrong network, sign rejection.
- Market list/detail load from mocked `dike-services`.
- Trade form calls quote before building transaction.
- Redeem form blocks unresolved markets.
- Council commit stores salt locally and reveal requires the same salt.

End-to-end tests:

- Connect wallet and view markets.
- Buy YES on a live market.
- Add and remove liquidity.
- Request resolution after expiry.
- Propose outcome and finalize undisputed after dispute window.
- Dispute, commit, reveal, finalize council case.
- Redeem winning and invalid/cancelled positions.

## Rollout Order

1. Bootstrap Next.js TypeScript project, env config, layout, wallet adapter, and manifest loader.
2. Add generated or manually wrapped contract clients and shared Dike types.
3. Build market list/detail pages from `dike-services` with direct RPC refresh.
4. Add quote, buy, sell, add liquidity, remove liquidity flows.
5. Add portfolio and redemption flows.
6. Add resolution workbench.
7. Add council dashboard.
8. Add admin/governance dashboard.
9. Add full transaction state handling and E2E coverage.

## Assumptions

- `dike-contracts` remains the canonical source of deployed contract addresses and public interfaces.
- The first target network is Stellar testnet.
- The default collateral is the testnet USDC SAC in `deployments/testnet.json`.
- `dike-web` does not run an indexer; it consumes `dike-services` for indexed state.
- User signing stays entirely in the browser wallet.
