import type { Metadata } from 'next'
import { Geist, Geist_Mono, Montserrat } from 'next/font/google'
import { ClerkProvider } from '@clerk/nextjs'

import '@/app/globals.css'
import { ThemeProvider } from '@/components/theme-provider'

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
            {children}
          </ThemeProvider>
        </ClerkProvider>
      </body>
    </html>
  )
}