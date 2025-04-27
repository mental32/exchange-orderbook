import SiteFooter from "@/components/site-footer";
import { ThemeProvider } from "@/components/theme-provider";
import { Button, buttonVariants } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { SignedIn, SignedOut, SignInButton, SignUpButton } from "@clerk/nextjs";
import { ChartCandlestickIcon } from "lucide-react";
import Link from "next/link";

export default function App() {
  return (
    <div className={`antialiased flex flex-col min-h-screen`}>
      <header className="sticky top-0 z-40 w-full border-b bg-background">
        <div className="container mx-auto pl-6 flex h-16 items-center justify-between space-x-2 sm:space-x-0">
          <div className="flex gap-6 md:gap-10">
            <Link href="/" className="flex items-center flex-row space-x-2">
              <ChartCandlestickIcon />
              <span className="font-bold text-xl inline-block"> Crypto Exchange</span>
            </Link>
          </div>
          <div className="flex items-center space-x-4">
            <nav className="flex items-center space-x-2">
              <SignedOut>
                <SignInButton>
                  <Button>Sign In</Button>
                </SignInButton>
                <SignUpButton>
                  <Button>Sign Up</Button>
                </SignUpButton>
              </SignedOut>
              <SignedIn>
                <Link href="/c/" className={cn(buttonVariants({ variant: 'link' }))}>
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
                  <Button asChild size="lg" className="bg-blue-600 hover:bg-blue-700 text-white">
                    <SignUpButton>
                      <Link href="/">Get Started</Link>
                    </SignUpButton>
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
