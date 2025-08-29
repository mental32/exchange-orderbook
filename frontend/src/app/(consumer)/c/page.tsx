import { auth, clerkClient } from '@clerk/nextjs/server'
import GoodMorning from "@/components/good-morning";
import { MiniPortfolio } from "@/components/mini-portfolio";
import { Watchlist } from "@/components/watchlist";
import { MiniExplore } from "@/components/mini-explore";
import { ScrollArea } from "@/components/ui/scroll-area";
import SiteFooter from "@/components/site-footer";

export default async function Home() {
    const { sessionId } = await auth()

    // Example of getting JWT token for backend API calls
    // if (sessionId) {
    //     try {
    //         const client = await clerkClient()
    //         const token = await client.sessions.getToken(sessionId, 'default')
    //         // Example: fetch user portfolio data from backend
    //         const response = await fetch(`${process.env.NEXT_PUBLIC_BACKEND_URL}/api/portfolio`, {
    //             headers: {
    //                 'Authorization': `Bearer ${token}`
    //             }
    //         })
    //         // Handle response...
    //     } catch (error) {
    //         // Backend not running yet or token template doesn't exist - that's expected
    //         console.log('Backend connection not available yet')
    //     }
    // }

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
