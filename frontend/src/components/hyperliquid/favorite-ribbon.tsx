import { cn } from "@/lib/utils";
import { Star } from "lucide-react";

interface HyperFavoriteRibbonProps {
  className?: string;
}

export function HyperFavoriteRibbon({ className }: HyperFavoriteRibbonProps) {
  return (
    <div
      className={cn(
        "flex w-full min-h-[40px] items-center gap-2.5 rounded-[5px] border border-[#12303c] bg-[#0b1b21] px-3 py-1.5 text-[11px] text-slate-300",
        className,
      )}
    >
      <Star className="h-3 w-3 text-[#ffb648]" />
      <span className="font-medium text-slate-200">Favorites</span>
      <div className="h-4 w-px bg-[#163947]" />
      <div className="flex flex-1 items-center gap-1.5 overflow-x-auto whitespace-nowrap">
        <Tag label="BTC" />
        <Tag label="ETH" />
        <Tag label="SOL" />
        <Tag label="ARB" />
        <span className="text-slate-500">Add more markets from Explore</span>
      </div>
    </div>
  );
}

function Tag({ label }: { label: string }) {
  return (
    <span className="rounded-full border border-[#163947] bg-[#081419] px-2.5 py-0.5 text-[10px] text-slate-200">
      {label}
    </span>
  );
}
