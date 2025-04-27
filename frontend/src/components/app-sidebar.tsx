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
import { useState, useEffect } from "react";

const items = [
    {
        title: "Home",
        url: '#',
        icon: Home,
    },
    {
        title: "Portfolio",
        url: '#',
        icon: Wallet,

    },
    {
        title: "Explore",
        url: '#',
        icon: Search,
    },
    {
        title: "Transfer",
        url: '#',
        icon: ArrowLeftRight,
    },
    {
        title: "Transactions",
        url: '#',
        icon: Clock,
    },
];

type SidebarMode = 'full' | 'collapsed' | 'hidden';

export function AppSidebar({ className }: React.ComponentPropsWithoutRef<'div'>) {
    const [sidebarMode, setSidebarMode] = useState<SidebarMode>('full');

    useEffect(() => {
        const handleResize = () => {
            const screenWidth = window.innerWidth; // Use innerWidth for viewport size
            const oneThirdScreenWidth = window.screen.width / 3; // Use screen.width for reference
            const twoThirdsScreenWidth = 2 * window.screen.width / 3;

            if (screenWidth < oneThirdScreenWidth) {
                setSidebarMode('hidden');
            } else if (screenWidth < twoThirdsScreenWidth) {
                setSidebarMode('collapsed');
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

    const isCollapsed = sidebarMode === 'collapsed';

    return (
        <Sidebar className={`${isCollapsed ? "w-20" : "w-64"} ${className}`} >
            {/* < SidebarHeader className={`flex items-center gap-3 p-2 rounded-md gapshadow-none bg-background`}>
            </SidebarHeader> */}
            <SidebarContent className={`shadow-none bg-background`}>
                <SidebarMenu>
                    {items.map((item) => (
                        <SidebarMenuItem className="pl-3 pr-3 pt-2 pb-2 ml-4 gap-0" key={item.title}>
                            <SidebarMenuButton className='[&>svg]:size-7 p-0 hover:bg-background hover:text-foreground text-muted-foreground focus-visible:ring-0 focus-visible:outline-none active:bg-background' asChild>
                                <a
                                    href={item.url}
                                    className={`flex items-center gap-3 rounded-md ${isCollapsed ? 'flex-col justify-center h-16 text-xs' : 'flex-row'} `}
                                    title={isCollapsed ? item.title : undefined} // Show title on hover when collapsed
                                >
                                    <item.icon className={`${isCollapsed ? "mb-1" : ""}`} />
                                    <span className={isCollapsed ? "text-center" : ""}>{isCollapsed ? item.title : item.title}</span>
                                </a>
                            </SidebarMenuButton>
                        </SidebarMenuItem>
                    ))}
                </SidebarMenu>
            </SidebarContent>
            <SidebarFooter className={`${isCollapsed ? "flex flex-col items-center space-y-2" : ""} shadow-none bg-background`}>
                <SignedOut>
                    {/* Consider how SignInButton/SignUpButton render when collapsed */}
                    <SignInButton />
                    <SignUpButton />
                </SignedOut>
                <SignedIn>
                    <UserButton />
                </SignedIn>
            </SidebarFooter>
        </Sidebar >
    )
}
