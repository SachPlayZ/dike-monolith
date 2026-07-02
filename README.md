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
   lib/contexts/wallet.tsx    lib/contracts/manifest.ts ──► testnet.json (contract IDs)
        │
   features/*  ◄──reads──  lib/api/client.ts ──► /api/proxy/* ──► dike-services REST API
```

**Read path:** pages call `lib/api` helpers, which hit `dike-services` through a same-origin proxy (`app/api/proxy/[...path]/route.ts` rewrites to `NEXT_PUBLIC_DIKE_SERVICES_URL`), and normalize responses via `lib/api/normalizers.ts`.

**Write path:** feature components build an unsigned transaction (`lib/contracts/clients.ts`), request a wallet signature (`lib/contexts/wallet.tsx`), then submit + poll via `lib/stellar/transaction.ts`. Contract errors are decoded through `DIKE_ERROR_MAP`, which must stay in sync with `dike_types::DikeError` in `dike-contracts`.

## Routes

| Path | Purpose |
| --- | --- |
| `/dashboard` | Portfolio overview: positions, LP shares, vault state, redeemables |
| `/markets` | Market list with status/tradeability filters |
| `/predictions` | Market detail, trading, liquidity |
| `/create-predic` | Approved-creator market creation flow |
| `/resolve` | Resolution lifecycle: propose, dispute, escalate, finalize |
| `/council` | Council of Dike commit-reveal voting + reward claims |
| `/admin` | Governance config, module addresses, timelock queue/execute |
| `/profile` | Connected wallet identity |

## Getting Started

```bash
npm install
cp .env.example .env
npm run dev
```

Open [http://localhost:3000](http://localhost:3000).

Requires a running `dike-services` instance (defaults to `http://localhost:4000`) for all read paths, and a Freighter-compatible wallet extension for signing transactions.

## Configuration

| Variable | Description |
| --- | --- |
| `NEXT_PUBLIC_STELLAR_NETWORK` | `mainnet`, `testnet`, or `local` |
| `NEXT_PUBLIC_STELLAR_RPC_URL` | Soroban RPC endpoint used for simulation/submission |
| `NEXT_PUBLIC_STELLAR_HORIZON_URL` | Horizon endpoint |
| `NEXT_PUBLIC_STELLAR_NETWORK_PASSPHRASE` | Must match the configured network |
| `NEXT_PUBLIC_DIKE_SERVICES_URL` | `dike-services` base URL, proxied at `/api/proxy/*` |
| `NEXT_PUBLIC_DIKE_MANIFEST_NETWORK` | Which key of `lib/contracts/testnet.json` to read contract IDs from |

Contract IDs come from `lib/contracts/testnet.json`, a copy of `dike-contracts/deployments/<network>.json`. Keep it in sync manually after any contract redeploy — there is no build-time fetch or symlink.

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
npm run lint       # eslint
npx tsc --noEmit   # typecheck
```

No automated UI test suite — verify trading/liquidity/resolution/council/governance flows manually against a running `dike-services` + testnet contracts before shipping changes to `lib/contracts` or `lib/stellar`.

## Deployment

No CI workflow in this repo; deploys are driven by the hosting platform's git integration on push. Set the `NEXT_PUBLIC_*` variables above in the hosting environment, matching whichever network's manifest `lib/contracts/testnet.json` reflects.
