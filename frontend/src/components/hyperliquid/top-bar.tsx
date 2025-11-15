"use client";

import { Button } from "@/components/ui/button";
import {
  Menu,
  ChevronDown,
  User,
  Bell,
  Settings,
  MessageSquare,
} from "lucide-react";

const navItems = [
  "Trade",
  "Vaults",
  "Portfolio",
  "Staking",
  "Referrals",
  "Leaderboard",
];

export function HyperTopBar() {
  return (
    <header className="w-full border-b border-[#0f232f] bg-[#04141d]">
      <div className="flex h-12 items-center justify-between gap-6 px-4 lg:px-6">
        <div className="flex items-center gap-4">
          <div className="flex items-center gap-2">
            <span className="text-sm font-semibold tracking-wide text-white hover:text-gray-50">
              <a href="https://github.com/mental32/exchange-orderbook">
                mental32/exchange-orderbook
              </a>
            </span>
          </div>
          <nav className="hidden items-center gap-4 text-xs font-medium text-slate-300/80 lg:flex">
            {navItems.map((item) => (
              <Button
                key={item}
                className="rounded-md px-2 py-1 transition-colors hover:bg-white/5 hover:text-white"
              >
                {item}
              </Button>
            ))}
            <Button className="flex items-center gap-1 rounded-md px-2 py-1 text-slate-300/80 transition-colors hover:bg-white/5 hover:text-white">
              More
              <ChevronDown className="h-3 w-3" />
            </Button>
          </nav>
        </div>

        <div className="flex items-center gap-3">
          <Button
            size="sm"
            className="hidden rounded-md bg-[#44d6c2] px-4 py-2 text-xs font-semibold text-[#04141d] hover:bg-[#5cedd7] lg:inline-flex"
          >
            Deposit
          </Button>
          <div className="flex items-center gap-2">
            <button className="rounded-md p-2 text-slate-300/80 transition-colors hover:bg-white/5 hover:text-white">
              <MessageSquare className="h-4 w-4" />
            </button>
            <button className="rounded-md p-2 text-slate-300/80 transition-colors hover:bg-white/5 hover:text-white">
              <Bell className="h-4 w-4" />
            </button>
            <button className="rounded-md p-2 text-slate-300/80 transition-colors hover:bg-white/5 hover:text-white">
              <Settings className="h-4 w-4" />
            </button>
          </div>
          <div className="hidden items-center gap-2 rounded-md border border-white/10 bg-white/5 px-3 py-2 text-xs text-slate-200 lg:flex">
            <User className="h-4 w-4" />
            demo@user
          </div>
          <button className="inline-flex items-center gap-1 rounded-md border border-white/10 px-2 py-2 text-slate-300/80 transition-colors hover:bg-white/5 hover:text-white lg:hidden">
            <Menu className="h-4 w-4" />
          </button>
        </div>
      </div>
    </header>
  );
}
