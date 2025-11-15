"use client";

import { useState, useRef, useEffect } from "react";
import { Star } from "lucide-react";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import confetti from "canvas-confetti";
import { useWatchlist } from "@/contexts/WatchlistContext";

interface ClickableStarProps {
  initialState?: boolean;
  size?: number;
  onChange?: (isStarred: boolean) => void;
  className?: string;
  symbol: string;
}

export default function ClickableStar({
  initialState = false,
  size = 24,
  onChange,
  className,
  symbol,
}: ClickableStarProps) {
  console.log("ClickableStar props:", {
    initialState,
    size,
    symbol,
    className,
  });

  // Early return if symbol is invalid
  if (!symbol || symbol.trim() === "") {
    console.error("ClickableStar: Invalid symbol provided:", symbol);
    return (
      <Button
        variant="ghost"
        size="icon"
        disabled
        className="p-0 h-auto w-auto opacity-50"
      >
        <Star size={size} className="text-muted-foreground" />
      </Button>
    );
  }

  const [isStarred, setIsStarred] = useState(initialState);
  const buttonRef = useRef<HTMLButtonElement>(null);

  // Always try to get context, handle symbol check in usage
  let watchlist = null;
  try {
    watchlist = useWatchlist();
    console.log(`Watchlist context available for ${symbol}:`, !!watchlist);
  } catch (error) {
    console.log(
      `Watchlist context not available for ${symbol}:`,
      error instanceof Error ? error.message : String(error),
    );
  }

  // Sync with watchlist context
  useEffect(() => {
    if (watchlist) {
      const isInWatchlist = watchlist.isInWatchlist(symbol);
      console.log(`Syncing star for ${symbol}: ${isInWatchlist}`);
      setIsStarred(isInWatchlist);
    }
  }, [symbol, watchlist, watchlist?.watchlist]);

  const triggerConfetti = () => {
    if (!buttonRef.current) return;

    // Get the position of the star
    const rect = buttonRef.current.getBoundingClientRect();
    const x = (rect.left + rect.width / 2) / window.innerWidth;
    const y = (rect.top + rect.height / 2) / window.innerHeight;

    // Create a more localized confetti effect
    confetti({
      particleCount: 30,
      spread: 40,
      origin: { x, y },
      colors: ["#FFD700", "#FFC107", "#FFEB3B", "#FFD600"],
      disableForReducedMotion: true,
      zIndex: 1000,
      startVelocity: 15,
      scalar: 0.7,
      gravity: 1.2,
      ticks: 50,
    });
  };

  const handleClick = () => {
    console.log(`Star clicked for ${symbol}, current state: ${isStarred}`);
    const newState = !isStarred;

    // Update watchlist context
    if (watchlist) {
      console.log(
        `Updating watchlist: ${newState ? "adding" : "removing"} ${symbol}`,
      );
      if (newState) {
        watchlist.addToWatchlist(symbol);
      } else {
        watchlist.removeFromWatchlist(symbol);
      }
    } else {
      // Fallback to local state if no context
      console.log("No watchlist context, using local state only");
      setIsStarred(newState);
    }

    onChange?.(newState);

    // Only trigger confetti when starring (not when unstarring)
    if (newState) {
      triggerConfetti();
    }
  };

  return (
    <Button
      ref={buttonRef}
      variant="ghost"
      size="icon"
      onClick={handleClick}
      className={cn("p-0 h-auto w-auto hover:bg-transparent", className)}
      aria-label={isStarred ? "Unstar" : "Star"}
    >
      <Star
        size={size}
        className={cn(
          "!w-auto !h-auto transition-colors duration-200",
          isStarred
            ? "fill-yellow-400 text-yellow-400"
            : "fill-none text-muted-foreground hover:text-foreground",
        )}
      />
    </Button>
  );
}
