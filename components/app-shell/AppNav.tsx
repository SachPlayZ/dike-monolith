"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { useState, useEffect, useRef } from "react";
import { Menu, X } from "lucide-react";
import { useWallet } from "@/lib/contexts/wallet";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

const NAV_LINKS = [
  { href: "/predictions", label: "Markets" },
  { href: "/dashboard", label: "Portfolio" },
  { href: "/create-predic", label: "Create" },
  { href: "/resolve", label: "Resolve", permission: "canResolve" as const },
  { href: "/council", label: "Council", permission: "canCouncil" as const },
  { href: "/admin", label: "Admin", permission: "canAdmin" as const },
];

export function AppNav() {
  const pathname = usePathname();
  const { address, isConnected, isConnecting, connect, disconnect, permissions } = useWallet();
  const visibleLinks = NAV_LINKS.filter(
    (link) => !link.permission || Boolean(permissions?.[link.permission])
  );
  const [isMobileMenuOpen, setIsMobileMenuOpen] = useState(false);
  const mobileMenuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    function handleClickOutside(event: MouseEvent) {
      if (
        mobileMenuRef.current &&
        !mobileMenuRef.current.contains(event.target as Node)
      ) {
        setIsMobileMenuOpen(false);
      }
    }
    if (isMobileMenuOpen)
      document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, [isMobileMenuOpen]);

  useEffect(() => {
    setIsMobileMenuOpen(false);
  }, [pathname]);

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
          {visibleLinks.map((link) => (
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
          <button
            onClick={() => setIsMobileMenuOpen((open) => !open)}
            className="md:hidden flex items-center justify-center w-9 h-9 text-foreground border border-white/10 rounded-sm hover:bg-white/5 transition-colors"
            aria-label="Toggle menu"
          >
            {isMobileMenuOpen ? <X className="w-5 h-5" /> : <Menu className="w-5 h-5" />}
          </button>
        </div>
      </div>

      {isMobileMenuOpen && (
        <div
          ref={mobileMenuRef}
          className="md:hidden flex flex-col border-t border-white/10 bg-background/95 backdrop-blur-md"
        >
          {visibleLinks.map((link) => (
            <Link
              key={link.href}
              href={link.href}
              className={cn(
                "px-4 py-3 text-sm tracking-wide text-center transition-colors duration-300 hover:bg-white/5",
                pathname.startsWith(link.href)
                  ? "text-foreground"
                  : "text-foreground/50"
              )}
            >
              {link.label}
            </Link>
          ))}
        </div>
      )}
    </header>
  );
}
