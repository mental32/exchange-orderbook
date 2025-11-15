export function HyperStatusBar() {
  return (
    <footer className=" h-7 flex items-center justify-between px-4 py-3 text-[11px] bg-[#1B2429]">
      <div className="flex items-center gap-2 mb-1">
        <span className="flex items-center gap-1 rounded-sm border border-[#50D2C1] bg-[#1E3C3B] px-2 py-1 text-[#50D2C1]">
          <span className="h-1 w-1 rounded-full bg-[#50D2C1]" />
          Online
        </span>
      </div>
      <div className="flex items-center gap-3 text-white">
        <button className="transition hover:text-slate-300">Docs</button>
        <button className="transition hover:text-slate-300">Support</button>
        <button className="transition hover:text-slate-300">Terms</button>
        <button className="transition hover:text-slate-300">
          Privacy Policy
        </button>
      </div>
    </footer>
  );
}
