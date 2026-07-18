import type { Metadata } from "next";
import { Activity, ArrowUpRight, Layers3, WalletCards } from "lucide-react";
import { Card, CardContent } from "@/components/ui/card";
import { EmptyState } from "@/components/data-state/EmptyState";
import {
  fetchProtocolStats,
  type ProtocolStats,
  type StatsTransaction,
} from "@/lib/api/stats";
import { networkConfig } from "@/lib/stellar/config";

export const metadata: Metadata = {
  title: "Protocol Stats — DIKE",
  robots: { index: false, follow: false },
};

function formatTopic(topic: string) {
  return topic.replaceAll("_", " ");
}

function formatTimestamp(timestamp: string) {
  return new Intl.DateTimeFormat("en-US", {
    dateStyle: "medium",
    timeStyle: "short",
    timeZone: "UTC",
  }).format(new Date(timestamp));
}

function truncateHash(hash: string) {
  if (hash.length <= 18) return hash;
  return `${hash.slice(0, 10)}…${hash.slice(-8)}`;
}

function TransactionRow({ transaction }: { transaction: StatsTransaction }) {
  const explorerUrl = `https://stellar.expert/explorer/${networkConfig.network}/tx/${transaction.hash}`;

  return (
    <div className="grid gap-4 border-t border-white/10 px-5 py-5 transition-colors hover:bg-white/[0.025] md:grid-cols-[minmax(0,1.35fr)_minmax(0,1fr)_0.55fr_0.85fr_auto] md:items-center md:px-6">
      <div className="min-w-0">
        <p className="mb-1 text-[0.625rem] font-semibold uppercase tracking-[0.18em] text-muted-foreground md:hidden">
          Transaction
        </p>
        <p className="truncate font-mono text-xs text-foreground" title={transaction.hash}>
          {truncateHash(transaction.hash)}
        </p>
      </div>

      <div className="min-w-0">
        <p className="mb-1 text-[0.625rem] font-semibold uppercase tracking-[0.18em] text-muted-foreground md:hidden">
          Activity
        </p>
        <div className="flex flex-wrap gap-x-2 gap-y-1">
          {transaction.topics.map((topic) => (
            <span
              key={topic}
              className="text-[0.625rem] font-semibold uppercase tracking-[0.14em] text-amber-300/80"
            >
              {formatTopic(topic)}
            </span>
          ))}
        </div>
      </div>

      <div>
        <p className="mb-1 text-[0.625rem] font-semibold uppercase tracking-[0.18em] text-muted-foreground md:hidden">
          Ledger
        </p>
        <p className="font-mono text-xs text-foreground/80">#{transaction.ledger}</p>
      </div>

      <div>
        <p className="mb-1 text-[0.625rem] font-semibold uppercase tracking-[0.18em] text-muted-foreground md:hidden">
          Indexed
        </p>
        <p className="text-xs text-muted-foreground">{formatTimestamp(transaction.createdAt)} UTC</p>
        <p className="mt-1 text-[0.625rem] uppercase tracking-[0.14em] text-muted-foreground/60">
          {transaction.eventCount} {transaction.eventCount === 1 ? "event" : "events"}
        </p>
      </div>

      <a
        href={explorerUrl}
        target="_blank"
        rel="noreferrer"
        className="inline-flex w-fit items-center gap-1.5 text-[0.625rem] font-semibold uppercase tracking-[0.16em] text-foreground/70 transition-colors hover:text-amber-300"
        aria-label={`View transaction ${transaction.hash} on Stellar Expert`}
      >
        Explorer
        <ArrowUpRight className="size-3.5" aria-hidden="true" />
      </a>
    </div>
  );
}

export default async function StatsPage() {
  let stats: ProtocolStats;

  try {
    stats = await fetchProtocolStats();
  } catch {
    return (
      <div className="space-y-6">
        <div>
          <p className="mb-2 text-[0.625rem] font-semibold uppercase tracking-[0.22em] text-amber-300/70">
            Internal analytics
          </p>
          <h1 className="font-heading text-4xl font-normal tracking-tight">Protocol Stats</h1>
        </div>
        <Card className="border border-white/10 bg-card/60">
          <EmptyState
            title="Stats unavailable"
            description="The private analytics service could not be reached or authorized."
          />
        </Card>
      </div>
    );
  }

  return (
    <div className="space-y-8">
      <div className="flex flex-col gap-3 border-b border-white/10 pb-7 sm:flex-row sm:items-end sm:justify-between">
        <div>
          <p className="mb-2 text-[0.625rem] font-semibold uppercase tracking-[0.22em] text-amber-300/70">
            Internal analytics
          </p>
          <h1 className="font-heading text-4xl font-normal tracking-tight sm:text-5xl">Protocol Stats</h1>
          <p className="mt-2 max-w-xl text-sm text-muted-foreground">
            Indexed participation and on-chain activity across the Dike protocol.
          </p>
        </div>
        <div className="flex items-center gap-2 text-[0.625rem] font-semibold uppercase tracking-[0.18em] text-muted-foreground">
          <span className="size-1.5 rounded-full bg-emerald-400 shadow-[0_0_12px_rgba(52,211,153,0.8)]" />
          {networkConfig.network} index
        </div>
      </div>

      <div className="grid gap-4 md:grid-cols-2">
        <Card className="relative border border-white/10 bg-card/70 shadow-[0_24px_80px_rgba(0,0,0,0.22)]">
          <CardContent className="flex items-end justify-between gap-6">
            <div>
              <p className="text-[0.625rem] font-semibold uppercase tracking-[0.2em] text-muted-foreground">
                Connected wallets
              </p>
              <p className="mt-5 font-heading text-6xl font-normal tracking-tight tabular-nums">
                {stats.connectedWallets.toLocaleString()}
              </p>
              <p className="mt-2 text-xs text-muted-foreground">Unique protocol participants</p>
            </div>
            <div className="flex size-14 items-center justify-center border border-amber-300/20 bg-amber-300/[0.06] text-amber-300">
              <WalletCards className="size-6" aria-hidden="true" />
            </div>
          </CardContent>
        </Card>

        <Card className="relative border border-white/10 bg-card/70 shadow-[0_24px_80px_rgba(0,0,0,0.22)]">
          <CardContent className="flex items-end justify-between gap-6">
            <div>
              <p className="text-[0.625rem] font-semibold uppercase tracking-[0.2em] text-muted-foreground">
                Transactions
              </p>
              <p className="mt-5 font-heading text-6xl font-normal tracking-tight tabular-nums">
                {stats.transactionCount.toLocaleString()}
              </p>
              <p className="mt-2 text-xs text-muted-foreground">Distinct indexed transactions</p>
            </div>
            <div className="flex size-14 items-center justify-center border border-red-400/20 bg-red-400/[0.06] text-red-300">
              <Activity className="size-6" aria-hidden="true" />
            </div>
          </CardContent>
        </Card>
      </div>

      <Card className="gap-0 border border-white/10 bg-card/70 py-0 shadow-[0_24px_80px_rgba(0,0,0,0.18)]">
        <div className="flex items-start gap-3 px-5 py-6 md:px-6">
          <div className="mt-0.5 flex size-8 shrink-0 items-center justify-center border border-white/10 bg-white/[0.03] text-foreground/70">
            <Layers3 className="size-4" aria-hidden="true" />
          </div>
          <div>
            <h2 className="font-heading text-2xl font-normal tracking-tight">Transaction index</h2>
            <p className="mt-1 text-xs text-muted-foreground">
              Every distinct transaction observed by the Dike contract indexer.
            </p>
          </div>
        </div>

        {stats.transactions.length === 0 ? (
          <div className="border-t border-white/10">
            <EmptyState title="No transactions" description="No contract activity has been indexed yet." />
          </div>
        ) : (
          <div>
            <div className="hidden grid-cols-[minmax(0,1.35fr)_minmax(0,1fr)_0.55fr_0.85fr_auto] gap-4 border-t border-white/10 bg-white/[0.02] px-6 py-3 md:grid">
              {["Transaction", "Activity", "Ledger", "Indexed", ""].map((label, index) => (
                <p
                  key={`${label}-${index}`}
                  className="text-[0.625rem] font-semibold uppercase tracking-[0.18em] text-muted-foreground"
                >
                  {label}
                </p>
              ))}
            </div>
            {stats.transactions.map((transaction) => (
              <TransactionRow key={transaction.hash} transaction={transaction} />
            ))}
          </div>
        )}
      </Card>
    </div>
  );
}
