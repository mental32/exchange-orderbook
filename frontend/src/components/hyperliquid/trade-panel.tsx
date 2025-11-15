"use client";

import { useState } from "react";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { ChevronDown } from "lucide-react";

type Mode = "cross" | "twenty" | "oneway";

const modes: Array<{ key: Mode; label: string }> = [
  { key: "cross", label: "Cross" },
  { key: "twenty", label: "20x" },
  { key: "oneway", label: "One-Way" },
];

const modeCopy: Record<Mode, string> = {
  cross: "Cross margin active",
  twenty: "20x leverage preset",
  oneway: "One-way mode preset",
};

const orderTabs = ["Market", "Limit"] as const;
const sizeAssets = ["BTC", "USDC"] as const;

const positionRows = [
  { label: "Available to Trade", value: "0.00" },
  { label: "Current Position", value: "0.00000 BTC" },
];

const summaryRows = [
  { label: "Liquidation Price", value: "N/A", underline: true },
  { label: "Order Value", value: "N/A" },
  { label: "Margin Required", value: "N/A" },
  { label: "Slippage", value: "Est: 0% / Max: 0.80%", underline: true },
  { label: "Fees", value: "0.0450% / 0.0150%", underline: true },
];

interface HyperTradePanelProps {
  className?: string;
}

type OrderTab = (typeof orderTabs)[number];
type SizeAsset = (typeof sizeAssets)[number];

export function HyperTradePanel({ className }: HyperTradePanelProps) {
  const [mode, setMode] = useState<Mode>("cross");
  const [orderTab, setOrderTab] = useState<OrderTab>("Market");
  const [side, setSide] = useState<"buy" | "sell">("buy");
  const [sizeAsset, setSizeAsset] = useState<SizeAsset>("BTC");
  const [sizeValue, setSizeValue] = useState("");
  const [sizePercent, setSizePercent] = useState(0);
  const [reduceOnly, setReduceOnly] = useState(false);
  const [takeProfit, setTakeProfit] = useState(false);

  const tabHighlightStyle = {
    width: `calc(100% / ${orderTabs.length})`,
    transform: `translateX(${orderTabs.indexOf(orderTab) * 100}%)`,
  };

  const handlePercentChange = (value: number) => {
    const next = Number.isFinite(value) ? Math.min(100, Math.max(0, value)) : 0;
    setSizePercent(next);
  };

  const toggleAsset = () => {
    const currentIndex = sizeAssets.indexOf(sizeAsset);
    const nextAsset = sizeAssets[(currentIndex + 1) % sizeAssets.length];
    setSizeAsset(nextAsset);
  };

  return (
    <section
      className={cn(
        "relative flex h-full flex-col overflow-hidden rounded-[5px] border border-[#12303c] bg-[#0F1A1E] text-[12px] text-slate-200",
        className,
      )}
    >
      <div className="grid h-full grid-rows-[auto_1fr_auto] overflow-hidden">
        <div className="flex flex-col gap-1.5 px-3 pt-3 pb-2">
          <div className="grid grid-cols-3 gap-[8px] text-[10px] uppercase tracking-wide text-slate-400">
            {modes.map((item) => (
              <button
                key={item.key}
                type="button"
                onClick={() => setMode(item.key)}
                className={cn(
                  "h-7 rounded-[6px] border border-transparent bg-[#0F1A1E] px-0 text-center font-semibold transition hover:text-white",
                  mode === item.key &&
                    "border-[#1d2f39] bg-[#1d2f39] text-white",
                )}
              >
                {item.label}
              </button>
            ))}
          </div>
          <p className="text-[10px] text-slate-500">{modeCopy[mode]}</p>
        </div>

        <div className="hidden-scroll overflow-y-auto px-3 pb-3">
          <div className="flex flex-col gap-2.5">
            <div className="flex items-center gap-2 text-[11px] uppercase tracking-wide text-slate-400">
              <div className="relative flex flex-1 overflow-hidden">
                <span
                  className="pointer-events-none absolute bottom-0 left-0 h-[2px] bg-[#44d6c2] transition-transform duration-150 ease-out"
                  style={tabHighlightStyle}
                />
                {orderTabs.map((tab) => (
                  <button
                    key={tab}
                    type="button"
                    onClick={() => setOrderTab(tab)}
                    className={cn(
                      "flex-1 pb-2 text-left font-medium transition hover:text-white",
                      orderTab === tab ? "text-slate-100" : "text-slate-400",
                    )}
                  >
                    {tab}
                  </button>
                ))}
              </div>
              <button
                type="button"
                className="flex items-center gap-1 rounded-[5px] border border-[#12303c] bg-[#0F1A1E] px-2 py-1 text-[11px] text-slate-400 transition hover:text-white"
              >
                Pro
                <ChevronDown className="h-3 w-3" />
              </button>
            </div>

            <div className="grid grid-cols-2 gap-1 rounded-[6px] bg-[#0F1A1E] p-1 text-[11px] uppercase tracking-wide text-slate-400">
              <button
                type="button"
                onClick={() => setSide("buy")}
                className={cn(
                  "rounded-[5px] px-0 py-2 text-center font-semibold transition",
                  side === "buy"
                    ? "border border-[#44d6c2]/50 bg-[#1d2f39] text-[#44d6c2]"
                    : "border border-transparent hover:border-white/10 hover:text-white",
                )}
              >
                Buy / Long
              </button>
              <button
                type="button"
                onClick={() => setSide("sell")}
                className={cn(
                  "rounded-[5px] px-0 py-2 text-center font-semibold transition",
                  side === "sell"
                    ? "border border-rose-400/40 bg-[#1d2f39] text-rose-300"
                    : "border border-transparent hover:border-white/10 hover:text-white",
                )}
              >
                Sell / Short
              </button>
            </div>

            <div className="space-y-1 text-[11px]">
              {positionRows.map((row) => (
                <div
                  key={row.label}
                  className="flex items-center justify-between"
                >
                  <span className="text-slate-500">{row.label}</span>
                  <span className="text-slate-100">{row.value}</span>
                </div>
              ))}
            </div>

            <div className="space-y-1">
              <span className="text-[11px] uppercase tracking-wide text-slate-400">
                Size
              </span>
              <div className="flex items-center gap-2 rounded-[6px] border border-[#12303c] bg-[#0F1A1E] px-2 py-2">
                <input
                  value={sizeValue}
                  onChange={(event) => setSizeValue(event.target.value)}
                  inputMode="decimal"
                  placeholder="0.00"
                  className="w-full bg-transparent text-right text-[13px] font-medium text-slate-200 outline-none placeholder:text-slate-600"
                />
                <button
                  type="button"
                  onClick={toggleAsset}
                  className="flex items-center gap-1 text-[11px] font-medium text-slate-200 transition hover:text-white"
                >
                  {sizeAsset}
                  <ChevronDown className="h-3 w-3" />
                </button>
              </div>
            </div>

            <div className="grid grid-cols-[1fr_66px] items-center gap-2">
              <div className="rounded-[6px] border border-[#12303c] bg-[#0F1A1E] px-2 py-2">
                <input
                  type="range"
                  min={0}
                  max={100}
                  step={1}
                  value={sizePercent}
                  onChange={(event) =>
                    handlePercentChange(Number(event.target.value))
                  }
                  className="w-full accent-[#44d6c2]"
                />
                <div className="mt-1 flex justify-between text-[10px] text-slate-500">
                  {([0, 25, 50, 75, 100] as const).map((mark) => (
                    <span key={mark}>{mark}%</span>
                  ))}
                </div>
              </div>
              <div className="flex items-center justify-end gap-1 rounded-[6px] border border-[#12303c] bg-[#0F1A1E] px-2 py-2 text-[12px] text-slate-200">
                <input
                  type="number"
                  value={sizePercent}
                  onChange={(event) =>
                    handlePercentChange(Number(event.target.value))
                  }
                  className="w-full bg-transparent text-right outline-none"
                />
                <span className="text-slate-500">%</span>
              </div>
            </div>

            <div className="space-y-1">
              <ToggleRow
                label="Reduce Only"
                active={reduceOnly}
                onClick={() => setReduceOnly((prev) => !prev)}
              />
              <ToggleRow
                label="Take Profit / Stop Loss"
                active={takeProfit}
                onClick={() => setTakeProfit((prev) => !prev)}
              />
            </div>

            <Button
              className={cn(
                "h-10 rounded-[6px] text-[11px] font-semibold uppercase tracking-wide transition",
                side === "buy"
                  ? "bg-[#44d6c2] text-[#04141d] hover:bg-[#5cedd7]"
                  : "bg-rose-500 text-white hover:bg-rose-400",
              )}
            >
              {side === "buy" ? "Buy / Long" : "Sell / Short"}
            </Button>
          </div>
        </div>

        <div className="flex flex-col gap-3 border-t border-[#12303c] px-3 pb-3 pt-3">
          <Button className="h-10 rounded-[6px] bg-[#44d6c2] text-[11px] font-semibold uppercase tracking-wide text-[#04141d] hover:bg-[#5cedd7]">
            Deposit
          </Button>

          <div className="h-px bg-[#12303c]" />

          <div className="space-y-2 text-[11px] text-slate-300">
            {summaryRows.map((row) => (
              <div
                key={row.label}
                className="flex items-center justify-between"
              >
                <span
                  className={cn(
                    "text-slate-400",
                    row.underline && "underline decoration-slate-500",
                  )}
                >
                  {row.label}
                </span>
                <span className="text-slate-200">{row.value}</span>
              </div>
            ))}
          </div>
        </div>
      </div>
    </section>
  );
}

interface ToggleRowProps {
  active: boolean;
  label: string;
  onClick: () => void;
}

function ToggleRow({ active, label, onClick }: ToggleRowProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        "flex items-center gap-2 rounded-[6px] px-2 py-2 text-left text-[11px] transition",
        active
          ? "bg-[#1d2f39] text-slate-100"
          : "bg-transparent text-slate-300 hover:bg-white/5",
      )}
    >
      <span
        className={cn(
          "inline-flex h-3 w-3 items-center justify-center rounded border border-[#163947]",
          active && "border-[#44d6c2] bg-[#44d6c2]",
        )}
      />
      <span>{label}</span>
    </button>
  );
}
