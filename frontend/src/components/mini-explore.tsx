'use client'

import { Button } from "@/components/ui/button";
import {
    Table,
    TableBody,
    TableCell,
    TableHead,
    TableHeader,
    TableRow,
} from "@/components/ui/table"
import { CircleStop, Flame, MoveUpRight, MoveDownRight, Percent, Sparkles, TrendingDown, TrendingUp } from "lucide-react";
import Image from 'next/image';
import ClickableStar from "@/components/clickable-star";
import { useRouter } from 'next/navigation';

const assets = [
    {
        title: "Bitcoin",
        short: "BTC",
        price: "£72,846.59",
        change: "0.54%",
        changeDirection: "up" as const,
        volume: "£28.4B",
        marketCap: "£1.44T",
        change1h: "0.12%",
        change1hDirection: "up" as const,
        change24h: "0.54%",
        change24hDirection: "up" as const,
        change1w: "2.18%",
        change1wDirection: "up" as const,
        image: {
            src: "/bitcoin-btc-logo.svg",
            alt: "Bitcoin logo"
        }
    },
    {
        title: "Ethereum",
        short: "ETH",
        price: "£1,381.07",
        change: "0.44%",
        changeDirection: "up" as const,
        volume: "£12.8B",
        marketCap: "£166B",
        change1h: "-0.08%",
        change1hDirection: "down" as const,
        change24h: "0.44%",
        change24hDirection: "up" as const,
        change1w: "1.92%",
        change1wDirection: "up" as const,
        image: {
            src: "/eth-glyph-colored.svg",
            alt: "Ether diamond colored logo"
        }
    },
    {
        title: "Tether USD",
        short: "USDT",
        price: "£0.7534",
        change: "0.26%",
        changeDirection: "up" as const,
        volume: "£42.1B",
        marketCap: "£95.2B",
        change1h: "0.01%",
        change1hDirection: "up" as const,
        change24h: "0.26%",
        change24hDirection: "up" as const,
        change1w: "-0.02%",
        change1wDirection: "down" as const,
        image: {
            src: "/tether-usdt-logo.svg",
            alt: "Tether USD"
        }
    },
    {
        title: "XRP",
        short: "XRP",
        price: "£1.67",
        change: "0.31%",
        changeDirection: "up" as const,
        volume: "£3.2B",
        marketCap: "£95.8B",
        change1h: "-0.15%",
        change1hDirection: "down" as const,
        change24h: "0.31%",
        change24hDirection: "up" as const,
        change1w: "-1.84%",
        change1wDirection: "down" as const,
        image: {
            src: "/xrp-xrp-logo.svg",
            alt: "XRP logo"
        }
    },
];

interface Asset {
    title: string
    short: string
    price: string
    change: string
    changeDirection: "up" | "down"
    volume: string
    marketCap: string
    change1h: string
    change1hDirection: "up" | "down"
    change24h: string
    change24hDirection: "up" | "down"
    change1w: string
    change1wDirection: "up" | "down"
    image: { src: string, alt: string }
}

function Row({ asset }: { asset: Asset }) {
    const router = useRouter();
    const PriceChangeIcon = asset.changeDirection === "up" ? MoveUpRight : MoveDownRight;
    const Change1hIcon = asset.change1hDirection === "up" ? MoveUpRight : MoveDownRight;
    const Change24hIcon = asset.change24hDirection === "up" ? MoveUpRight : MoveDownRight;
    const Change1wIcon = asset.change1wDirection === "up" ? MoveUpRight : MoveDownRight;

    const positiveColor = "#139E62";
    const negativeColor = "#dc2626";

    const handleRowClick = (e: React.MouseEvent) => {
        // Prevent navigation if clicking on the star
        if ((e.target as HTMLElement).closest('.clickable-star')) {
            return;
        }
        router.push(`/c/assets/${asset.short.toLowerCase()}`);
    };

    return (
        <TableRow className="cursor-pointer hover:bg-muted/50" onClick={handleRowClick}>
            <TableCell className="w-[180px]">
                <div className="flex flex-row gap-4">
                    <Image src={asset.image.src} alt={asset.image.alt} height={32} width={32} />
                    <div className="flex flex-col">
                        <span className="text-base text-foreground font-medium">
                            {asset.title}
                        </span>
                        <span className="text-muted-foreground">{asset.short}</span>
                    </div>
                </div>
            </TableCell>
            <TableCell className="w-[120px]">
                <div className="flex flex-col items-end">
                    <span className="text-base font-bold">{asset.price}</span>
                    <div className="flex items-center gap-0.5 text-xs">
                        <PriceChangeIcon
                            color={asset.changeDirection === "up" ? positiveColor : negativeColor}
                            strokeWidth={3}
                            size={12}
                        />
                        <span className={asset.changeDirection === "up" ? "text-[#139E62]" : "text-red-600"}>
                            {asset.change}
                        </span>
                    </div>
                </div>
            </TableCell>
            <TableCell className="w-[100px] text-right">
                <span className="text-sm font-medium">{asset.volume}</span>
            </TableCell>
            <TableCell className="w-[100px] text-right">
                <span className="text-sm font-medium">{asset.marketCap}</span>
            </TableCell>
            <TableCell className="w-[80px] text-right">
                <div className="flex items-center justify-end gap-0.5 text-xs">
                    <Change1hIcon
                        color={asset.change1hDirection === "up" ? positiveColor : negativeColor}
                        strokeWidth={3}
                        size={10}
                    />
                    <span className={asset.change1hDirection === "up" ? "text-[#139E62]" : "text-red-600"}>
                        {asset.change1h}
                    </span>
                </div>
            </TableCell>
            <TableCell className="w-[80px] text-right">
                <div className="flex items-center justify-end gap-0.5 text-xs">
                    <Change24hIcon
                        color={asset.change24hDirection === "up" ? positiveColor : negativeColor}
                        strokeWidth={3}
                        size={10}
                    />
                    <span className={asset.change24hDirection === "up" ? "text-[#139E62]" : "text-red-600"}>
                        {asset.change24h}
                    </span>
                </div>
            </TableCell>
            <TableCell className="w-[80px] text-right">
                <div className="flex items-center justify-end gap-0.5 text-xs">
                    <Change1wIcon
                        color={asset.change1wDirection === "up" ? positiveColor : negativeColor}
                        strokeWidth={3}
                        size={10}
                    />
                    <span className={asset.change1wDirection === "up" ? "text-[#139E62]" : "text-red-600"}>
                        {asset.change1w}
                    </span>
                </div>
            </TableCell>
            <TableCell className="w-[50px]">
                <ClickableStar size={20} className="cursor-pointer clickable-star" symbol={asset.short} />
            </TableCell>
        </TableRow>
    );
}

export function MiniExplore() {
    return (
        <section className="flex flex-col gap-[1rem]">
            <div className="text-xl font-bold">Explore the market</div>
            <div className="flex gap-2">
                <Button variant="outline" className="cursor-pointer">
                    <Flame /> Most Popular
                </Button>
                <Button variant="outline" className="cursor-pointer">
                    <Percent />
                    Earn
                </Button>
                <Button variant="outline" className="cursor-pointer">
                    <TrendingUp /> Gainers
                </Button>
                <Button variant="outline" className="cursor-pointer">
                    <TrendingDown /> Losers
                </Button>
                <Button variant="outline" className="cursor-pointer">
                    <CircleStop />
                    Stabelcoins
                </Button>
                <Button variant="outline" className="cursor-pointer">
                    <Sparkles />
                    Newly Listed
                </Button>
            </div>
            <div className="bg-card rounded-[20px] p-6">
                <Table className="">
                    <TableHeader className="[&_tr]:border-none [&_tr]:hover:bg-background">
                        <TableRow>
                            <TableHead className="w-[180px] text-left text-xs uppercase text-muted-foreground">
                                Asset
                            </TableHead>
                            <TableHead className="w-[120px] text-right text-xs uppercase text-muted-foreground">
                                Price
                            </TableHead>
                            <TableHead className="w-[100px] text-right text-xs uppercase text-muted-foreground">
                                Volume
                            </TableHead>
                            <TableHead className="w-[100px] text-right text-xs uppercase text-muted-foreground">
                                Market Cap
                            </TableHead>
                            <TableHead className="w-[80px] text-right text-xs uppercase text-muted-foreground">
                                1h
                            </TableHead>
                            <TableHead className="w-[80px] text-right text-xs uppercase text-muted-foreground">
                                24h
                            </TableHead>
                            <TableHead className="w-[80px] text-right text-xs uppercase text-muted-foreground">
                                1w
                            </TableHead>
                            <TableHead className="w-[50px]">

                            </TableHead>
                        </TableRow>
                    </TableHeader>
                    <TableBody>
                        {
                            assets.map((asset) => (
                                <Row key={asset.title} asset={asset} />
                            ))
                        }
                    </TableBody>
                </Table>
            </div>
        </section>
    );
}