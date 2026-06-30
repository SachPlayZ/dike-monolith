import type { ReactNode } from "react";
import { AppNav } from "@/components/app-shell/AppNav";
import { Toaster } from "sonner";

export default function AppLayout({ children }: { children: ReactNode }) {
  return (
    <div className="dark min-h-screen bg-background text-foreground">
      <AppNav />
      <main className="mx-auto max-w-7xl px-4 py-8">{children}</main>
      <Toaster theme="dark" position="bottom-right" richColors />
    </div>
  );
}
