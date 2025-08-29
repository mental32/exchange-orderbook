'use client'

import * as React from "react";
import { CartesianGrid, Line, LineChart, XAxis } from "recharts";
import {
    Card,
    CardContent,
    CardHeader,
    CardTitle,
} from "@/components/ui/card";
import {
    HoverCard,
    HoverCardContent,
    HoverCardTrigger,
} from "@/components/ui/hover-card";
import {
    ChartContainer,
    ChartTooltip,
    ChartTooltipContent,
} from "@/components/ui/chart";
import {
    Table,
    TableBody,
    TableCell,
    TableHead,
    TableHeader,
    TableRow,
} from "@/components/ui/table"
import { Button } from "@/components/ui/button";
import { MoveUpRight, ArrowDownToLine, ArrowUpFromLine } from "lucide-react";
import Image from 'next/image';
import { useRouter } from 'next/navigation';

// Asset data from mini-explore.tsx
const cryptoAssets = [
    {
        title: "Bitcoin",
        short: "BTC",
        price: "£72,846.59",
        holdings: "0.045",
        totalValue: "£3,278.10",
        image: {
            src: "/bitcoin-btc-logo.svg",
            alt: "Bitcoin logo"
        }
    },
    {
        title: "Ethereum",
        short: "ETH",
        price: "£1,381.07",
        holdings: "0.234",
        totalValue: "£323.17",
        image: {
            src: "/eth-glyph-colored.svg",
            alt: "Ether diamond colored logo"
        }
    },
    {
        title: "XRP",
        short: "XRP",
        price: "£1.67",
        holdings: "142.7",
        totalValue: "£238.31",
        image: {
            src: "/xrp-xrp-logo.svg",
            alt: "XRP logo"
        }
    },
];

const cashAssets = [
    {
        title: "Pound Sterling",
        short: "GBP",
        balance: "£1,245.89",
        available: "£1,245.89"
    },
    {
        title: "Tether USD",
        short: "USDT",
        balance: "£485.62",
        available: "£485.62",
        image: {
            src: "/tether-usdt-logo.svg",
            alt: "Tether USD"
        }
    }
];

// Portfolio data
const chartData = [
    { date: "2024-05-01", portfolioValue: 4890.23 },
    { date: "2024-05-15", portfolioValue: 5120.45 },
    { date: "2024-06-01", portfolioValue: 4995.67 },
    { date: "2024-06-15", portfolioValue: 5245.12 },
    { date: "2024-07-01", portfolioValue: 5359.74 },
    { date: "2024-07-15", portfolioValue: 5180.33 },
    { date: "2024-07-30", portfolioValue: 5485.47 }
];

function PortfolioValueSection() {
    return (
        <div className="flex flex-col gap-4">
            {/* Portfolio Value */}
            <div className="flex flex-col">
                <HoverCard>
                    <HoverCardTrigger asChild>
                        <Button
                            variant="link"
                            className="p-0 text-muted-foreground underline decoration-wavy focus-visible:ring-0 focus-visible:outline-none cursor-pointer w-fit"
                        >
                            Portfolio Value
                        </Button>
                    </HoverCardTrigger>
                    <HoverCardContent>
                        <span className="text-foreground">Portfolio Value</span>
                    </HoverCardContent>
                </HoverCard>
                <div className="flex items-baseline text-5xl">
                    <span className="text-muted-foreground">£</span>
                    <span style={{ color: 'oklch(0.25 0.133 284)' }}>5,485.47</span>
                </div>
            </div>
            
            {/* Balance Change */}
            <div className="flex items-center gap-0.5 text-sm">
                <MoveUpRight color="#35df8d" strokeWidth={3} size={16} />
                <span className="text-[#139E62]">2.34% (£125.73)</span>
                <span className="text-muted-foreground">last month</span>
            </div>
            
            {/* Unrealised Return */}
            <div className="flex flex-col">
                <span className="text-muted-foreground text-sm">Unrealised Return</span>
                <div className="flex items-center gap-0.5">
                    <MoveUpRight color="#35df8d" strokeWidth={3} size={14} />
                    <span className="text-[#139E62] font-semibold">£342.18 (6.67%)</span>
                </div>
            </div>
            
            {/* Action Buttons */}
            <div className="flex gap-3 mt-4">
                <Button className="flex items-center gap-2" size="sm">
                    <ArrowDownToLine size={16} />
                    Deposit
                </Button>
                <Button variant="outline" className="flex items-center gap-2" size="sm">
                    <ArrowUpFromLine size={16} />
                    Withdraw
                </Button>
            </div>
        </div>
    );
}

function PortfolioChart() {
    return (
        <div className="flex-1 h-[400px]">
            <ChartContainer config={{}} className="aspect-auto h-full w-full">
                <LineChart
                    accessibilityLayer
                    data={chartData}
                    margin={{
                        left: 12,
                        right: 12,
                        top: 12,
                        bottom: 12,
                    }}
                >
                    <CartesianGrid vertical={false} />
                    <XAxis
                        dataKey="date"
                        tickLine={false}
                        axisLine={false}
                        tickMargin={8}
                        minTickGap={32}
                        tickFormatter={(value) => {
                            const date = new Date(value);
                            return date.toLocaleDateString("en-US", {
                                month: "short",
                                day: "numeric",
                            });
                        }}
                    />
                    <ChartTooltip
                        content={
                            <ChartTooltipContent
                                className="w-[150px]"
                                nameKey="portfolioValue"
                                labelFormatter={(value) => {
                                    return new Date(value).toLocaleDateString("en-US", {
                                        month: "short",
                                        day: "numeric",
                                        year: "numeric",
                                    });
                                }}
                                formatter={(value) => [`£${value}`, "Portfolio Value"]}
                            />
                        }
                    />
                    <Line
                        dataKey="portfolioValue"
                        type="monotone"
                        stroke="#8b5cf6"
                        strokeWidth={3}
                        dot={false}
                    />
                </LineChart>
            </ChartContainer>
        </div>
    );
}

export default function Page() {
    const router = useRouter();
    
    return (
        <div className="flex flex-col gap-8 p-6">
            {/* Portfolio Overview - Complex Flexbox Layout */}
            <div className="flex gap-8 h-[450px]">
                {/* Top-left: Portfolio Value, Change, Return, and Actions */}
                <div className="w-80">
                    <PortfolioValueSection />
                </div>
                
                {/* Main Chart Area */}
                <PortfolioChart />
            </div>
            
            {/* Your Balances Section */}
            <div className="flex flex-col gap-6">
                <h2 className="text-2xl font-bold">Your Balances</h2>
                
                {/* Cryptocurrencies Section */}
                <div className="bg-card rounded-[20px] p-6">
                    <h3 className="text-xl font-semibold mb-4">Cryptocurrencies</h3>
                    <Table>
                        <TableHeader className="[&_tr]:border-none [&_tr]:hover:bg-background">
                            <TableRow>
                                <TableHead className="w-[200px] text-left text-xs uppercase text-muted-foreground">
                                    Asset
                                </TableHead>
                                <TableHead className="w-[120px] text-right text-xs uppercase text-muted-foreground">
                                    Price
                                </TableHead>
                                <TableHead className="w-[120px] text-right text-xs uppercase text-muted-foreground">
                                    Holdings
                                </TableHead>
                                <TableHead className="w-[120px] text-right text-xs uppercase text-muted-foreground">
                                    Total Value
                                </TableHead>
                            </TableRow>
                        </TableHeader>
                        <TableBody>
                            {cryptoAssets.map((asset) => (
                                <TableRow key={asset.short} className="cursor-pointer hover:bg-muted/50" onClick={() => router.push(`/c/assets/${asset.short.toLowerCase()}`)}>
                                    <TableCell className="w-[200px]">
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
                                    <TableCell className="w-[120px] text-right">
                                        <span className="text-base font-bold">{asset.price}</span>
                                    </TableCell>
                                    <TableCell className="w-[120px] text-right">
                                        <span className="text-base font-medium">{asset.holdings} {asset.short}</span>
                                    </TableCell>
                                    <TableCell className="w-[120px] text-right">
                                        <span className="text-base font-bold">{asset.totalValue}</span>
                                    </TableCell>
                                </TableRow>
                            ))}
                        </TableBody>
                    </Table>
                </div>
                
                {/* Cash & Stablecoins Section */}
                <div className="bg-card rounded-[20px] p-6">
                    <h3 className="text-xl font-semibold mb-4">Cash & Stablecoins</h3>
                    <Table>
                        <TableHeader className="[&_tr]:border-none [&_tr]:hover:bg-background">
                            <TableRow>
                                <TableHead className="w-[200px] text-left text-xs uppercase text-muted-foreground">
                                    Currency
                                </TableHead>
                                <TableHead className="w-[120px] text-right text-xs uppercase text-muted-foreground">
                                    Balance
                                </TableHead>
                                <TableHead className="w-[120px] text-right text-xs uppercase text-muted-foreground">
                                    Available
                                </TableHead>
                            </TableRow>
                        </TableHeader>
                        <TableBody>
                            {cashAssets.map((asset) => (
                                <TableRow key={asset.short}>
                                    <TableCell className="w-[200px]">
                                        <div className="flex flex-row gap-4">
                                            {asset.image ? (
                                                <Image src={asset.image.src} alt={asset.image.alt} height={32} width={32} />
                                            ) : (
                                                <div className="h-8 w-8 rounded-full bg-muted flex items-center justify-center">
                                                    <span className="text-sm font-bold">{asset.short}</span>
                                                </div>
                                            )}
                                            <div className="flex flex-col">
                                                <span className="text-base text-foreground font-medium">
                                                    {asset.title}
                                                </span>
                                                <span className="text-muted-foreground">{asset.short}</span>
                                            </div>
                                        </div>
                                    </TableCell>
                                    <TableCell className="w-[120px] text-right">
                                        <span className="text-base font-bold">{asset.balance}</span>
                                    </TableCell>
                                    <TableCell className="w-[120px] text-right">
                                        <span className="text-base font-medium">{asset.available}</span>
                                    </TableCell>
                                </TableRow>
                            ))}
                        </TableBody>
                    </Table>
                </div>
            </div>
        </div>
    );
}