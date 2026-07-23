import Link from "next/link";
import { ExternalLink } from "lucide-react";
import { networkConfig } from "@/lib/stellar/config";
import { CONTRACT_IDS } from "@/lib/contracts/manifest";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";

export const metadata = { title: "Reviewer Guide — DIKE" };

const REVIEW_STEPS = [
  ["Browse", "Open Markets. Live markets accept trades; expired markets are clearly marked and cannot be traded."],
  ["Verify", "Open the market rules, expiry, collateral, reserves, and resolution state before connecting a wallet."],
  ["Trade", "Connect a wallet on the displayed Stellar network, choose YES or NO, and inspect output, slippage, price impact, and fee before signing."],
  ["Resolve", "After expiry, follow request, proposal, dispute, council, and finalization state from the market page."],
  ["Redeem", "Open Portfolio after resolution. DIKE shows YES and NO balances and debt separately before redemption."],
] as const;

export default function ReviewPage() {
  const explorerBase = `https://stellar.expert/explorer/${networkConfig.explorerNetwork}/contract`;
  const contracts = [
    ["Market factory", CONTRACT_IDS.marketFactory()],
    ["Market registry", CONTRACT_IDS.marketRegistry()],
    ["AMM", CONTRACT_IDS.amm()],
    ["Collateral vault", CONTRACT_IDS.collateralVault()],
    ["Resolution oracle", CONTRACT_IDS.codOracle()],
  ] as const;

  return (
    <div className="space-y-6">
      <div>
        <p className="text-xs font-semibold uppercase tracking-widest text-primary">SCF reviewer path</p>
        <h1 className="mt-2 font-heading text-4xl tracking-tight">Review DIKE end to end</h1>
        <p className="mt-2 max-w-2xl text-sm text-muted-foreground">
          The interface is configured for {networkConfig.label}. All writes are wallet-signed Soroban transactions; indexed reads come from Dike services.
        </p>
      </div>

      <Alert variant={networkConfig.network === "mainnet" ? "warning" : "default"}>
        <AlertDescription>
          {networkConfig.network === "mainnet"
            ? "Mainnet uses real USDC and XLM. Review public state first; only sign a transaction you intend to execute."
            : "Testnet assets have no monetary value. Use a funded Stellar Testnet wallet for the interactive flow."}
        </AlertDescription>
      </Alert>

      <div className="grid gap-3 md:grid-cols-2">
        {REVIEW_STEPS.map(([title, description], index) => (
          <Card key={title} size="sm">
            <CardHeader>
              <CardTitle className="normal-case tracking-normal">{index + 1}. {title}</CardTitle>
            </CardHeader>
            <CardContent>
              <p className="text-sm text-muted-foreground">{description}</p>
            </CardContent>
          </Card>
        ))}
      </div>

      <div className="flex flex-wrap gap-3">
        <Button asChild><Link href="/predictions">Open Markets</Link></Button>
        <Button asChild variant="outline"><Link href="/dashboard">Open Portfolio</Link></Button>
        <Button asChild variant="outline">
          <a href="https://github.com/Dike-Protocol" target="_blank" rel="noopener noreferrer">
            Source <ExternalLink aria-hidden="true" />
          </a>
        </Button>
      </div>

      <Card>
        <CardHeader>
          <CardTitle className="normal-case tracking-normal">Deployed contracts</CardTitle>
        </CardHeader>
        <CardContent className="divide-y divide-border">
          {contracts.map(([label, contractId]) => (
            <a
              key={label}
              href={`${explorerBase}/${contractId}`}
              target="_blank"
              rel="noopener noreferrer"
              className="flex flex-col gap-1 py-3 text-sm hover:text-primary sm:flex-row sm:items-center sm:justify-between"
            >
              <span>{label}</span>
              <span className="font-mono text-xs text-muted-foreground">{contractId}</span>
            </a>
          ))}
        </CardContent>
      </Card>
    </div>
  );
}
