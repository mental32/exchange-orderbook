import { Card } from "./ui/card";


export function Watchlist() {
    return <section className="flex flex-col gap-[1rem]">
        <div className="text-xl font-bold">Watchlist</div>
        <div className="grid auto-cols-max grid-flow-col gap-[1rem] overflow-hidden">
            <Card className="h-[12rem] w-[12rem] p-0 border-0">
            </Card>
            <Card className="h-[12rem] w-[12rem] p-0 border-0">
            </Card>
            <Card className="h-[12rem] w-[12rem] p-0 border-0">
            </Card>
            <Card className="h-[12rem] w-[12rem] p-0 border-0">
            </Card>
        </div>
    </section>
}