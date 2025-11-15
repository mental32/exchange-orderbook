'use client';

import { useEffect, useRef } from "react";
import {
    CandlestickSeries,
    createChart,
    CrosshairMode,
    type CandlestickData,
} from "lightweight-charts";

import { cn } from "@/lib/utils";

interface HyperMarketCanvasProps {
    className?: string;
}

const SAMPLE_CANDLES: CandlestickData[] = [
    { time: "2024-10-14", open: 62350, high: 62870, low: 61580, close: 62040 },
    { time: "2024-10-15", open: 62040, high: 62610, low: 61220, close: 62410 },
    { time: "2024-10-16", open: 62410, high: 63320, low: 62100, close: 63080 },
    { time: "2024-10-17", open: 63080, high: 63600, low: 62410, close: 62740 },
    { time: "2024-10-18", open: 62740, high: 63290, low: 61850, close: 62190 },
    { time: "2024-10-21", open: 62190, high: 62950, low: 61930, close: 62660 },
    { time: "2024-10-22", open: 62660, high: 63480, low: 62210, close: 63340 },
    { time: "2024-10-23", open: 63340, high: 63980, low: 62930, close: 63760 },
    { time: "2024-10-24", open: 63760, high: 64490, low: 63210, close: 64130 },
    { time: "2024-10-25", open: 64130, high: 64670, low: 63550, close: 63800 },
    { time: "2024-10-28", open: 63800, high: 64290, low: 63180, close: 63640 },
    { time: "2024-10-29", open: 63640, high: 64350, low: 63330, close: 64180 },
    { time: "2024-10-30", open: 64180, high: 64880, low: 63740, close: 64710 },
    { time: "2024-10-31", open: 64710, high: 65260, low: 64250, close: 65090 },
    { time: "2024-11-01", open: 65090, high: 65670, low: 64580, close: 65420 },
    { time: "2024-11-04", open: 65420, high: 65990, low: 65010, close: 65750 },
    { time: "2024-11-05", open: 65750, high: 66230, low: 65360, close: 66080 },
    { time: "2024-11-06", open: 66080, high: 66520, low: 65640, close: 65810 },
    { time: "2024-11-07", open: 65810, high: 66330, low: 65370, close: 65560 },
    { time: "2024-11-08", open: 65560, high: 66190, low: 65210, close: 66010 },
];

export function HyperMarketCanvas({ className }: HyperMarketCanvasProps) {
    const containerRef = useRef<HTMLDivElement | null>(null);

    useEffect(() => {
        if (!containerRef.current) {
            return;
        }

        const chart = createChart(containerRef.current, {
            width: containerRef.current.clientWidth,
            height: containerRef.current.clientHeight,
            layout: {
                background: { color: "transparent" },
                textColor: "#a3b2c7",
            },
            crosshair: {
                mode: CrosshairMode.Normal,
            },
            grid: {
                horzLines: {
                    color: "rgba(38, 58, 77, 0.35)",
                },
                vertLines: {
                    color: "rgba(38, 58, 77, 0.25)",
                },
            },
            timeScale: {
                borderColor: "rgba(31, 45, 59, 0.6)",
            },
            rightPriceScale: {
                borderColor: "rgba(31, 45, 59, 0.6)",
            },
        });

        const series = chart.addSeries(CandlestickSeries, {
            upColor: "#22c55e",
            downColor: "#ef4444",
            borderVisible: false,
            wickUpColor: "#22c55e",
            wickDownColor: "#ef4444",
        });

        series.setData(SAMPLE_CANDLES);
        chart.timeScale().fitContent();

        const observer = new ResizeObserver((entries) => {
            const entry = entries[0];
            if (!entry || entry.contentRect.width === 0) {
                return;
            }

            chart.applyOptions({
                width: entry.contentRect.width,
                height: entry.contentRect.height,
            });
        });

        observer.observe(containerRef.current);

        return () => {
            observer.disconnect();
            chart.remove();
        };
    }, []);

    return (
        <div
            className={cn(
                "relative flex h-full min-h-[420px] w-full items-stretch overflow-hidden rounded-[5px] border border-[#12303c] bg-[#0e1a22]",
                className,
            )}
        >
            <div ref={containerRef} className="h-full w-full" />
        </div>
    );
}
