import type { Metadata } from 'next'
import { Geist, Geist_Mono, Montserrat } from 'next/font/google'
import { ClerkProvider } from '@clerk/nextjs'

import '@/app/globals.css'
import { ThemeProvider } from '@/components/theme-provider'
import { SidebarInset, SidebarProvider } from '@/components/ui/sidebar'
import { AppSidebar } from '@/components/app-sidebar'
import { InvestmentDisclaimer } from '@/components/investment-disclaimer'
import { SearchBar } from '@/components/search-bar'
import { WatchlistProvider } from '@/contexts/WatchlistContext'

const geistSans = Geist({
  variable: '--font-geist-sans',
  subsets: ['latin'],
})

const geistMono = Geist_Mono({
  variable: '--font-geist-mono',
  subsets: ['latin'],
})

const montserrat = Montserrat({
  variable: '--font-montserrat',
  subsets: []
})

export const metadata: Metadata = {
  title: 'Crypto Exchange',
}

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode
}>) {
  return (
    <ClerkProvider afterSignOutUrl="/">
      <ThemeProvider
        attribute="class"
        defaultTheme="dark"
        enableSystem
        disableTransitionOnChange
      >
        <WatchlistProvider>
          <div className="m-auto h-full w-full max-h-screen bg-background">
            <SidebarProvider className="flex flex-col">
              <header className="flex flex-col w-full bg-background">
                <InvestmentDisclaimer />
                <SearchBar />
              </header>
              <div className="flex flex-1">
                <AppSidebar className={"sticky top-0 z-40 pl-4 pt-6 px-1 border-none shadow-none bg-background !h-[calc(100vh-120px)]"}></AppSidebar>
                <SidebarInset className="pt-2 pl-0 pr-6 overflow-hidden">
                  {children}
                </SidebarInset >
              </div >
            </SidebarProvider >
          </div >
        </WatchlistProvider>
      </ThemeProvider>
    </ClerkProvider>
  )
}
