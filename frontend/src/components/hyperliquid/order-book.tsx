"use client";

import { useMemo } from "react";

import { cn } from "@/lib/utils";

const orderBookData: {
  asks: {
    price: string;
    size: string;
    btc: string;
  }[];
  bids: {
    price: string;
    size: string;
    btc: string;
  }[];
} = {
  asks: [
    { price: "120,000", size: "13.87734", btc: "1.88089785" },
    { price: "119,000", size: "363.065", btc: "0.18602801" },
    { price: "118,000", size: "76.4372", btc: "1.45321892" },
    { price: "117,000", size: "90.4629", btc: "1.42751567" },
    { price: "116,000", size: "45.117", btc: "0.33794034" },
    { price: "115,000", size: "36.297", btc: "0.32500125" },
    { price: "114,000", size: "52.736", btc: "1.25603966" },
    { price: "113,000", size: "147.871", btc: "2.05028708" },
    { price: "112,000", size: "104.819", btc: "1.05502587" },
    { price: "111,000", size: "583.328", btc: "0.95060205" },
  ],
  bids: [
    { price: "109,000", size: "829.305", btc: "0.829305" },
    { price: "108,000", size: "368.140", btc: "1.19744597" },
    { price: "107,000", size: "147.743", btc: "1.34518942" },
    { price: "106,000", size: "213.513", btc: "1.55867029" },
    { price: "105,000", size: "178.030", btc: "1.73673322" },
    { price: "104,000", size: "250.995", btc: "1.98772082" },
    { price: "103,000", size: "252.734", btc: "2.24046351" },
    { price: "102,000", size: "400.475", btc: "2.64093872" },
    { price: "101,000", size: "363.277", btc: "0.36403932" },
    { price: "100,000", size: "144.965", btc: "3.14918261" },
  ],
} as const;

const numberFrom = (value: string) => Number(value.replace(/,/g, ""));

function withDepth(
  rows: readonly (typeof orderBookData.asks)[number][],
  max: number,
) {
  return rows.map((row) => ({
    ...row,
    depth: max ? Math.min(100, (numberFrom(row.size) / max) * 100) : 0,
  }));
}

interface HyperOrderBookProps {
  className?: string;
}

export function HyperOrderBook({ className }: HyperOrderBookProps) {
  const spread = useMemo(() => ({ price: "1,000", pct: "0.913%" }), []);
  const { asks, bids } = useMemo(() => {
    const maxAsk = Math.max(
      ...orderBookData.asks.map((row) => numberFrom(row.size)),
    );
    const maxBid = Math.max(
      ...orderBookData.bids.map((row) => numberFrom(row.size)),
    );

    return {
      asks: withDepth(orderBookData.asks, maxAsk),
      bids: withDepth(orderBookData.bids, maxBid),
    };
  }, []);

  return (
    <section className={cn("flex h-full flex-col", className)}>
      <header className="flex items-center justify-between border-b border-[#12303c] px-3 py-2 text-[11px]">
        <div className="flex items-center gap-3">
          <select className="rounded-[4px] border border-[#12303c] bg-[#050f17] px-2 py-1 text-slate-200">
            <option value="1000">1,000</option>
            <option value="100">100</option>
            <option value="10">10</option>
          </select>
          <div className="flex items-center gap-2.5 text-slate-400">
            <span className="font-medium text-slate-100">Order Book</span>
            <button className="rounded-md px-2 py-1 transition hover:bg-white/5 hover:text-white">
              Trades
            </button>
          </div>
        </div>
        <button className="rounded-md px-2 py-1 text-slate-400 transition hover:bg-white/5 hover:text-white">
          ···
        </button>
      </header>
      <div className="flex items-center justify-between px-3 pt-1 pb-[2px] text-[10px] uppercase tracking-[0.08em] text-slate-500">
        <span className="w-[34%]">Price</span>
        <span className="w-[33%] text-right">Size (BTC)</span>
        <span className="w-[33%] text-right">BTC</span>
      </div>
      <div className="flex-1 overflow-hidden">
        <div className="overflow-y-auto overflow-x-hidden px-3 pb-2">
          {asks.map((row) => (
            <Row key={`ask-${row.price}`} tone="ask" {...row} />
          ))}
          <div className="mt-1 flex items-center justify-between border-y border-[#12303c] py-1.5 text-[10px] uppercase tracking-[0.08em] text-slate-400">
            <span className="w-[34%]">Spread</span>
            <span className="w-[33%] text-right text-slate-300">
              {spread.price}
            </span>
            <span className="w-[33%] text-right text-[#44d6c2]">
              {spread.pct}
            </span>
          </div>
          {bids.map((row) => (
            <Row key={`bid-${row.price}`} tone="bid" {...row} />
          ))}
        </div>
      </div>
    </section>
  );
}

function DepthBar({ tone, depth }: { tone: "ask" | "bid"; depth: number }) {
  const tint = tone === "ask" ? "bg-[#ed7088]/22" : "bg-[#13302D]";
  const width = Math.max(depth, 4);
  return (
    <span
      className={cn(
        "pointer-events-none absolute inset-y-[4px] left-0 z-0 rounded-r",
        tint,
      )}
      style={{ width: `${width}%` }}
    />
  );
}

interface RowProps {
  price: string;
  size: string;
  btc: string;
  tone: "ask" | "bid";
  depth: number;
}

function Row({ price, size, btc, tone, depth }: RowProps) {
  return (
    <div className="relative flex items-center overflow-hidden py-[4px] text-[11px] font-medium">
      <DepthBar tone={tone} depth={depth} />
      <span className="relative z-10 w-[34%] text-left font-mono tabular-nums text-slate-200">
        {price}
      </span>
      <span className="relative z-10 w-[33%] text-right font-mono tabular-nums text-slate-300">
        {size}
      </span>
      <span className="relative z-10 w-[33%] text-right font-mono tabular-nums text-slate-300">
        {btc}
      </span>
    </div>
  );
}
