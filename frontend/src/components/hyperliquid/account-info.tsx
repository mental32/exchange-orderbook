"use client";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

interface HyperAccountInfoProps {
  className?: string;
}

const overviewRows: Array<{ label: string; value: string; accent?: boolean }> =
  [
    { label: "Balance", value: "$0.00" },
    { label: "Unrealized PNL", value: "$0.00" },
    { label: "Cross Margin Ratio", value: "0.00%", accent: true },
    { label: "Maintenance Margin", value: "$0.00" },
    { label: "Cross Account Leverage", value: "0.00x" },
  ];

export function HyperAccountInfo({ className }: HyperAccountInfoProps) {
  return (
    <section className={cn("flex h-full flex-col text-white", className)}>
      <Button className="w-full rounded-md bg-[#44d6c2] text-xs font-semibold uppercase tracking-wide text-[#04141d] hover:bg-[#5cedd7]">
        Deposit
      </Button>
      <div className="grid grid-cols-2 gap-3 text-xs uppercase tracking-wide">
        <Button variant="outline" className="rounded-md">
          Perps ➜ Spot
        </Button>
        <Button variant="outline" className="rounded-md">
          Withdraw
        </Button>
      </div>
      <div className="border border-b border-gray-500"></div>
      <div className="grid gap-4 text-[11px] text-slate-300">
        <InfoGroup
          title="Account Equity"
          rows={[
            { label: "Spot", value: "$0.00" },
            { label: "Perps", value: "$0.00" },
          ]}
        />
        <InfoGroup
          title="Perps Overview"
          rows={overviewRows.map((row) => ({
            label: row.label,
            value: row.value,
            accent: row.accent,
          }))}
        />
      </div>
    </section>
  );
}

interface InfoGroupProps {
  title: string;
  rows: Array<{ label: string; value: string; accent?: boolean }>;
}

function InfoGroup({ title, rows }: InfoGroupProps) {
  return (
    <div className="space-y-3">
      <div className="text-xs tracking-wide text-white">{title}</div>
      <div className="space-y-2">
        {rows.map((row) => (
          <div key={row.label} className="flex items-center justify-between">
            <span className="text-[#949E9C]">{row.label}</span>
            <span className={row.accent ? "text-[#50d2c1]" : "text-slate-200"}>
              {row.value}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}
