"use client"

import {
    Sidebar,
    SidebarContent,
    SidebarFooter,
    SidebarMenu,
    SidebarMenuButton,
    SidebarMenuItem,
} from "@/components/ui/sidebar"
import { UserButton } from "@/components/user-button";
import {
    SignInButton,
    SignUpButton,
    SignedIn,
    SignedOut,
} from '@clerk/nextjs'
import { ArrowLeftRight, Clock, Home, Search, Wallet } from "lucide-react";
import Link from "next/link";
import { usePathname } from "next/navigation";
import path from "path";
import { useState, useEffect, act } from "react";

const items = [
    {
        title: "Home",
        url: '/c',
        icon: Home,
    },
    {
        title: "Portfolio",
        url: '/c/portfolio',
        icon: Wallet,
    },
    {
        title: "Explore",
        url: '/c/explore',
        icon: Search,
    },
    {
        title: "Transfer",
        url: '/c/transfer/',
        icon: ArrowLeftRight,
    },
    {
        title: "Transactions",
        url: '/c/transactions',
        icon: Clock,
    },
];

type SidebarMode = 'full' | 'shrunk' | 'hidden';

export function AppSidebar({ className }: React.ComponentPropsWithoutRef<'div'>) {
    const [sidebarMode, setSidebarMode] = useState<SidebarMode>('full');
    const pathname = usePathname();

    useEffect(() => {
        const handleResize = () => {
            const screenWidth = window.innerWidth; // Use innerWidth for viewport size
            const oneThirdScreenWidth = window.screen.width / 3; // Use screen.width for reference
            const twoThirdsScreenWidth = 2 * window.screen.width / 3;

            if (screenWidth < oneThirdScreenWidth) {
                setSidebarMode('hidden');
            } else if (screenWidth < twoThirdsScreenWidth) {
                setSidebarMode('shrunk');
            } else {
                setSidebarMode('full');
            }
        };

        // Set initial state
        handleResize();

        window.addEventListener('resize', handleResize);

        // Cleanup listener on component unmount
        return () => window.removeEventListener('resize', handleResize);
    }, []);

    if (sidebarMode === 'hidden') {
        return null; // Don't render the sidebar at all
    }

    const isShrunk = sidebarMode === 'shrunk';

    return (
        <Sidebar
            className={`${isShrunk ? "w-[80px]" : "w-(--sidebar-width)"
                } ${className}`}
        >
            {/* < SidebarHeader className={`flex items-center gap-3 p-2 rounded-md gapshadow-none bg-background`}>
            </SidebarHeader> */}
            <SidebarContent className={`shadow-none bg-background`}>
                <SidebarMenu>
                    {items.map((item) => (
                        <SidebarMenuItem
                            className="pl-3 pr-3 pt-2 pb-2 ml-4 gap-0"
                            key={item.title}
                        >
                            <SidebarMenuButton
                                className="p-0 hover:bg-background hover:text-foreground text-muted-foreground focus-visible:ring-0 focus-visible:outline-none data-[active=true]:bg-background"
                                isActive={pathname === item.url}
                                asChild
                            >
                                <Link
                                    prefetch={false}
                                    href={item.url}
                                    className={`[&>svg]:size-7 > svg flex items-center gap-3 rounded-md overflow-visible ${isShrunk
                                        ? "flex-col justify-center h-16 text-xs"
                                        : "flex-row"
                                        } `}
                                    title={isShrunk ? item.title : undefined} // Show title on hover when collapsed
                                >
                                    <item.icon className={`${isShrunk ? "mb-1" : ""}`} />
                                    <span
                                        className={`${isShrunk ? "text-center" : ""
                                            } overflow-visible`}
                                    >
                                        {item.title}
                                    </span>
                                </Link>
                            </SidebarMenuButton>
                        </SidebarMenuItem>
                    ))}
                </SidebarMenu>
            </SidebarContent>
            <SidebarFooter
                className={`${isShrunk ? "flex flex-col items-center space-y-2" : ""
                    } shadow-none bg-background`}
            >
                <SignedOut>
                    {/* Consider how SignInButton/SignUpButton render when collapsed */}
                    <SignInButton />
                    <SignUpButton />
                </SignedOut>
                <SignedIn>
                    <UserButton />
                </SignedIn>
            </SidebarFooter>
        </Sidebar>
    );
}
