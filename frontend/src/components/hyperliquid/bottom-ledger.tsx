import { cn } from "@/lib/utils";

const tabs = [
  "Balances",
  "Positions",
  "Open Orders",
  "TWAP",
  "Trade History",
  "Funding History",
  "Order History",
];

const headers = [
  "Time",
  "Type",
  "Coin",
  "Direction",
  "Size",
  "Filled Size",
  "Order Value",
  "Price",
  "Reduce Only",
  "Trigger Conditions",
  "TP/SL",
  "Status",
  "Order ID",
];

interface HyperBottomLedgerProps {
  className?: string;
}

export function HyperBottomLedger({ className }: HyperBottomLedgerProps) {
  return (
    <section
      className={cn(
        "space-y-[10px] rounded-[5px] border border-[#12303c] bg-[#0F1A1E] p-3",
        className,
      )}
    >
      <div className="flex flex-wrap items-center gap-1.5 text-[10px] uppercase tracking-wide text-slate-400">
        {tabs.map((tab) => (
          <button
            key={tab}
            className={`rounded-[5px] px-3 py-[6px] transition ${
              tab === "Order History"
                ? "bg-[#1d2f39] text-white"
                : "border border-transparent hover:border-white/10 hover:bg-white/5"
            }`}
          >
            {tab}
          </button>
        ))}
      </div>
      <div className="overflow-auto">
        <table className="min-w-full text-left text-[10px] text-slate-400">
          <thead>
            <tr className="border-b border-[#12303c]">
              {headers.map((header) => (
                <th
                  key={header}
                  className="px-2 py-2 font-medium uppercase tracking-wide"
                >
                  {header}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            <tr>
              <td
                colSpan={headers.length}
                className="px-2 py-6 text-center text-slate-500"
              >
                No historical orders yet
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </section>
  );
}
