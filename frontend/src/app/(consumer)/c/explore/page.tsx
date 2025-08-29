'use client'

import * as React from "react";
import { CartesianGrid, Line, LineChart, XAxis, PieChart, Pie, Cell } from "recharts";
import {
    Card,
    CardContent,
    CardHeader,
    CardTitle,
} from "@/components/ui/card";
import {
    Table,
    TableBody,
    TableCell,
    TableHead,
    TableHeader,
    TableRow,
} from "@/components/ui/table"
import {
    ChartContainer,
    ChartTooltip,
    ChartTooltipContent,
} from "@/components/ui/chart";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
    Flame,
    Percent,
    TrendingUp,
    TrendingDown,
    CircleStop,
    Sparkles,
    MoveUpRight,
    MoveDownRight,
    Coins,
    Layers,
    Zap,
    Smile
} from "lucide-react";
import Image from 'next/image';
import SiteFooter from "@/components/site-footer";
import ClickableStar from "@/components/clickable-star";
import { useRouter } from 'next/navigation';

// Extended crypto assets data with additional market information
const cryptoAssets = [
    {
        title: "Bitcoin",
        short: "BTC",
        price: "£72,846.59",
        change24h: "2.54%",
        change24hDirection: "up" as const,
        change1h: "0.12%",
        change1hDirection: "up" as const,
        change1w: "8.18%",
        change1wDirection: "up" as const,
        volume: "£28.4B",
        marketCap: "£1.44T",
        volumeMarketCapRatio: "0.0197",
        circulatingSupply: "19.8M BTC",
        allTimeHigh: "£73,750.07",
        dominance: 54.2,
        image: {
            src: "/bitcoin-btc-logo.svg",
            alt: "Bitcoin logo"
        },
        sparklineData: [
            { price: 68500 }, { price: 69200 }, { price: 70100 }, { price: 71800 },
            { price: 70900 }, { price: 72200 }, { price: 72846 }
        ]
    },
    {
        title: "Ethereum",
        short: "ETH",
        price: "£1,381.07",
        change24h: "1.44%",
        change24hDirection: "up" as const,
        change1h: "-0.08%",
        change1hDirection: "down" as const,
        change1w: "5.92%",
        change1wDirection: "up" as const,
        volume: "£12.8B",
        marketCap: "£166B",
        volumeMarketCapRatio: "0.0771",
        circulatingSupply: "120.4M ETH",
        allTimeHigh: "£3,456.23",
        dominance: 18.7,
        image: {
            src: "/eth-glyph-colored.svg",
            alt: "Ether diamond colored logo"
        },
        sparklineData: [
            { price: 1320 }, { price: 1340 }, { price: 1365 }, { price: 1355 },
            { price: 1370 }, { price: 1378 }, { price: 1381 }
        ]
    },
    {
        title: "Solana",
        short: "SOL",
        price: "£194.73",
        change24h: "3.21%",
        change24hDirection: "up" as const,
        change1h: "0.45%",
        change1hDirection: "up" as const,
        change1w: "12.87%",
        change1wDirection: "up" as const,
        volume: "£2.1B",
        marketCap: "£92.4B",
        volumeMarketCapRatio: "0.0227",
        circulatingSupply: "474.8M SOL",
        allTimeHigh: "£210.18",
        dominance: 3.1,
        image: {
            src: "/solana-sol-logo.svg",
            alt: "Solana logo"
        },
        sparklineData: [
            { price: 185 }, { price: 188 }, { price: 192 }, { price: 189 },
            { price: 193 }, { price: 196 }, { price: 194 }
        ]
    },
    {
        title: "XRP",
        short: "XRP",
        price: "£1.67",
        change24h: "-0.31%",
        change24hDirection: "down" as const,
        change1h: "-0.15%",
        change1hDirection: "down" as const,
        change1w: "-2.84%",
        change1wDirection: "down" as const,
        volume: "£3.2B",
        marketCap: "£95.8B",
        volumeMarketCapRatio: "0.0334",
        circulatingSupply: "57.4B XRP",
        allTimeHigh: "£2.89",
        dominance: 3.2,
        image: {
            src: "/xrp-xrp-logo.svg",
            alt: "XRP logo"
        },
        sparklineData: [
            { price: 1.72 }, { price: 1.70 }, { price: 1.68 }, { price: 1.69 },
            { price: 1.66 }, { price: 1.67 }, { price: 1.67 }
        ]
    },
    {
        title: "Cardano",
        short: "ADA",
        price: "£0.89",
        change24h: "4.12%",
        change24hDirection: "up" as const,
        change1h: "0.23%",
        change1hDirection: "up" as const,
        change1w: "7.45%",
        change1wDirection: "up" as const,
        volume: "£1.8B",
        marketCap: "£31.2B",
        volumeMarketCapRatio: "0.0577",
        circulatingSupply: "35.1B ADA",
        allTimeHigh: "£2.31",
        dominance: 1.0,
        image: {
            src: "/cardano-ada-logo.svg",
            alt: "Cardano logo"
        },
        sparklineData: [
            { price: 0.84 }, { price: 0.86 }, { price: 0.87 }, { price: 0.88 },
            { price: 0.87 }, { price: 0.89 }, { price: 0.89 }
        ]
    },
    {
        title: "Tether USD",
        short: "USDT",
        price: "£0.7534",
        change24h: "0.02%",
        change24hDirection: "up" as const,
        change1h: "0.01%",
        change1hDirection: "up" as const,
        change1w: "-0.02%",
        change1wDirection: "down" as const,
        volume: "£42.1B",
        marketCap: "£95.2B",
        volumeMarketCapRatio: "0.442",
        circulatingSupply: "126.4B USDT",
        allTimeHigh: "£0.96",
        dominance: 3.2,
        image: {
            src: "/tether-usdt-logo.svg",
            alt: "Tether USD"
        },
        sparklineData: [
            { price: 0.753 }, { price: 0.754 }, { price: 0.753 }, { price: 0.754 },
            { price: 0.753 }, { price: 0.754 }, { price: 0.753 }
        ]
    }
];

// Market statistics data
const marketStats = [
    {
        title: "Total Market Cap",
        value: "£2.98T",
        change: "2.1%",
        changeDirection: "up" as const,
        description: "Global cryptocurrency market capitalization"
    },
    {
        title: "24h Trading Volume",
        value: "£89.4B",
        change: "12.7%",
        changeDirection: "up" as const,
        description: "Total trading volume across all exchanges"
    },
    {
        title: "Bitcoin Dominance",
        value: "54.2%",
        change: "-0.8%",
        changeDirection: "down" as const,
        description: "Bitcoin's share of total crypto market cap"
    },
    {
        title: "Active Cryptocurrencies",
        value: "12,847",
        change: "0.3%",
        changeDirection: "up" as const,
        description: "Number of actively traded cryptocurrencies"
    }
];

// Market dominance data for pie chart
const dominanceData = [
    { name: 'Bitcoin', value: 54.2, color: '#f7931a' },
    { name: 'Ethereum', value: 18.7, color: '#627eea' },
    { name: 'Others', value: 27.1, color: '#8b5cf6' }
];

function MarketOverviewSection() {
    return (
        <section className="flex flex-col gap-6">
            <h1 className="text-3xl font-bold">Explore Markets</h1>
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
                {marketStats.map((stat) => {
                    const ChangeIcon = stat.changeDirection === "up" ? MoveUpRight : MoveDownRight;
                    const changeColor = stat.changeDirection === "up" ? "#139E62" : "#dc2626";

                    return (
                        <Card key={stat.title}>
                            <CardHeader className="pb-2">
                                <CardTitle className="text-sm font-medium text-muted-foreground">
                                    {stat.title}
                                </CardTitle>
                            </CardHeader>
                            <CardContent>
                                <div className="text-2xl font-bold">{stat.value}</div>
                                <div className="flex items-center gap-1 text-sm mt-1">
                                    <ChangeIcon
                                        color={changeColor}
                                        strokeWidth={3}
                                        size={14}
                                    />
                                    <span style={{ color: changeColor }}>
                                        {stat.change}
                                    </span>
                                    <span className="text-muted-foreground">24h</span>
                                </div>
                                <p className="text-xs text-muted-foreground mt-2">
                                    {stat.description}
                                </p>
                            </CardContent>
                        </Card>
                    );
                })}
            </div>
        </section>
    );
}

function FilterSection() {
    return (
        <section className="flex flex-col gap-4">
            <h2 className="text-xl font-bold">Categories</h2>
            <ScrollArea className="w-full">
                <div className="flex gap-2 pb-2">
                    <Button variant="outline" className="cursor-pointer whitespace-nowrap">
                        <Flame className="h-4 w-4" /> Most Popular
                    </Button>
                    <Button variant="outline" className="cursor-pointer whitespace-nowrap">
                        <TrendingUp className="h-4 w-4" /> Gainers
                    </Button>
                    <Button variant="outline" className="cursor-pointer whitespace-nowrap">
                        <TrendingDown className="h-4 w-4" /> Losers
                    </Button>
                    <Button variant="outline" className="cursor-pointer whitespace-nowrap">
                        <CircleStop className="h-4 w-4" /> Stablecoins
                    </Button>
                    <Button variant="outline" className="cursor-pointer whitespace-nowrap">
                        <Coins className="h-4 w-4" /> DeFi
                    </Button>
                    <Button variant="outline" className="cursor-pointer whitespace-nowrap">
                        <Layers className="h-4 w-4" /> Layer 1
                    </Button>
                    <Button variant="outline" className="cursor-pointer whitespace-nowrap">
                        <Zap className="h-4 w-4" /> Layer 2
                    </Button>
                    <Button variant="outline" className="cursor-pointer whitespace-nowrap">
                        <Smile className="h-4 w-4" /> Memecoins
                    </Button>
                    <Button variant="outline" className="cursor-pointer whitespace-nowrap">
                        <Sparkles className="h-4 w-4" /> Newly Listed
                    </Button>
                    <Button variant="outline" className="cursor-pointer whitespace-nowrap">
                        <Percent className="h-4 w-4" /> Earn
                    </Button>
                </div>
            </ScrollArea>
        </section>
    );
}

function MiniSparkline({ data }: { data: { price: number }[] }) {
    const minPrice = Math.min(...data.map(d => d.price));
    const maxPrice = Math.max(...data.map(d => d.price));
    const isPositive = data[data.length - 1].price > data[0].price;

    return (
        <div className="h-12 w-20">
            <ChartContainer config={{}} className="h-full w-full">
                <LineChart data={data} margin={{ top: 2, right: 2, bottom: 2, left: 2 }}>
                    <Line
                        dataKey="price"
                        type="monotone"
                        stroke={isPositive ? "#139E62" : "#dc2626"}
                        strokeWidth={1.5}
                        dot={false}
                    />
                </LineChart>
            </ChartContainer>
        </div>
    );
}

function CryptoCardsGrid() {
    const router = useRouter();

    return (
        <section className="flex flex-col gap-6">
            <h2 className="text-xl font-bold">Quick Overview</h2>
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
                {cryptoAssets.slice(0, 8).map((asset) => {
                    const ChangeIcon = asset.change24hDirection === "up" ? MoveUpRight : MoveDownRight;
                    const changeColor = asset.change24hDirection === "up" ? "#139E62" : "#dc2626";

                    return (
                        <Card key={asset.short} className="p-4 cursor-pointer hover:shadow-lg transition-shadow" onClick={() => router.push(`/c/assets/${asset.short.toLowerCase()}`)}>
                            <div className="flex items-start justify-between mb-3">
                                <div className="flex items-center gap-3">
                                    <Image
                                        src={asset.image.src}
                                        alt={asset.image.alt}
                                        height={32}
                                        width={32}
                                    />
                                    <div>
                                        <div className="font-semibold">{asset.short}</div>
                                        <div className="text-xs text-muted-foreground">{asset.title}</div>
                                    </div>
                                </div>
                                <MiniSparkline data={asset.sparklineData} />
                            </div>
                            <div className="space-y-1">
                                <div className="text-lg font-bold">{asset.price}</div>
                                <div className="flex items-center gap-1 text-sm">
                                    <ChangeIcon
                                        color={changeColor}
                                        strokeWidth={3}
                                        size={12}
                                    />
                                    <span style={{ color: changeColor }}>{asset.change24h}</span>
                                </div>
                            </div>
                        </Card>
                    );
                })}
            </div>
        </section>
    );
}

function DetailedDataTable() {
    const router = useRouter();

    return (
        <section className="flex flex-col gap-6">
            <h2 className="text-xl font-bold">Market Data</h2>
            <div className="bg-card rounded-[20px] p-6">
                <Table>
                    <TableHeader className="[&_tr]:border-none [&_tr]:hover:bg-background">
                        <TableRow>
                            <TableHead className="w-[200px] text-left text-xs uppercase text-muted-foreground">
                                Asset
                            </TableHead>
                            <TableHead className="w-[120px] text-right text-xs uppercase text-muted-foreground">
                                Price
                            </TableHead>
                            <TableHead className="w-[100px] text-right text-xs uppercase text-muted-foreground">
                                1h
                            </TableHead>
                            <TableHead className="w-[100px] text-right text-xs uppercase text-muted-foreground">
                                24h
                            </TableHead>
                            <TableHead className="w-[100px] text-right text-xs uppercase text-muted-foreground">
                                7d
                            </TableHead>
                            <TableHead className="w-[120px] text-right text-xs uppercase text-muted-foreground">
                                Volume
                            </TableHead>
                            <TableHead className="w-[120px] text-right text-xs uppercase text-muted-foreground">
                                Market Cap
                            </TableHead>
                            <TableHead className="w-[100px] text-right text-xs uppercase text-muted-foreground">
                                Supply
                            </TableHead>
                            <TableHead className="w-[100px] text-center text-xs uppercase text-muted-foreground">
                                7d Chart
                            </TableHead>
                            <TableHead className="w-[50px] text-center text-xs uppercase text-muted-foreground">

                            </TableHead>
                        </TableRow>
                    </TableHeader>
                    <TableBody>
                        {cryptoAssets.map((asset, index) => {
                            const Change1hIcon = asset.change1hDirection === "up" ? MoveUpRight : MoveDownRight;
                            const Change24hIcon = asset.change24hDirection === "up" ? MoveUpRight : MoveDownRight;
                            const Change1wIcon = asset.change1wDirection === "up" ? MoveUpRight : MoveDownRight;

                            const positiveColor = "#139E62";
                            const negativeColor = "#dc2626";

                            console.log(`Rendering asset ${index}:`, { title: asset.title, short: asset.short });
                            return (
                                <TableRow key={asset.short} className="hover:bg-muted/50 cursor-pointer" onClick={() => router.push(`/c/assets/${asset.short.toLowerCase()}`)}>
                                    <TableCell className="w-[200px]">
                                        <div className="flex flex-row gap-4">
                                            <div className="flex items-center justify-center w-6 h-6 text-xs text-muted-foreground">
                                                {index + 1}
                                            </div>
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
                                    <TableCell className="w-[100px] text-right">
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
                                    <TableCell className="w-[100px] text-right">
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
                                    <TableCell className="w-[100px] text-right">
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
                                    <TableCell className="w-[120px] text-right">
                                        <span className="text-sm font-medium">{asset.volume}</span>
                                    </TableCell>
                                    <TableCell className="w-[120px] text-right">
                                        <span className="text-sm font-medium">{asset.marketCap}</span>
                                    </TableCell>
                                    <TableCell className="w-[100px] text-right">
                                        <span className="text-xs font-medium">{asset.circulatingSupply}</span>
                                    </TableCell>
                                    <TableCell className="w-[100px]">
                                        <div className="flex justify-center">
                                            <MiniSparkline data={asset.sparklineData} />
                                        </div>
                                    </TableCell>
                                    <TableCell className="w-[50px]">
                                        <div className="flex justify-center">
                                            <ClickableStar
                                                size={16}
                                                symbol={asset.short}
                                            />
                                        </div>
                                    </TableCell>
                                </TableRow>
                            );
                        })}
                    </TableBody>
                </Table>
            </div>
        </section>
    );
}

function MarketAnalysisSection() {
    const router = useRouter();

    return (<></>)

    // return (
    //     <>
    //         <section className="flex flex-col gap-6">
    //             <h2 className="text-xl font-bold">Market Analysis</h2>
    //             <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
    //                 {/* Market Dominance Chart */}
    //                 <Card>
    //                     <CardHeader>
    //                         <CardTitle>Market Dominance</CardTitle>
    //                     </CardHeader>
    //                     <CardContent>
    //                         <div className="h-[300px]">
    //                             <ChartContainer config={{}} className="h-full w-full">
    //                                 <PieChart>
    //                                     <Pie
    //                                         data={dominanceData}
    //                                         cx="50%"
    //                                         cy="50%"
    //                                         innerRadius={60}
    //                                         outerRadius={100}
    //                                         paddingAngle={2}
    //                                         dataKey="value"
    //                                     >
    //                                         {dominanceData.map((entry, index) => (
    //                                             <Cell key={`cell-${index}`} fill={entry.color} />
    //                                         ))}
    //                                     </Pie>
    //                                     <ChartTooltip
    //                                         content={
    //                                             <ChartTooltipContent
    //                                                 formatter={(value, name) => [`${value}%`, name]}
    //                                             />
    //                                         }
    //                                     />
    //                                 </PieChart>
    //                             </ChartContainer>
    //                         </div>
    //                         <div className="flex justify-center gap-4 mt-4">
    //                             {dominanceData.map((item) => (
    //                                 <div key={item.name} className="flex items-center gap-2">
    //                                     <div
    //                                         className="w-3 h-3 rounded-full"
    //                                         style={{ backgroundColor: item.color }}
    //                                     />
    //                                     <span className="text-sm">{item.name}</span>
    //                                     <span className="text-sm font-semibold">{item.value}%</span>
    //                                 </div>
    //                             ))}
    //                         </div>
    //                     </CardContent>
    //                 </Card>

    //                 {/* Top Performers vs Worst Performers */}
    //                 <Card>
    //                     <CardHeader>
    //                         <CardTitle>Performance Leaders</CardTitle>
    //                     </CardHeader>
    //                     <CardContent>
    //                         <div className="space-y-4">
    //                             <div>
    //                                 <h4 className="text-sm font-semibold text-green-600 mb-2">Top Gainers (24h)</h4>
    //                                 {cryptoAssets
    //                                     .filter(asset => asset.change24hDirection === "up")
    //                                     .sort((a, b) => parseFloat(b.change24h) - parseFloat(a.change24h))
    //                                     .slice(0, 3)
    //                                     .map((asset) => (
    //                                     <div key={asset.short} className="flex items-center justify-between py-1 cursor-pointer hover:bg-muted/50 px-2 rounded" onClick={() => router.push(`/c/assets/${asset.short.toLowerCase()}`)}
    //                                         <div className="flex items-center gap-2">
    //                                             <Image src={asset.image.src} alt={asset.image.alt} height={20} width={20} />
    //                                             <span className="text-sm font-medium">{asset.short}</span>
    //                                         </div>
    //                                         <span className="text-sm font-semibold text-green-600">+{asset.change24h}</span>
    //                                     </div>
    //                                 ))}
    //                         </div>

    //                         <div>
    //                             <h4 className="text-sm font-semibold text-red-600 mb-2">Top Losers (24h)</h4>
    //                             {cryptoAssets
    //                                 .filter(asset => asset.change24hDirection === "down")
    //                                 .sort((a, b) => parseFloat(a.change24h) - parseFloat(b.change24h))
    //                                 .slice(0, 3)
    //                                 .map((asset) => (
    //                                     <div key={asset.short} className="flex items-center justify-between py-1 cursor-pointer hover:bg-muted/50 px-2 rounded" onClick={() => router.push(`/c/assets/${asset.short.toLowerCase()}`)}
    //                                         <div className="flex items-center gap-2">
    //                                             <Image src={asset.image.src} alt={asset.image.alt} height={20} width={20} />
    //                                             <span className="text-sm font-medium">{asset.short}</span>
    //                                         </div>
    //                                         <span className="text-sm font-semibold text-red-600">{asset.change24h}</span>
    //                                     </div>
    //                                 ))}
    //                     </div>
    //             </div>
    //         </CardContent>
    //     </Card >
    //         </div >
    //     </section >
    //     </>
    // );
}

export default function Explore() {
    return (
        <div className="flex flex-col gap-8 p-6">
            <MarketOverviewSection />
            <FilterSection />
            <CryptoCardsGrid />
            <DetailedDataTable />
            <MarketAnalysisSection />
            <SiteFooter />
        </div>
    );
}