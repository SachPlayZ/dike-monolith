import type { Metadata } from "next";
import { Instrument_Serif, Instrument_Sans, Geist_Mono } from "next/font/google";
import "./globals.css";
import { cn } from "@/lib/utils";
import { WalletProvider } from "@/lib/contexts/wallet";

const instrumentSerif = Instrument_Serif({
  variable: "--font-heading",
  weight: "400",
  subsets: ["latin"],
  display: "swap",
});

const instrumentSans = Instrument_Sans({
  variable: "--font-sans",
  weight: ["400", "600"],
  subsets: ["latin"],
  display: "swap",
});

const geistMono = Geist_Mono({
  variable: "--font-geist-mono",
  subsets: ["latin"],
});

export const metadata: Metadata = {
  title: "DIKE — Prediction Markets",
  description: "Decentralized prediction markets on Stellar",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html
      lang="en"
      className={cn("h-full antialiased", instrumentSerif.variable, instrumentSans.variable, geistMono.variable, "font-sans")}
    >
      <body className="min-h-full flex flex-col">
        <WalletProvider>{children}</WalletProvider>
      </body>
    </html>
  );
}
