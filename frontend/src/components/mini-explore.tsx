import { Button } from "@/components/ui/button";
import {
    Table,
    TableBody,
    TableCell,
    TableHead,
    TableHeader,
    TableRow,
} from "@/components/ui/table"
import { CircleStop, Flame, MoveUpRight, Percent, Sparkles, TrendingDown, TrendingUp } from "lucide-react";
import Image from 'next/image';
import ClickableStar from "@/components/clickable-star";

const assets = [
    {
        title: "Bitcoin",
        short: "BTC",
        price: "£72,846.59",
        change: "0.54%",
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
    image: { src: string, alt: string }
}

function Row({ asset }: { asset: Asset }) {
    return (
        <TableRow className="">
            <TableCell className="w-[150px]">
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
                        <MoveUpRight color="#35df8d" strokeWidth={3} size={12} />
                        <span className="text-[#35df8d]">{asset.change}</span>
                    </div>
                </div>
            </TableCell>
            <TableCell>
                <Button variant={"secondary"}>Buy</Button>
            </TableCell>
            <TableCell>
                <ClickableStar size={24} />
            </TableCell>
        </TableRow>
    );
}

export function MiniExplore() {
    return (
        <section className="flex flex-col gap-[1rem]">
            <div className="text-xl font-bold">Explore the market</div>
            <div className="flex gap-2">
                <Button variant="secondary">
                    <Flame color="#ffffff" /> Most Popular
                </Button>
                <Button variant="secondary">
                    <Percent color="#ffffff" />
                    Earn
                </Button>
                <Button variant="secondary">
                    <TrendingUp color="#ffffff" /> Gainers
                </Button>
                <Button variant="secondary">
                    <TrendingDown color="#ffffff" /> Losers
                </Button>
                <Button variant="secondary">
                    <CircleStop color="#ffffff" />
                    Stabelcoins
                </Button>
                <Button variant="secondary">
                    <Sparkles color="#ffffff" />
                    Newly Listed
                </Button>
            </div>
            <div className="bg-card rounded-[20px] p-6">
                <Table className="">
                    <TableHeader className="[&_tr]:border-none [&_tr]:hover:bg-background [&_tr]:hover:cursor-pointer">
                        <TableRow className="flex">
                            <TableHead className="w-[150px] min-w-[20px] text-left">
                                Asset
                            </TableHead>
                            <TableHead className="w-[120px] min-w-[20px] text-right">
                                Price
                            </TableHead>
                            <TableHead> </TableHead>
                            <TableHead> </TableHead>
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