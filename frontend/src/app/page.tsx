import SiteFooter from "@/components/site-footer";
import { Button, buttonVariants } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { SignedIn, SignedOut } from "@clerk/nextjs";
import { ChartCandlestickIcon } from "lucide-react";
import Link from "next/link";

import { HyperAccountInfo } from "@/components/hyperliquid/account-info";
import { HyperAnnouncement } from "@/components/hyperliquid/banner";
import { HyperBottomLedger } from "@/components/hyperliquid/bottom-ledger";
import { HyperFavoriteRibbon } from "@/components/hyperliquid/favorite-ribbon";
import { HyperMarketCanvas } from "@/components/hyperliquid/market-canvas";
import { HyperMarketSummary } from "@/components/hyperliquid/market-summary";
import { HyperOrderBook } from "@/components/hyperliquid/order-book";
import { HyperStatusBar } from "@/components/hyperliquid/status-bar";
import { HyperTopBar } from "@/components/hyperliquid/top-bar";
import { HyperTradePanel } from "@/components/hyperliquid/trade-panel";

export default function App() {
  return (
    <div className="flex h-screen flex-col overflow-hidden bg-[#1B2429]">
      <div className="sticky top-0 z-40 bg-[#020d14]">
        <HyperTopBar />
        <HyperAnnouncement />
      </div>
      <main className="flex-1 min-h-0 overflow-hidden overscroll-contain m-1">
        <div className="h-full w-full min-h-0 flex gap-1">
          <div className="flex flex-col flex-[4] gap-1">
            <div className="flex gap-1 overflow-hidden">
              <div className="flex-[3] flex flex-col gap-1">
                <HyperFavoriteRibbon className="bg-background " />
                <HyperMarketSummary className="bg-background " />
                <HyperMarketCanvas className="bg-background " />
              </div>
              <HyperOrderBook className="flex-1 bg-background border border-[#12303c]" />
            </div>
            <HyperBottomLedger className="gap-1 flex-1" />
          </div>
          <div className="flex flex-col flex-1 gap-2">
            <HyperTradePanel className="flex-[4] gap-2" />
            <HyperAccountInfo className="flex-[1] gap-2 rounded-lg border border-[#0f232f] bg-[#061922] p-4" />
          </div>
        </div>
      </main>
      <div className="sticky bottom-0 z-40 bg-[#020d14]">
        <HyperStatusBar />
      </div>
    </div>
  );
}

// Legacy landing kept for reference. This preserves the original marketing
// entry point without deleting the implementation. Update callers if we
// want to restore the previous experience.
export function LegacyLanding() {
  return (
    <div className={`antialiased flex flex-col min-h-screen`}>
      <header className="sticky top-0 z-40 w-full border-b bg-background">
        <div className="container mx-auto pl-6 flex h-16 items-center justify-between space-x-2 sm:space-x-0">
          <div className="flex gap-6 md:gap-10">
            <Link href="/" className="flex items-center flex-row space-x-2">
              <ChartCandlestickIcon />
              <span className="font-bold text-xl inline-block">
                {" "}
                Crypto Exchange
              </span>
            </Link>
          </div>
          <div className="flex items-center space-x-4">
            <nav className="flex items-center space-x-2">
              <SignedOut>
                <Link href="/sign-in">
                  <Button>Sign In</Button>
                </Link>
                <Link href="/sign-up">
                  <Button>Sign Up</Button>
                </Link>
              </SignedOut>
              <SignedIn>
                <Link
                  href="/c/"
                  className={cn(buttonVariants({ variant: "link" }))}
                >
                  My Account
                </Link>
              </SignedIn>
            </nav>
          </div>
        </div>
      </header>

      <main className="flex-1 container mx-auto ">
        <section className="w-full py-12 md:py-24 lg:py-32">
          <div className="px-4 md:px-6">
            <div className="grid gap-6 lg:grid-cols-2 lg:gap-12 items-center">
              <div className="space-y-4">
                <h1 className="text-3xl font-bold tracking-tighter sm:text-4xl md:text-5xl lg:text-6xl">
                  Invest in your future
                </h1>
                <p className="text-muted-foreground md:text-xl">
                  Grow your portfolio in a fair and open financial system.
                </p>
                <div className="flex flex-col sm:flex-row gap-3">
                  <Button
                    asChild
                    size="lg"
                    className="bg-blue-600 hover:bg-blue-700 text-white"
                  >
                    <Link href="/sign-up">Get Started</Link>
                  </Button>
                </div>
              </div>
              <div className="grid grid-cols-2 gap-4 lg:gap-8 p-4">
                {/* <CryptoIcon symbol="BTC" color="bg-orange-500" rotation={-5} />
                      <CryptoIcon symbol="ETH" color="bg-blue-600" rotation={3} />
                      <CryptoIcon symbol="USDC" color="bg-blue-800" rotation={-3} />
                      <CryptoIcon symbol="SOL" color="bg-teal-500" rotation={5} /> */}
              </div>
            </div>
          </div>
        </section>
      </main>

      <SiteFooter />
    </div>
  );
}
