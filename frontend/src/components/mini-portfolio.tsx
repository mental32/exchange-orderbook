'use client'

import * as React from "react";
import { CartesianGrid, Line, LineChart, XAxis } from "recharts";
import {
    Card,
    CardContent,
    CardFooter,
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
import { Button } from "@/components/ui/button";
import { Separator } from "@/components/ui/separator";
import { ArrowDownToLine, ArrowLeftRight, ArrowUpFromLine, Minus, MoveUpRight, Plus } from "lucide-react";

function PointerButton({ icon, text }: { icon: React.ReactNode, text: string }) {
    return <div className="flex flex-col items-center justify-center">
        <Button
            className="md:max-w-[5rem] h-12 w-12 rounded-full cursor-pointer"
            variant={"outline"}
            size={"icon"}
        >
            {icon}
        </Button>
        <span>{text}</span>
    </div>
}

export function MiniPortfolio() {
    const chartData = [
        // Phase 1: Long period at ~£40
        { date: "2024-01-01", portfolioValue: 40.00 },
        { date: "2024-01-15", portfolioValue: 39.50 },
        { date: "2024-02-01", portfolioValue: 40.25 },
        { date: "2024-02-15", portfolioValue: 39.75 },
        { date: "2024-03-01", portfolioValue: 40.50 },
        { date: "2024-03-15", portfolioValue: 40.10 },
        { date: "2024-04-01", portfolioValue: 39.90 },
        { date: "2024-04-15", portfolioValue: 40.30 },
        { date: "2024-05-01", portfolioValue: 40.00 },
        
        // Phase 2: Sharp jump to ~£800 with fluctuations
        { date: "2024-05-15", portfolioValue: 820.00 },
        { date: "2024-06-01", portfolioValue: 795.50 },
        { date: "2024-06-15", portfolioValue: 810.25 },
        { date: "2024-07-01", portfolioValue: 785.75 },
        { date: "2024-07-15", portfolioValue: 805.50 },
        { date: "2024-08-01", portfolioValue: 792.30 },
        { date: "2024-08-15", portfolioValue: 815.80 },
        { date: "2024-09-01", portfolioValue: 798.60 },
        { date: "2024-09-15", portfolioValue: 808.40 },
        { date: "2024-10-01", portfolioValue: 801.20 },
        
        // Phase 3: Another jump to final value £1,234.97
        { date: "2024-10-15", portfolioValue: 1234.97 }
    ];

    return (
        <Card className="h-[25rem]">
            <CardHeader className="h-1/3 px-6">
                <div className="flex flex-col">
                    <CardTitle className="flex items-center p-0">
                        <HoverCard>
                            <HoverCardTrigger asChild>
                                <Button
                                    variant="link"
                                    className="p-0 text-muted-foreground underline decoration-wavy focus-visible:ring-0 focus-visible:outline-none cursor-pointer"
                                >
                                    Portfolio Value
                                </Button>
                            </HoverCardTrigger>
                            <HoverCardContent>
                                <span className="text-foreground">Portfolio Value</span>
                            </HoverCardContent>
                        </HoverCard>
                    </CardTitle>
                    <div className="flex items-baseline text-5xl">
                        <span className="text-muted-foreground">£</span>
                        <span style={{ color: 'oklch(0.25 0.133 284)' }}>1,234.97</span>
                    </div>
                    <div className="flex items-center gap-0.5 text-xs">
                        <MoveUpRight color="#35df8d" strokeWidth={3} size={12} />
                        <span className="text-[#139E62]">0.13% (£0.0025)</span>
                        <span className="text-muted-foreground">last month</span>
                    </div>
                </div>
            </CardHeader>
            <CardContent className="h-1/3 p-0">
                <ChartContainer config={{}} className="aspect-auto h-full w-full">
                    <LineChart
                        accessibilityLayer
                        data={chartData}
                        margin={{
                            left: 12,
                            right: 12,
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
                                    nameKey="views"
                                    labelFormatter={(value) => {
                                        return new Date(value).toLocaleDateString("en-US", {
                                            month: "short",
                                            day: "numeric",
                                            year: "numeric",
                                        });
                                    }}
                                />
                            }
                        />
                        <Line
                            dataKey="portfolioValue"
                            type="monotone"
                            stroke="#8b5cf6"
                            strokeWidth={2}
                            dot={false}
                        />
                    </LineChart>
                </ChartContainer>
            </CardContent>
            <CardFooter className="h-1/3 flex flex-row justify-between m-auto gap-6">
                <PointerButton icon={<Plus />} text={"Buy"} />
                <PointerButton icon={<Minus />} text={"Sell"} />
                <PointerButton icon={<ArrowLeftRight />} text={"Convert"} />
                <Separator orientation="vertical" />
                <PointerButton icon={<ArrowDownToLine />} text={"Deposit"} />
                <PointerButton icon={<ArrowUpFromLine />} text={"Withdraw"} />
            </CardFooter>
        </Card>
    );
}
