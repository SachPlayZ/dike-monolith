"use client";

import Link from "next/link";
import { Instrument_Sans, Instrument_Serif } from "next/font/google";
import { ChevronDown, Menu, X } from "lucide-react";
import { useState, useEffect, useRef } from "react";
import { WalletButton } from "@/components/WalletButton";

const instrumentSerif = Instrument_Serif({
  subsets: ["latin"],
  weight: ["400"],
  display: "swap",
});

const instrumentSans = Instrument_Sans({
  subsets: ["latin"],
  weight: ["400"],
  display: "swap",
});

export default function LandingNavbar() {
  const [isDropdownOpen, setIsDropdownOpen] = useState(false);
  const [isMobileMenuOpen, setIsMobileMenuOpen] = useState(false);
  const dropdownRef = useRef<HTMLDivElement>(null);
  const mobileMenuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    function handleClickOutside(event: MouseEvent) {
      if (
        dropdownRef.current &&
        !dropdownRef.current.contains(event.target as Node)
      ) {
        setIsDropdownOpen(false);
      }
      if (
        mobileMenuRef.current &&
        !mobileMenuRef.current.contains(event.target as Node)
      ) {
        setIsMobileMenuOpen(false);
      }
    }
    if (isDropdownOpen || isMobileMenuOpen)
      document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, [isDropdownOpen, isMobileMenuOpen]);

  return (
    <nav
      className={`${instrumentSans.className} absolute top-0 left-0 right-0 z-50 flex items-center justify-between py-4 px-6 md:px-8`}
    >
      <Link
        href="/"
        className={`${instrumentSerif.className} md:hidden relative text-3xl font-normal tracking-wide text-white transition-colors duration-300 hover:text-white/80`}
      >
        DIKE
      </Link>

      <div className="hidden md:flex absolute left-1/2 -translate-x-1/2 items-center gap-12 text-white">
        <Link
          href="/predictions"
          className="relative text-base font-normal tracking-wide transition-colors duration-300 hover:text-white/80 group"
        >
          predictions
          <span className="absolute bottom-0 left-0 w-0 h-px bg-white transition-all duration-300 group-hover:w-full"></span>
        </Link>

        <div className="relative" ref={dropdownRef}>
          <button
            onClick={() => setIsDropdownOpen(!isDropdownOpen)}
            aria-expanded={isDropdownOpen}
            aria-controls="trading-menu"
            className="relative flex items-center gap-1 text-base font-normal tracking-wide transition-colors duration-300 hover:text-white/80 group"
          >
            trading
            <ChevronDown
              className={`w-4 h-4 transition-transform duration-300 ${isDropdownOpen ? "rotate-180" : ""}`}
            />
            <span className="absolute bottom-0 left-0 w-0 h-px bg-white transition-all duration-300 group-hover:w-full"></span>
          </button>

          {isDropdownOpen && (
            <div id="trading-menu" className="absolute top-full left-1/2 -translate-x-1/2 mt-4 py-3 min-w-40 backdrop-blur-md bg-linear-to-b from-white/10 to-white/5 border border-white/30 rounded-md shadow-xl shadow-black/50">
              <Link
                href="/dashboard"
                onClick={() => setIsDropdownOpen(false)}
                className="block px-5 py-2.5 text-base font-normal tracking-wide transition-all duration-300 hover:text-white hover:bg-white/20 hover:pl-6"
              >
                dashboard
              </Link>
            </div>
          )}
        </div>

        <Link
          href="/"
          className={`${instrumentSerif.className} relative text-4xl font-normal tracking-wide transition-colors duration-300 hover:text-white/80 group`}
        >
          DIKE
          <span className="absolute bottom-0 left-0 w-0 h-px bg-white transition-all duration-300 group-hover:w-full"></span>
        </Link>

        <Link
          href="/create-predic"
          className="relative text-base font-normal tracking-wide transition-colors duration-300 hover:text-white/80 group"
        >
          create
          <span className="absolute bottom-0 left-0 w-0 h-px bg-white transition-all duration-300 group-hover:w-full"></span>
        </Link>

        <Link
          href="/profile"
          className="relative text-base font-normal tracking-wide transition-colors duration-300 hover:text-white/80 group"
        >
          profile
          <span className="absolute bottom-0 left-0 w-0 h-px bg-white transition-all duration-300 group-hover:w-full"></span>
        </Link>
      </div>

      <div className="flex items-center gap-3">
        <WalletButton className="relative flex items-center gap-2 px-3 py-2 md:px-4 text-sm md:text-base font-normal tracking-wide text-white transition-all duration-300 hover:text-white/80 border border-white/20 hover:border-white/40 rounded-sm backdrop-blur-sm bg-white/5 hover:bg-white/10" />
        <button
          onClick={() => setIsMobileMenuOpen(!isMobileMenuOpen)}
          className="md:hidden flex items-center justify-center w-10 h-10 text-white border border-white/20 rounded-sm bg-white/5 hover:bg-white/10 transition-colors duration-300"
          aria-label="Toggle menu"
          aria-expanded={isMobileMenuOpen}
          aria-controls="landing-mobile-menu"
        >
          {isMobileMenuOpen ? <X className="w-5 h-5" /> : <Menu className="w-5 h-5" />}
        </button>
      </div>

      {isMobileMenuOpen && (
        <div
          id="landing-mobile-menu"
          ref={mobileMenuRef}
          className="md:hidden absolute top-full left-0 right-0 flex flex-col items-center gap-1 py-4 bg-black/90 backdrop-blur-md border-t border-white/10"
        >
          <Link
            href="/predictions"
            onClick={() => setIsMobileMenuOpen(false)}
            className="w-full text-center py-3 text-base text-white tracking-wide hover:bg-white/10 transition-colors duration-300"
          >
            predictions
          </Link>
          <Link
            href="/dashboard"
            onClick={() => setIsMobileMenuOpen(false)}
            className="w-full text-center py-3 text-base text-white tracking-wide hover:bg-white/10 transition-colors duration-300"
          >
            dashboard
          </Link>
          <Link
            href="/create-predic"
            onClick={() => setIsMobileMenuOpen(false)}
            className="w-full text-center py-3 text-base text-white tracking-wide hover:bg-white/10 transition-colors duration-300"
          >
            create
          </Link>
          <Link
            href="/profile"
            onClick={() => setIsMobileMenuOpen(false)}
            className="w-full text-center py-3 text-base text-white tracking-wide hover:bg-white/10 transition-colors duration-300"
          >
            profile
          </Link>
        </div>
      )}
    </nav>
  );
}
