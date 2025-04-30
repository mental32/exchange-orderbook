'use client'

import { useUser } from "@clerk/nextjs";
import { Label } from "./ui/label";

export default function GoodMorning() {
    const { user } = useUser();
    return (
        <header>
            <span className="text-transparent text-3xl font-bold bg-linear-to-r from-cyan-500 to-blue-500 bg-clip-text w-fit">
                Good Morning, {user?.firstName}
            </span>
        </header>
    );
}