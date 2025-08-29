'use client'

import React, { createContext, useContext, ReactNode } from 'react'
import { useLocalStorage } from '@/hooks/useLocalStorage'

interface WatchlistContextType {
  watchlist: string[]
  addToWatchlist: (symbol: string) => void
  removeFromWatchlist: (symbol: string) => void
  isInWatchlist: (symbol: string) => boolean
}

const WatchlistContext = createContext<WatchlistContextType | undefined>(undefined)

export function useWatchlist() {
  const context = useContext(WatchlistContext)
  if (context === undefined) {
    throw new Error('useWatchlist must be used within a WatchlistProvider')
  }
  return context
}

interface WatchlistProviderProps {
  children: ReactNode
}

export function WatchlistProvider({ children }: WatchlistProviderProps) {
  const [watchlist, setWatchlist] = useLocalStorage<string[]>('crypto-watchlist', [])

  const addToWatchlist = (symbol: string) => {
    console.log('Adding to watchlist:', symbol)
    setWatchlist(prev => {
      if (!prev.includes(symbol)) {
        const newWatchlist = [...prev, symbol]
        console.log('New watchlist:', newWatchlist)
        return newWatchlist
      }
      return prev
    })
  }

  const removeFromWatchlist = (symbol: string) => {
    console.log('Removing from watchlist:', symbol)
    setWatchlist(prev => {
      const newWatchlist = prev.filter(item => item !== symbol)
      console.log('New watchlist:', newWatchlist)
      return newWatchlist
    })
  }

  const isInWatchlist = (symbol: string) => {
    return watchlist.includes(symbol)
  }

  const value: WatchlistContextType = {
    watchlist,
    addToWatchlist,
    removeFromWatchlist,
    isInWatchlist,
  }

  return (
    <WatchlistContext.Provider value={value}>
      {children}
    </WatchlistContext.Provider>
  )
}