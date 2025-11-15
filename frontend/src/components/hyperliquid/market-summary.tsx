import { cn } from "@/lib/utils";
import { Star } from "lucide-react";

const marketStats = [
  { label: "Mark", value: "109,879" },
  { label: "Oracle", value: "109,663" },
  { label: "24h Change", value: "+1,469 / +1.36%", accent: true },
  { label: "24h Volume", value: "$7,307,188,602.00" },
  { label: "Open Interest", value: "$2,563,201,375.26" },
  { label: "Funding / Countdown", value: "0.0013% / 00:32:32" },
];

interface HyperMarketSummaryProps {
  className?: string;
}

export function HyperMarketSummary({ className }: HyperMarketSummaryProps) {
  return (
    <section
      className={cn(
        "rounded-[5px] border border-[#12303c] bg-[#061922]",
        className,
      )}
    >
      <div className="flex min-h-[63px] flex-wrap items-center gap-x-4 gap-y-2 px-3 py-2 text-[11px] text-slate-200">
        <button className="flex items-center gap-2 rounded-[6px] border border-transparent bg-white/5 px-3 py-[6px] text-[12px] font-medium text-white transition hover:border-white/10">
          <Star className="h-3.5 w-3.5 text-[#44d6c2]" />
          BTC-USD
        </button>
        <div className="hidden h-5 w-px bg-[#12303c] lg:block" />
        {marketStats.map((stat) => (
          <div key={stat.label} className="flex flex-col gap-1">
            <span className="text-[11px] uppercase tracking-wide text-slate-400">
              {stat.label}
            </span>
            <span
              className={
                stat.accent
                  ? "text-[13px] font-semibold text-[#44d6c2]"
                  : "text-[13px] font-semibold text-slate-200"
              }
            >
              {stat.value}
            </span>
          </div>
        ))}
      </div>
    </section>
  );
}
