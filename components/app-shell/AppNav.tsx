"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { useWallet } from "@/lib/contexts/wallet";
import { cn } from "@/lib/utils";

const NAV_LINKS = [
  { href: "/predictions", label: "Markets" },
  { href: "/dashboard", label: "Portfolio" },
  { href: "/create-predic", label: "Create" },
  { href: "/resolve", label: "Resolve" },
  { href: "/council", label: "Council" },
  { href: "/admin", label: "Admin" },
];

export function AppNav() {
  const pathname = usePathname();
  const { address, isConnected, isConnecting, connect, disconnect } =
    useWallet();

  return (
    <header className="sticky top-0 z-40 border-b border-border bg-background/80 backdrop-blur-md">
      <div className="mx-auto flex h-14 max-w-7xl items-center justify-between px-4">
        <Link href="/" className="font-bold tracking-widest text-sm">
          DIKE
        </Link>

        <nav className="hidden md:flex items-center gap-6 text-sm">
          {NAV_LINKS.map((link) => (
            <Link
              key={link.href}
              href={link.href}
              className={cn(
                "transition-colors hover:text-foreground",
                pathname.startsWith(link.href)
                  ? "text-foreground font-medium"
                  : "text-muted-foreground"
              )}
            >
              {link.label}
            </Link>
          ))}
        </nav>

        <div className="flex items-center gap-2">
          {isConnected && address ? (
            <button
              onClick={disconnect}
              className="rounded-md border border-border px-3 py-1 text-xs font-mono hover:bg-muted transition-colors"
              title={address}
            >
              {address.slice(0, 4)}…{address.slice(-4)}
            </button>
          ) : (
            <button
              onClick={connect}
              disabled={isConnecting}
              className="rounded-md bg-primary px-3 py-1 text-xs text-primary-foreground hover:bg-primary/90 transition-colors disabled:opacity-60"
            >
              {isConnecting ? "Connecting…" : "Connect Wallet"}
            </button>
          )}
        </div>
      </div>
    </header>
  );
}
