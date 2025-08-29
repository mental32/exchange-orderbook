'use client'

import * as React from "react"
import { useParams, notFound } from 'next/navigation'
import { CartesianGrid, Line, LineChart, XAxis, YAxis, ResponsiveContainer } from "recharts"
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
} from "@/components/ui/card"
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
} from "@/components/ui/chart"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { ScrollArea } from "@/components/ui/scroll-area"
import { 
  MoveUpRight, 
  MoveDownRight,
  TrendingUp,
  TrendingDown,
  Activity,
  DollarSign,
  Percent,
  BarChart3,
  Info,
  Star
} from "lucide-react"
import Image from 'next/image'
import ClickableStar from "@/components/clickable-star"

// Asset data mapping
const assetsData = {
  btc: {
    title: "Bitcoin",
    short: "BTC",
    description: "The world's first and largest cryptocurrency",
    price: "£72,846.59",
    change24h: "2.54%",
    change24hDirection: "up" as const,
    change1h: "0.12%",
    change1hDirection: "up" as const,
    change1w: "8.18%",
    change1wDirection: "up" as const,
    volume24h: "£28.4B",
    marketCap: "£1.44T",
    circulatingSupply: "19.8M BTC",
    totalSupply: "21M BTC",
    allTimeHigh: "£73,750.07",
    allTimeLow: "£52.23",
    dominance: "54.2%",
    image: {
      src: "/bitcoin-btc-logo.svg",
      alt: "Bitcoin logo"
    }
  },
  eth: {
    title: "Ethereum",
    short: "ETH",
    description: "A decentralized platform for smart contracts",
    price: "£1,381.07",
    change24h: "1.44%",
    change24hDirection: "up" as const,
    change1h: "-0.08%",
    change1hDirection: "down" as const,
    change1w: "5.92%",
    change1wDirection: "up" as const,
    volume24h: "£12.8B",
    marketCap: "£166B",
    circulatingSupply: "120.4M ETH",
    totalSupply: "120.4M ETH",
    allTimeHigh: "£3,456.23",
    allTimeLow: "£0.34",
    dominance: "18.7%",
    image: {
      src: "/eth-glyph-colored.svg",
      alt: "Ether diamond colored logo"
    }
  },
  sol: {
    title: "Solana",
    short: "SOL",
    description: "High performance blockchain supporting builders around the world",
    price: "£194.73",
    change24h: "3.21%",
    change24hDirection: "up" as const,
    change1h: "0.45%",
    change1hDirection: "up" as const,
    change1w: "12.34%",
    change1wDirection: "up" as const,
    volume24h: "£2.1B",
    marketCap: "£91.2B",
    circulatingSupply: "468.5M SOL",
    totalSupply: "574.2M SOL",
    allTimeHigh: "£260.06",
    allTimeLow: "£0.38",
    dominance: "3.4%",
    image: {
      src: "/solana-sol-logo.svg",
      alt: "Solana logo"
    }
  },
  xrp: {
    title: "XRP",
    short: "XRP",
    description: "Digital payment protocol and cryptocurrency",
    price: "£1.67",
    change24h: "0.31%",
    change24hDirection: "up" as const,
    change1h: "-0.15%",
    change1hDirection: "down" as const,
    change1w: "-1.84%",
    change1wDirection: "down" as const,
    volume24h: "£3.2B",
    marketCap: "£95.8B",
    circulatingSupply: "57.4B XRP",
    totalSupply: "100B XRP",
    allTimeHigh: "£2.78",
    allTimeLow: "£0.0027",
    dominance: "2.1%",
    image: {
      src: "/xrp-xrp-logo.svg",
      alt: "XRP logo"
    }
  },
  ada: {
    title: "Cardano",
    short: "ADA",
    description: "Blockchain platform for changemakers and innovators",
    price: "£0.3184",
    change24h: "-0.72%",
    change24hDirection: "down" as const,
    change1h: "0.23%",
    change1hDirection: "up" as const,
    change1w: "-4.56%",
    change1wDirection: "down" as const,
    volume24h: "£245M",
    marketCap: "£11.4B",
    circulatingSupply: "35.8B ADA",
    totalSupply: "45B ADA",
    allTimeHigh: "£2.45",
    allTimeLow: "£0.017",
    dominance: "0.8%",
    image: {
      src: "/cardano-ada-logo.svg",
      alt: "Cardano logo"
    }
  },
  usdt: {
    title: "Tether USD",
    short: "USDT",
    description: "Stablecoin pegged to the US Dollar",
    price: "£0.7534",
    change24h: "0.26%",
    change24hDirection: "up" as const,
    change1h: "0.01%",
    change1hDirection: "up" as const,
    change1w: "-0.02%",
    change1wDirection: "down" as const,
    volume24h: "£42.1B",
    marketCap: "£95.2B",
    circulatingSupply: "126.3B USDT",
    totalSupply: "126.3B USDT",
    allTimeHigh: "£0.76",
    allTimeLow: "£0.74",
    dominance: "4.2%",
    image: {
      src: "/tether-usdt-logo.svg",
      alt: "Tether USD"
    }
  },
  usdc: {
    title: "USD Coin",
    short: "USDC",
    description: "Digital dollar stablecoin",
    price: "£0.7532",
    change24h: "0.24%",
    change24hDirection: "up" as const,
    change1h: "0.02%",
    change1hDirection: "up" as const,
    change1w: "-0.01%",
    change1wDirection: "down" as const,
    volume24h: "£5.8B",
    marketCap: "£25.1B",
    circulatingSupply: "33.3B USDC",
    totalSupply: "33.3B USDC",
    allTimeHigh: "£0.76",
    allTimeLow: "£0.74",
    dominance: "1.1%",
    image: {
      src: "/usd-coin-usdc-logo.svg",
      alt: "USD Coin"
    }
  }
}

// Mock chart data
const generateChartData = (basePrice: number) => {
  const data = []
  const now = Date.now()
  for (let i = 30; i >= 0; i--) {
    const variance = (Math.random() - 0.5) * basePrice * 0.02
    data.push({
      time: new Date(now - i * 24 * 60 * 60 * 1000).toLocaleDateString(),
      price: basePrice + variance
    })
  }
  return data
}

// Mock order book data
const generateOrderBook = () => ({
  bids: Array.from({ length: 10 }, (_, i) => ({
    price: 72846.59 - (i + 1) * 50,
    amount: (Math.random() * 2).toFixed(4),
    total: ((72846.59 - (i + 1) * 50) * Math.random() * 2).toFixed(2)
  })),
  asks: Array.from({ length: 10 }, (_, i) => ({
    price: 72846.59 + (i + 1) * 50,
    amount: (Math.random() * 2).toFixed(4),
    total: ((72846.59 + (i + 1) * 50) * Math.random() * 2).toFixed(2)
  }))
})

// Mock recent trades
const generateRecentTrades = (symbol: string) => 
  Array.from({ length: 20 }, (_, i) => ({
    time: new Date(Date.now() - i * 60000).toLocaleTimeString(),
    price: `£${(72846.59 + (Math.random() - 0.5) * 100).toFixed(2)}`,
    amount: `${(Math.random() * 0.5).toFixed(4)} ${symbol}`,
    side: Math.random() > 0.5 ? 'buy' as const : 'sell' as const
  }))

export default function AssetDetailPage() {
  const params = useParams()
  const symbol = params.symbol as string
  const asset = assetsData[symbol as keyof typeof assetsData]
  const [timeRange, setTimeRange] = React.useState('1m')
  const [tradeMode, setTradeMode] = React.useState<'buy' | 'sell'>('buy')

  if (!asset) {
    notFound()
  }

  const chartData = generateChartData(parseFloat(asset.price.replace(/[£,]/g, '')))
  const orderBook = generateOrderBook()
  const recentTrades = generateRecentTrades(asset.short)

  const positiveColor = "#139E62"
  const negativeColor = "#dc2626"
  
  const timeRanges = [
    { value: '1d', label: '1D' },
    { value: '1w', label: '1W' },
    { value: '1m', label: '1M' },
    { value: '3m', label: '3M' },
    { value: '1y', label: '1Y' },
    { value: 'all', label: 'ALL' },
  ]

  return (
    <div className="container mx-auto p-6 space-y-6">
      {/* Header Section */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-4">
          <Image src={asset.image.src} alt={asset.image.alt} height={48} width={48} />
          <div>
            <h1 className="text-3xl font-bold flex items-center gap-2">
              {asset.title}
              <span className="text-muted-foreground text-xl">({asset.short})</span>
              <ClickableStar symbol={asset.short} size={24} />
            </h1>
            <p className="text-muted-foreground">{asset.description}</p>
          </div>
        </div>
        <div className="text-right">
          <div className="text-3xl font-bold">{asset.price}</div>
          <div className="flex items-center gap-1 text-sm">
            {asset.change24hDirection === 'up' ? (
              <MoveUpRight color={positiveColor} strokeWidth={3} size={16} />
            ) : (
              <MoveDownRight color={negativeColor} strokeWidth={3} size={16} />
            )}
            <span style={{ color: asset.change24hDirection === 'up' ? positiveColor : negativeColor }}>
              {asset.change24h} (24h)
            </span>
          </div>
        </div>
      </div>

      {/* Market Stats Grid */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Market Cap</CardTitle>
            <DollarSign className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{asset.marketCap}</div>
            <p className="text-xs text-muted-foreground">Rank #1</p>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">24h Volume</CardTitle>
            <Activity className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{asset.volume24h}</div>
            <p className="text-xs text-muted-foreground">Volume/MCap: 0.0197</p>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Circulating Supply</CardTitle>
            <BarChart3 className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{asset.circulatingSupply}</div>
            <p className="text-xs text-muted-foreground">Total: {asset.totalSupply}</p>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Dominance</CardTitle>
            <Percent className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{asset.dominance}</div>
            <p className="text-xs text-muted-foreground">Of total market cap</p>
          </CardContent>
        </Card>
      </div>

      {/* Price Chart and Trading Interface */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Price Chart */}
        <Card className="lg:col-span-2">
          <CardHeader>
            <CardTitle>Price Chart</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="flex gap-1">
              {timeRanges.map((range) => (
                <Button
                  key={range.value}
                  variant={timeRange === range.value ? "default" : "outline"}
                  size="sm"
                  onClick={() => setTimeRange(range.value)}
                >
                  {range.label}
                </Button>
              ))}
            </div>
            <ChartContainer
              config={{
                price: {
                  label: "Price",
                  color: "hsl(var(--chart-1))",
                },
              }}
              className="h-[400px]"
            >
              <ResponsiveContainer width="100%" height="100%">
                <LineChart data={chartData}>
                  <CartesianGrid strokeDasharray="3 3" />
                  <XAxis dataKey="time" />
                  <YAxis />
                  <ChartTooltip content={<ChartTooltipContent />} />
                  <Line
                    type="monotone"
                    dataKey="price"
                    stroke="var(--color-price)"
                    strokeWidth={2}
                    dot={false}
                  />
                </LineChart>
              </ResponsiveContainer>
            </ChartContainer>
          </CardContent>
        </Card>

        {/* Trading Interface */}
        <Card>
          <CardHeader>
            <CardTitle>Trade {asset.short}</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="grid grid-cols-2 gap-2">
              <Button
                variant={tradeMode === 'buy' ? 'default' : 'outline'}
                onClick={() => setTradeMode('buy')}
              >
                Buy
              </Button>
              <Button
                variant={tradeMode === 'sell' ? 'default' : 'outline'}
                onClick={() => setTradeMode('sell')}
              >
                Sell
              </Button>
            </div>
            
            {tradeMode === 'buy' ? (
              <div className="space-y-4">
                <div className="space-y-2">
                  <Label htmlFor="buy-amount">Amount ({asset.short})</Label>
                  <Input id="buy-amount" placeholder="0.00" type="number" />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="buy-price">Price (GBP)</Label>
                  <Input id="buy-price" placeholder={asset.price} type="number" />
                </div>
                <div className="space-y-2">
                  <Label>Total (GBP)</Label>
                  <Input placeholder="0.00" disabled />
                </div>
                <Button className="w-full" size="lg">
                  Buy {asset.short}
                </Button>
              </div>
            ) : (
              <div className="space-y-4">
                <div className="space-y-2">
                  <Label htmlFor="sell-amount">Amount ({asset.short})</Label>
                  <Input id="sell-amount" placeholder="0.00" type="number" />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="sell-price">Price (GBP)</Label>
                  <Input id="sell-price" placeholder={asset.price} type="number" />
                </div>
                <div className="space-y-2">
                  <Label>Total (GBP)</Label>
                  <Input placeholder="0.00" disabled />
                </div>
                <Button className="w-full" size="lg" variant="destructive">
                  Sell {asset.short}
                </Button>
              </div>
            )}
          </CardContent>
        </Card>
      </div>

      {/* Order Book and Recent Trades */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Order Book */}
        <Card>
          <CardHeader>
            <CardTitle>Order Book</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="grid grid-cols-2 gap-4">
              <div>
                <h3 className="text-sm font-medium text-green-600 mb-2">Bids</h3>
                <ScrollArea className="h-[300px]">
                  <Table>
                    <TableHeader>
                      <TableRow>
                        <TableHead className="text-xs">Price</TableHead>
                        <TableHead className="text-xs">Amount</TableHead>
                        <TableHead className="text-xs">Total</TableHead>
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {orderBook.bids.map((bid, i) => (
                        <TableRow key={i}>
                          <TableCell className="text-xs text-green-600">£{bid.price}</TableCell>
                          <TableCell className="text-xs">{bid.amount}</TableCell>
                          <TableCell className="text-xs">£{bid.total}</TableCell>
                        </TableRow>
                      ))}
                    </TableBody>
                  </Table>
                </ScrollArea>
              </div>
              <div>
                <h3 className="text-sm font-medium text-red-600 mb-2">Asks</h3>
                <ScrollArea className="h-[300px]">
                  <Table>
                    <TableHeader>
                      <TableRow>
                        <TableHead className="text-xs">Price</TableHead>
                        <TableHead className="text-xs">Amount</TableHead>
                        <TableHead className="text-xs">Total</TableHead>
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {orderBook.asks.map((ask, i) => (
                        <TableRow key={i}>
                          <TableCell className="text-xs text-red-600">£{ask.price}</TableCell>
                          <TableCell className="text-xs">{ask.amount}</TableCell>
                          <TableCell className="text-xs">£{ask.total}</TableCell>
                        </TableRow>
                      ))}
                    </TableBody>
                  </Table>
                </ScrollArea>
              </div>
            </div>
          </CardContent>
        </Card>

        {/* Recent Trades */}
        <Card>
          <CardHeader>
            <CardTitle>Recent Trades</CardTitle>
          </CardHeader>
          <CardContent>
            <ScrollArea className="h-[350px]">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead className="text-xs">Time</TableHead>
                    <TableHead className="text-xs">Price</TableHead>
                    <TableHead className="text-xs">Amount</TableHead>
                    <TableHead className="text-xs">Side</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {recentTrades.map((trade, i) => (
                    <TableRow key={i}>
                      <TableCell className="text-xs">{trade.time}</TableCell>
                      <TableCell className="text-xs">{trade.price}</TableCell>
                      <TableCell className="text-xs">{trade.amount}</TableCell>
                      <TableCell className={`text-xs ${trade.side === 'buy' ? 'text-green-600' : 'text-red-600'}`}>
                        {trade.side.toUpperCase()}
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </ScrollArea>
          </CardContent>
        </Card>
      </div>

      {/* Additional Information */}
      <Card>
        <CardHeader>
          <CardTitle>About {asset.title}</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
            <div>
              <p className="text-sm text-muted-foreground">All Time High</p>
              <p className="font-medium">{asset.allTimeHigh}</p>
            </div>
            <div>
              <p className="text-sm text-muted-foreground">All Time Low</p>
              <p className="font-medium">{asset.allTimeLow}</p>
            </div>
            <div>
              <p className="text-sm text-muted-foreground">1 Hour</p>
              <p className={`font-medium ${asset.change1hDirection === 'up' ? 'text-green-600' : 'text-red-600'}`}>
                {asset.change1h}
              </p>
            </div>
            <div>
              <p className="text-sm text-muted-foreground">1 Week</p>
              <p className={`font-medium ${asset.change1wDirection === 'up' ? 'text-green-600' : 'text-red-600'}`}>
                {asset.change1w}
              </p>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  )
}