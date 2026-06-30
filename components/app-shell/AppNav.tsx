"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { useWallet } from "@/lib/contexts/wallet";
import { Button } from "@/components/ui/button";
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
  const { address, isConnected, isConnecting, connect, disconnect } = useWallet();

  return (
    <header className="sticky top-0 z-40 border-b border-white/10 bg-background/80 backdrop-blur-md">
      <div className="mx-auto flex h-14 max-w-7xl items-center justify-between px-4">
        <Link
          href="/"
          className="font-heading text-2xl font-normal tracking-wide text-foreground hover:opacity-80 transition-opacity"
        >
          DIKE
        </Link>

        <nav className="hidden md:flex items-center gap-6">
          {NAV_LINKS.map((link) => (
            <Link
              key={link.href}
              href={link.href}
              className={cn(
                "relative text-sm tracking-wide transition-colors duration-300 hover:text-foreground group",
                pathname.startsWith(link.href)
                  ? "text-foreground"
                  : "text-foreground/50"
              )}
            >
              {link.label}
              <span className="absolute -bottom-0.5 left-0 w-0 h-px bg-foreground transition-all duration-300 group-hover:w-full" />
            </Link>
          ))}
        </nav>

        <div className="flex items-center gap-2">
          {isConnected && address ? (
            <Button variant="outline" size="xs" onClick={disconnect} title={address}>
              {address.slice(0, 4)}…{address.slice(-4)}
            </Button>
          ) : (
            <Button size="xs" onClick={connect} disabled={isConnecting}>
              {isConnecting ? "Connecting…" : "Connect"}
            </Button>
          )}
        </div>
      </div>
    </header>
  );
}
