# dike-web

The frontend for [Dike Protocol](https://github.com/Dike-Protocol) — a binary prediction-market platform built on Stellar/Soroban.

Next.js App Router app that reads market/portfolio/council/governance state from `dike-services`' REST API and submits transactions directly to the Soroban contracts in `dike-contracts` via a connected Stellar wallet (Freighter-style signing). There are no generated contract bindings here — Soroban XDR encoding/decoding is hand-written in `lib/stellar` and `lib/contracts`.

> [!IMPORTANT]
> This project's `next` dependency has non-standard changes vs upstream Next.js. Check `node_modules/next/dist/docs/` before relying on API behavior from training data or public Next.js docs.

## Architecture

```
Wallet (Freighter) ──sign──► lib/stellar/transaction.ts ──submit──► Soroban RPC
        ▲                            │
        │                    lib/contracts/clients.ts (hand-rolled XDR encode)
        │                            │
   lib/contexts/wallet.tsx    lib/contracts/manifest.ts ──► testnet.json / mainnet.json (contract IDs)
        │
   features/*  ◄──reads──  lib/api/client.ts ──► /api/proxy/* ──► dike-services REST API
```

**Read path:** pages call `lib/api` helpers, which hit `dike-services` through a same-origin proxy (`app/api/proxy/[...path]/route.ts` rewrites to `NEXT_PUBLIC_DIKE_SERVICES_URL`), and normalize responses via `lib/api/normalizers.ts`.

**Write path:** feature components build an unsigned transaction (`lib/contracts/clients.ts`), request a wallet signature (`lib/contexts/wallet.tsx`), then submit + poll via `lib/stellar/transaction.ts`. Contract errors are decoded through `DIKE_ERROR_MAP`, which must stay in sync with `dike_types::DikeError` in `dike-contracts`.

## Routes

| Path | Purpose |
| --- | --- |
| `/dashboard` | Portfolio overview: positions, LP shares, vault state, redeemables |
| `/predictions` | Market list with status/category filters |
| `/markets/[marketId]` | Market detail, trading, liquidity, and resolution |
| `/review` | SCF reviewer path and deployed-contract links |
| `/create-predic` | Approved-creator market creation flow |
| `/resolve` | Resolution lifecycle: propose, dispute, escalate, finalize |
| `/council` | Council of Dike commit-reveal voting + reward claims |
| `/admin` | Governance config, module addresses, timelock queue/execute |
| `/profile` | Connected wallet identity |

## Getting Started

```bash
pnpm install
cp .env.example .env
pnpm dev
```

Open [http://localhost:3000](http://localhost:3000).

Requires a running `dike-services` instance (defaults to `http://localhost:4000`) for all read paths, and a Freighter-compatible wallet extension for signing transactions.

## Configuration

| Variable | Description |
| --- | --- |
| `NEXT_PUBLIC_STELLAR_NETWORK` | `mainnet` or `testnet` |
| `NEXT_PUBLIC_STELLAR_RPC_URL` | Soroban RPC endpoint used for simulation/submission |
| `NEXT_PUBLIC_STELLAR_HORIZON_URL` | Horizon endpoint |
| `NEXT_PUBLIC_STELLAR_NETWORK_PASSPHRASE` | Must match the configured network |
| `NEXT_PUBLIC_DIKE_SERVICES_URL` | `dike-services` base URL, proxied at `/api/proxy/*` |
| `NEXT_PUBLIC_DIKE_MANIFEST_NETWORK` | `testnet` or `mainnet` — selects which manifest `lib/contracts/manifest.ts` reads contract IDs from |

Contract IDs come from `lib/contracts/testnet.json` and `lib/contracts/mainnet.json`, copies of `dike-contracts/deployments/<network>.json`. Keep them in sync manually after any contract redeploy — there is no build-time fetch or symlink.

The app validates the selected network, passphrase, manifest, and endpoint URL shapes at runtime. Configuration errors are shown in the UI and block wallet signing. Mainnet transactions use real assets; testnet assets have no monetary value.

## Reviewer flow

1. Open `/review` and confirm the displayed Stellar network.
2. Browse `/predictions`; expired markets are labeled and cannot be traded.
3. Open a live market and inspect its public HTTPS rules, reserves, and quote.
4. Connect a wallet on the displayed network. Review the declared maximum XLM fee before approving the wallet signature.
5. Follow resolution state on the market page and redeem finalized positions from `/dashboard`.

Market rules and resolution evidence must use public HTTPS URLs. Placeholder, local, credential-bearing, and non-HTTPS URLs are rejected.

## Project Structure

```
app/              # App Router routes ((app) group: dashboard, markets, predictions, etc.)
features/         # Feature components (trading, admin, council, portfolio, resolution)
lib/api/          # dike-services REST client + response normalizers
lib/contexts/     # Wallet connection context
lib/contracts/    # Manifest loader, hand-rolled contract call builders
lib/stellar/      # ScVal codecs, transaction build/sign/submit/poll, error decoding
lib/types/        # Shared frontend types
```

## Testing

```bash
pnpm test
pnpm lint
pnpm exec tsc --noEmit
pnpm build
```

Focused unit tests cover transaction encoding, ScVal conversions, portfolio normalization, and public-reference URL validation. CI runs tests, lint, typecheck, and a production build. Interactive wallet flows still require manual verification against `dike-services` and the selected Stellar network.

## Deployment

Deploys are driven by the hosting platform's git integration. Set the `NEXT_PUBLIC_*` variables above in the build environment, matching the selected manifest. Public variables are frozen into the browser bundle by `next build`.

## License

Apache-2.0. See [LICENSE](LICENSE).
