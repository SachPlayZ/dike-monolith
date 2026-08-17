# Todo

## Plan

### Scope and decisions

- [ ] Add fee sponsorship only in `dike-services` and `dike-web`; no Soroban contract or deployment changes.
- [ ] Keep the connected G-account as the inner transaction source. The wallet signs the assembled Soroban transaction; the service pays only the fee through a fee-bump envelope.
- [ ] Sponsor one `invokeHostFunction` operation targeting an allowlisted Dike contract/method from the active deployment manifest.
- [ ] Use a server-only sponsor seed behind a signer interface. Fail startup when sponsorship is enabled with invalid/missing key material. Never send or log the seed.
- [ ] Use an explicit rollout flag. When sponsorship is enabled, failures are shown to the user; never silently fall back to charging the wallet. When disabled, retain the current direct-submit path.
- [ ] Protect funds with XDR/body limits, source-signature and time-bound checks, contract/method allowlists, fee/resource caps, per-IP/per-source quotas, a conservative daily declared-fee budget, and replay protection in Redis.
- [ ] Return the outer fee-bump hash and base64 Soroban return-value XDR so existing UI behavior, including council reward decoding, remains intact.
- [ ] V1 limitation: standard G-account source signatures only. Multisig/custom signer policy support is a follow-up.

### Target flow

1. Web loads the source account, builds one Dike contract invocation with the network minimum inner inclusion fee, simulates it, and assembles Soroban resources.
2. Wallet verifies network and signs the inner transaction.
3. Web sends only the signed inner XDR to `POST /sponsorship/transactions` through the restricted Next.js proxy.
4. Service parses and verifies network/type/source/signature/time bounds, operation target/method, resource fee, total fee, quota, budget, and replay state.
5. Service builds `TransactionBuilder.buildFeeBumpTransaction(...)`, signs the outer envelope with the sponsor, submits through Stellar RPC, and polls the outer hash.
6. Web decodes the response, shows sponsored/pending/success states, and refreshes existing views.

### 25 commits

- [ ] **01 — `document fee sponsorship architecture`**
  - Add `docs/fee-sponsorship.md` with trust boundaries, sequence diagram, fee math, rollout behavior, threat model, v1 limitations, and rollback procedure.
  - Record that the outer total is `sponsor inclusion bid × (inner operations + 1) + Soroban resource fee`; do not reuse the current inner `0.2 XLM` inclusion ceiling as the sponsored inner bid.
  - Verify: architecture review against current `dike-web/lib/stellar/transaction.ts` flow and official Stellar fee-bump rules.

- [ ] **02 — `configure fee sponsorship`**
  - Extend `dike-services/src/config/env.ts`, `.env.example`, and env tests with enable flag, sponsor seed, outer inclusion bid, max total/resource fee, per-minute quotas, daily budget, replay TTL, and confirmation timeout.
  - Require safe integer-string stroop values; reject enabled mode with absent/invalid limits.
  - Verify: `npm test -- src/config/env.test.ts`; `npm run lint` in `dike-services`.

- [ ] **03 — `add sponsor signer`**
  - Add `dike-services/src/sponsorship/signer.ts` with a narrow signer interface and an Ed25519 seed-backed implementation.
  - Derive/expose only the public address; ensure thrown errors and logs cannot contain seed text. Leave the interface replaceable by KMS/HSM signing.
  - Verify: signer unit tests for valid signing, invalid seed, public address, and secret non-disclosure.

- [ ] **04 — `define sponsorship errors`**
  - Add stable domain/result types and public error codes: disabled, malformed XDR, unsupported envelope/source/operation, bad signature, expired, disallowed contract/method, fee limit, quota, budget, replay, RPC rejected, failed, timeout.
  - Add one safe error-to-HTTP mapper; keep raw RPC/XDR diagnostics server-side.
  - Verify: table-driven mapping and serialization tests.

- [ ] **05 — `validate sponsorship payloads`**
  - Add the request schema/parser for `{ signedTransactionXdr }`, strict unknown-field handling, base64 validation, and bounded encoded/decoded size.
  - Add Fastify request validation tests without constructing a sponsor or hitting RPC.
  - Verify: targeted request-validation tests and services typecheck.

- [ ] **06 — `parse sponsored inner transactions`**
  - Add `dike-services/src/sponsorship/validator.ts` to parse XDR with the configured passphrase and require a v1 inner `Transaction`, one operation, a G-account source, Soroban data, non-empty signatures, and finite time bounds.
  - Reject nested fee bumps, classic operations, muxed/contract sources, malformed extensions, and expired/overlong validity windows.
  - Verify: fixture-based valid/invalid envelope unit tests.

- [ ] **07 — `verify inner source signatures`**
  - Verify a decorated signature over the inner transaction hash against the source G-account and configured network passphrase; reject absent, mismatched, duplicate-only, or wrong-network signatures.
  - State the single-source-signature v1 constraint in public errors/docs.
  - Verify: deterministic keypair tests including tampered payload and wrong passphrase.

- [ ] **08 — `allowlist sponsored contract calls`**
  - Decode `invokeContract` host functions and require the contract ID to match the active manifest plus the method to match a checked-in app-method allowlist.
  - Cover AMM, vault, registry, oracle, council, timelock, and market-factory write methods currently built by `dike-web/lib/contracts/clients.ts`; exclude read simulations, arbitrary contracts, and unknown methods.
  - Verify: every current app write is accepted; representative unknown contract/method and non-invoke host functions are rejected.

- [ ] **09 — `enforce sponsorship fee limits`**
  - Extract Soroban resource fee and inner inclusion fee safely with bigint math; require minimum-valid inner fee, configured resource ceiling, configured outer declared-fee ceiling, one operation, and no integer overflow.
  - Calculate the exact proposed outer maximum before reserving budget.
  - Verify: boundary tests for minimum, exact maximum, over-cap, negative/overflow-like XDR values, and high resource fees.

- [ ] **10 — `limit sponsorship spend`**
  - Add Redis-backed atomic per-IP and per-source rolling quotas plus a UTC daily conservative budget in declared stroops.
  - Reserve budget before signing; release reservation only on pre-submission failure. Expire all quota/budget keys.
  - Verify: concurrency tests prove caps cannot be bypassed and keys expire/reset.

- [ ] **11 — `prevent sponsored transaction replay`**
  - Key idempotency by inner transaction hash. Use a short Redis lock while processing and cache terminal success/failure for the configured TTL.
  - Return cached success for identical retries; reject an in-flight duplicate; never create multiple outer envelopes for one inner hash.
  - Verify: concurrent duplicate, retry-after-success, retry-after-safe-pre-submit-failure, and TTL tests.

- [ ] **12 — `build fee bump envelopes`**
  - Add a pure fee-bump builder using `TransactionBuilder.buildFeeBumpTransaction`, configured sponsor address/base fee, original signed inner envelope, and configured passphrase.
  - Sign only the outer envelope; preserve the inner XDR/signatures byte-for-byte; assert computed outer fee/source before submission.
  - Verify: decode round-trip tests for outer fee source, fee math, inner signatures, resource fee, and both testnet/mainnet passphrases.

- [ ] **13 — `submit sponsored transactions`**
  - Add an injectable Stellar RPC submit/poll adapter accepting `FeeBumpTransaction`.
  - Handle `ERROR`, `PENDING`, duplicate/pending statuses, success, inner failure, and bounded timeout. Return outer hash and optional base64 `returnValueXdr`.
  - Verify: mocked RPC status-sequence tests; confirm failure XDR is sanitized for clients.

- [ ] **14 — `orchestrate fee sponsorship`**
  - Add `FeeSponsorshipService`: validate → quota/budget → replay lock → build/sign → submit/poll → cache result.
  - Make dependencies injectable and make cleanup paths explicit so locks and budget reservations cannot leak on pre-submit exceptions.
  - Verify: service-level happy path plus validation, quota, signing, RPC rejection, failure, timeout, and duplicate tests.

- [ ] **15 — `expose sponsored transaction api`**
  - Register `POST /sponsorship/transactions` in `dike-services/src/api/server.ts` via a focused route module.
  - Apply a stricter route rate limit/body limit, derive client IP server-side, ignore spoofable address claims, and return stable JSON/status codes.
  - Verify: Fastify inject tests for success and all public rejection classes; existing protected-route behavior unchanged.

- [ ] **16 — `instrument fee sponsorship`**
  - Extend `MetricsStore` with requested/accepted/rejected/confirmed/failed/timeout counts, rejection reasons, declared stroops reserved, and latency.
  - Add structured logs containing inner hash, outer hash, source, method, and outcome—never full XDR, auth data, or sponsor seed.
  - Verify: metrics/log redaction tests and existing metrics snapshot tests.

- [ ] **17 — `report sponsorship availability`**
  - Add `GET /sponsorship/status` returning enabled/available, network, sponsor public address, policy limits, and a generic unavailable reason; never return seed or exact balance.
  - Include sponsor account/RPC/Redis readiness in `/health` without taking unrelated read APIs down when sponsorship is disabled/degraded.
  - Verify: enabled, disabled, unfunded/missing account, Redis failure, and degraded RPC health tests.

- [ ] **18 — `proxy sponsorship requests`**
  - Extend `dike-web/app/api/proxy/[...path]/route.ts` with POST forwarding only for the exact sponsorship transaction path.
  - Preserve content type/status, disable caching, cap body size, add upstream timeout, and keep `/admin` and `/metrics` blocked.
  - Verify: proxy route tests for allowed POST, blocked paths, oversized body, timeout, and upstream error.

- [ ] **19 — `add sponsorship web client`**
  - Add `dike-web/lib/api/sponsorship.ts` for status and submit calls with typed results/errors and `returnValueXdr` decoding.
  - Distinguish sponsor unavailable, policy rejection, transaction failure, timeout, and transport failure for `parseDikeError`/UI use.
  - Verify: mocked fetch tests for success, encoded return value, stable errors, and malformed responses.

- [ ] **20 — `prepare inner transactions for sponsorship`**
  - Split direct and sponsored inclusion-fee policies in `dike-web/lib/stellar/transaction.ts`; sponsored inners use the network minimum inclusion bid before simulation while retaining assembled resource fees.
  - Make fee helpers understand normal and fee-bump envelopes and report `wallet fee = 0` separately from sponsor maximum.
  - Verify: transaction tests for assembled resource fee preservation, direct fee behavior, sponsored inner fee, and fee-bump decoding.

- [ ] **21 — `show gasless signing state`**
  - Add `sponsoring` to `TxStatus`/`TxStateDisplay` and update wallet confirmation copy to say the app pays the network fee when sponsorship is active.
  - Show network, contract method, and zero wallet network fee; do not claim asset transfers/bonds/liquidity are free.
  - Verify: component/context tests for sponsored, direct, wrong-network, user-cancelled, and unavailable states.

- [ ] **22 — `centralize transaction execution`**
  - Add a shared web executor/hook for build → sign → sponsor/direct submit → decode → state transitions, preserving explicit direct mode when sponsorship is disabled.
  - Prevent double submission and never silently direct-submit after a sponsorship failure.
  - Verify: unit tests for both modes and every transition/error path.

- [ ] **23 — `sponsor trading transactions`**
  - Migrate `TradeForm.tsx`, `ChildTradeForm.tsx`, and `LiquidityForm.tsx` to the shared executor.
  - Preserve slippage/deadline behavior, balance refresh, explorer links, and existing button locking.
  - Verify: focused UI tests plus web test/lint/typecheck.

- [ ] **24 — `sponsor remaining app transactions`**
  - Migrate redeem, close trading, sweep fees, resolution, council vote/reward, market creation, and timelock execution flows.
  - Preserve council reward return-value decoding, local reveal salt handling, role gates, confirmations, toasts, and refresh behavior.
  - Verify: focused tests for each flow; `pnpm test`, `pnpm lint`, `pnpm exec tsc --noEmit`, and `pnpm build`.

- [ ] **25 — `verify fee sponsorship end to end`**
  - Add a local/testnet smoke script and runbook that funds only the sponsor for fees, executes a wallet-signed Dike call, confirms the outer fee source/hash and inner source/result, checks replay/quota behavior, and records rollback steps.
  - Update root/services/web READMEs and deployment instructions with sponsor funding, secret rotation, monitoring, alert thresholds, daily-budget exhaustion, staged testnet→mainnet rollout, and the Stellar wallet sponsorship calculator.
  - Verify full services suite (`npm test`, lint, build), full web suite (test, lint, typecheck, build), smoke test on local Quickstart or testnet, git diff, and no contract/deployment manifest changes.

## Verification

- [ ] Confirm exactly 25 focused commits; every commit builds and includes its relevant tests.
- [ ] Services: `npm test`, `npm run lint`, `npm run build`, `npm run audit`.
- [ ] Web: `pnpm test`, `pnpm lint`, `pnpm exec tsc --noEmit`, `pnpm build`, `pnpm audit --prod --audit-level high`.
- [ ] Security: tampered/wrong-network/expired/disallowed/oversized/high-fee/replayed requests never reach sponsor signing.
- [ ] Funds: quota and daily budget remain atomic under concurrency; sponsor exhaustion degrades only sponsorship.
- [ ] Network: testnet/local proof shows inner user signature retained, outer sponsor signature/source correct, wallet XLM fee unchanged, and sponsor charged.
- [ ] UX: all write paths show sponsoring/pending/success/failure accurately and never silently charge the wallet after sponsor failure.
- [ ] Review final diff for secret exposure, full-XDR logging, unrelated refactors, contract changes, and deployment-manifest changes.

## Files likely touched

- `docs/fee-sponsorship.md`
- `dike-services/src/config/env.ts`, `.env.example`, `src/api/server.ts`
- `dike-services/src/sponsorship/*`, `src/api/sponsorship-routes.ts`
- `dike-services/src/observability/metrics.ts`, `src/observability/health.ts`
- `dike-services/scripts/*`, `dike-services/README.md`, `dike-services/package.json`
- `dike-web/app/api/proxy/[...path]/route.ts`
- `dike-web/lib/api/sponsorship.ts`, `lib/stellar/transaction.ts`, `lib/stellar/execute.ts`
- `dike-web/lib/contexts/wallet.tsx`, `lib/types/index.ts`, `components/data-state/TxState.tsx`
- Existing transaction-producing forms under `dike-web/features/**` and `dike-web/app/(app)/**`
- Root `README.md` and corresponding unit/integration tests

## Unresolved questions

- [ ] Production signer backend: encrypted seed injected at runtime for v1, or KMS/HSM before mainnet?
- [ ] Final per-source/per-IP quotas, daily XLM budget, max resource fee, and outer inclusion bid.
- [ ] Sponsor all allowlisted admin/timelock writes, or user-facing writes only?

## Review

### Changed

- Added this repo-specific, 25-commit implementation plan.

### Verified

- Mapped all current web write flows and service boundaries.
- Checked fee-bump/Soroban fee math against current Stellar SDK/docs.
- Confirmed no contract change is required and exactly 25 commits are defined.

### Risks

- Sponsor-key custody and spend policy must be decided before mainnet enablement.
- V1 deliberately excludes multisig/custom source-signature policies.

### Follow-ups

- Resolve the three product/operations questions before commits 02, 03, 08, and 10 are finalized.
