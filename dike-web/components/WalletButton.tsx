"use client";

import { useWallet } from "@/lib/contexts/wallet";
import { cn } from "@/lib/utils";

interface WalletButtonProps {
  className?: string;
}

export function WalletButton({ className }: WalletButtonProps) {
  const { address, isConnected, isConnecting, connect, disconnect } =
    useWallet();

  if (isConnected && address) {
    return (
      <button
        onClick={disconnect}
        className={cn(className)}
        title={address}
        aria-label={`Disconnect wallet ${address}`}
      >
        {address.slice(0, 4)}…{address.slice(-4)}
      </button>
    );
  }

  return (
    <button
      onClick={connect}
      disabled={isConnecting}
      className={cn(className, isConnecting && "opacity-60 cursor-wait")}
      aria-label={isConnecting ? "Connecting wallet" : "Connect wallet"}
    >
      {isConnecting ? "connecting…" : "connect wallet"}
    </button>
  );
}
