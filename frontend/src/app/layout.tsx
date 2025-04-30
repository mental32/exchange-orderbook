import type { Metadata } from 'next'
import { Geist, Geist_Mono, Montserrat } from 'next/font/google'
import { ClerkProvider } from '@clerk/nextjs'

import '@/app/globals.css'
import { ThemeProvider } from '@/components/theme-provider'
import { SidebarInset, SidebarProvider } from '@/components/ui/sidebar'
import { AppSidebar } from '@/components/app-sidebar'

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
    <html lang="en" suppressHydrationWarning>
      <body className={`${montserrat.variable} ${geistSans.variable} ${geistMono.variable} bg-background`}>
        <ClerkProvider afterSignOutUrl="/">
          <ThemeProvider
            attribute="class"
            defaultTheme="dark"
            enableSystem
            disableTransitionOnChange
          >
            <div className="m-auto h-full w-full max-h-screen bg-background">
              <SidebarProvider className="flex flex-col">
                <header className="flex sticky top-0 z-50 w-full items-center bg-background h-[6rem]"></header>
                <div className="flex flex-1 max-h-[calc(100vh - 6rem)]">
                  <AppSidebar className={"sticky z-40 pl-4 pt-6 px-1 border-none shadow-none bg-background top-[6rem] !h-[calc(100svh-6rem)]"}></AppSidebar>
                  <SidebarInset className="pt-10 pl-0 pr-6 overflow-hidden">
                    {children}
                  </SidebarInset >
                </div >
              </SidebarProvider >
            </div >
          </ThemeProvider>
        </ClerkProvider>
      </body>
    </html>
  )
}
