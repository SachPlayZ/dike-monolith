import type { ReactNode } from "react";
import { AppNav } from "@/components/app-shell/AppNav";
import { Toaster } from "sonner";

export default function AppLayout({ children }: { children: ReactNode }) {
  return (
    <div className="dark min-h-screen bg-background text-foreground">
      {/* Ambient glow orbs — brand warmth */}
      <div className="fixed inset-0 -z-10 overflow-hidden pointer-events-none" aria-hidden="true">
        <div className="absolute -top-72 -right-72 w-[500px] h-[500px] sm:w-[1000px] sm:h-[1000px] rounded-full bg-orange-500/[0.07] blur-[180px]" />
        <div className="absolute top-1/2 -left-72 w-[400px] h-[400px] sm:w-[800px] sm:h-[800px] rounded-full bg-amber-600/[0.04] blur-[150px]" />
        <div className="absolute -bottom-60 right-1/4 w-[300px] h-[300px] sm:w-[600px] sm:h-[600px] rounded-full bg-red-700/[0.03] blur-[120px]" />
      </div>
      <AppNav />
      <main className="mx-auto max-w-7xl px-4 py-8">{children}</main>
      <Toaster
        theme="dark"
        position="bottom-right"
        richColors
        toastOptions={{
          classNames: {
            toast: "!flex-wrap !items-start !rounded-xl !p-4 !gap-x-3",
            content: "!flex-1 !basis-full",
            icon: "!mt-0.5",
            actionButton:
              "!ml-0 !mt-3 !w-full !basis-full !h-9 !justify-center !rounded-lg !border !border-current/25 !bg-current/10 !text-current !font-medium !shadow-none hover:!bg-current/20",
            cancelButton:
              "!ml-0 !mt-3 !w-full !basis-full !h-9 !justify-center !rounded-lg",
          },
        }}
      />
    </div>
  );
}
