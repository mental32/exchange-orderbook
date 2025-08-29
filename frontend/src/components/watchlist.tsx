'use client'

import { useWatchlist } from "@/contexts/WatchlistContext";
import { WatchlistCard } from "./WatchlistCard";


// Import crypto assets data from explore page
const cryptoAssets = [
    {
        title: "Bitcoin",
        short: "BTC",
        price: "£72,846.59",
        change24h: "2.54%",
        change24hDirection: "up" as const,
        image: {
            src: "/bitcoin-btc-logo.svg",
            alt: "Bitcoin logo"
        }
    },
    {
        title: "Ethereum",
        short: "ETH",
        price: "£1,381.07",
        change24h: "1.44%",
        change24hDirection: "up" as const,
        image: {
            src: "/eth-glyph-colored.svg",
            alt: "Ether diamond colored logo"
        }
    },
    {
        title: "Solana",
        short: "SOL",
        price: "£194.73",
        change24h: "3.21%",
        change24hDirection: "up" as const,
        image: {
            src: "/solana-sol-logo.svg",
            alt: "Solana logo"
        }
    },
    {
        title: "XRP",
        short: "XRP",
        price: "£1.67",
        change24h: "-0.31%",
        change24hDirection: "down" as const,
        image: {
            src: "/xrp-xrp-logo.svg",
            alt: "XRP logo"
        }
    },
    {
        title: "Cardano",
        short: "ADA",
        price: "£0.89",
        change24h: "4.12%",
        change24hDirection: "up" as const,
        image: {
            src: "/cardano-ada-logo.svg",
            alt: "Cardano logo"
        }
    },
    {
        title: "Tether USD",
        short: "USDT",
        price: "£0.7534",
        change24h: "0.02%",
        change24hDirection: "up" as const,
        image: {
            src: "/tether-usdt-logo.svg",
            alt: "Tether USD"
        }
    }
];

export function Watchlist() {
    const { watchlist } = useWatchlist();
    console.log('Watchlist component render, current watchlist:', watchlist);

    // Get full asset data for watchlist items
    const watchlistAssets = watchlist.map(symbol =>
        cryptoAssets.find(asset => asset.short === symbol)
    ).filter(Boolean);

    return (
        <section className="flex flex-col gap-[1rem]">
            <div className="text-xl font-bold">Watchlist</div>
            <div className="grid auto-cols-max grid-flow-col gap-[1rem] overflow-hidden">
                {watchlistAssets.length === 0 ? (
                    <div className="text-muted-foreground text-sm py-8">
                        No assets in your watchlist. Star assets in the Explore page to add them here.
                    </div>
                ) : (
                    watchlistAssets.map((asset) => (
                        <WatchlistCard
                            key={asset!.short}
                            title={asset!.title}
                            short={asset!.short}
                            price={asset!.price}
                            change24h={asset!.change24h}
                            change24hDirection={asset!.change24hDirection}
                            image={asset!.image}
                        />
                    ))
                )}
            </div>
        </section>
    );
}