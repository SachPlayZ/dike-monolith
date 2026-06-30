import type { ReactNode } from "react";
import { AppNav } from "@/components/app-shell/AppNav";

export default function AppLayout({ children }: { children: ReactNode }) {
  return (
    <>
      <AppNav />
      <main className="mx-auto max-w-7xl px-4 py-8">{children}</main>
    </>
  );
}
