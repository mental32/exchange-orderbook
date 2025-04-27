import { AppSidebar } from "@/components/app-sidebar";
import { LineChartComp } from "@/components/line-chart";
import { SidebarInset, SidebarProvider } from "@/components/ui/sidebar";

export default function Home() {
    return (
        <div className="m-auto h-full w-full max-h-screen bg-background">
            <SidebarProvider className="flex flex-col">
                <header className="flex sticky top-0 z-50 w-full items-center border-b bg-background h-[6rem]"></header>
                <div className="flex flex-1 max-h-[calc(100vh - 6rem)]">
                    <AppSidebar className={"static pl-4 pt-6 px-1 border-none shadow-none bg-background top-[6rem] !h-[calc(100svh-6rem)]"}></AppSidebar>
                    <SidebarInset>
                        <main>
                            <LineChartComp></LineChartComp>
                        </main>
                    </SidebarInset>

                </div>
            </SidebarProvider>
        </div >
    );
}