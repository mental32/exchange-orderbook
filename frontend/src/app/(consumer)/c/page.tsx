import GoodMorning from "@/components/good-morning";
import { MiniPortfolio } from "@/components/mini-portfolio";
import { Watchlist } from "@/components/watchlist";
import { MiniExplore } from "@/components/mini-explore";
import { ScrollArea } from "@/components/ui/scroll-area";
import SiteFooter from "@/components/site-footer";

export default function Home() {
    return (
        <div className="grid gap-y-[1rem]">
            <GoodMorning />
            <ScrollArea>
                <section className="grid gap-y-4">
                    <MiniPortfolio />
                    <Watchlist />
                    <MiniExplore />
                </section>
            </ScrollArea>
            <SiteFooter />
        </div>
    );
}
