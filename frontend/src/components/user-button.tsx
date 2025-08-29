"use client"

import { useClerk, useUser } from "@clerk/nextjs"
import { DropdownMenu } from "@radix-ui/react-dropdown-menu";
import { DropdownMenuContent, DropdownMenuGroup, DropdownMenuItem, DropdownMenuLabel, DropdownMenuPortal, DropdownMenuSeparator, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { Button } from "@/components/ui/button";
import { useRouter } from "next/navigation";
import Link from "next/link";
import { useTheme } from "next-themes";
import {
    User,
    Shield,
    Bell,
    Settings,
    CreditCard,
    FileText,
    LogOut,
    Copy,
    Sun,
    Moon,
    Monitor,
    Check
} from "lucide-react";
import { useState, useEffect } from "react";
import { cn } from "@/lib/utils";

type SidebarMode = 'full' | 'shrunk' | 'hidden';

interface UserButtonProps {
    mode?: SidebarMode;
}

export function UserButton({ mode = 'full' }: UserButtonProps) {
    const { isLoaded, user } = useUser();
    const { signOut, openUserProfile } = useClerk();
    const router = useRouter();
    const { theme, setTheme, resolvedTheme } = useTheme();
    const [copied, setCopied] = useState(false);
    const [mounted, setMounted] = useState(false);

    useEffect(() => {
        setMounted(true);
    }, []);

    // Make sure that the useUser() hook has loaded
    if (!isLoaded) return null
    // Make sure there is valid user data
    if (!user) return null
    if (!user?.id) return null

    const isShrunk = mode === 'shrunk';
    const username = user?.username || `${user?.firstName || ''}${user?.lastName || ''}` || 'User';
    const initials = (user?.firstName?.[0] || '') + (user?.lastName?.[0] || '') ||
                       (user?.fullName?.[0] || '') + (user?.fullName?.[1] || '') || 'U';

    const copyUsername = async () => {
        try {
            await navigator.clipboard.writeText(`@${username}`);
            setCopied(true);
            setTimeout(() => setCopied(false), 2000);
        } catch (err) {
            console.error('Failed to copy username:', err);
        }
    };

    const menuItems = [
        { icon: User, label: 'Account', href: '/c/account-settings/account' },
        { icon: Shield, label: 'Security', href: '/c/account-settings/security' },
        { icon: Bell, label: 'Notifications', href: '/c/account-settings/notifications' },
        { icon: CreditCard, label: 'Payment methods', href: '/c/account-settings/payment-methods' },
        { icon: FileText, label: 'Documents', href: '/c/account-settings/documents' },
        { icon: Settings, label: 'Device management', href: '/c/account-settings/device-management' },
    ];

    return (
        <div className="pl-3 pr-3 pt-2 pb-2 ml-4">
            <DropdownMenu>
                <DropdownMenuTrigger asChild>
                    <Button
                        variant="ghost"
                        className={`flex items-center gap-3 rounded-md p-0 hover:bg-background text-muted-foreground hover:text-foreground focus-visible:ring-0 focus-visible:outline-none w-full h-auto ${isShrunk ? "flex-col justify-center h-16 text-xs" : "flex-row"
                            }`}
                    >
                        <Avatar className="size-7">
                            <AvatarImage src={user?.imageUrl}></AvatarImage>
                            <AvatarFallback>
                                {initials}
                            </AvatarFallback>
                        </Avatar>
                        {!isShrunk && (
                            <span className="overflow-hidden text-ellipsis whitespace-nowrap">
                                {user?.firstName || user?.fullName}
                            </span>
                        )}
                    </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent className="w-[20.5rem] p-3" align="start" sideOffset={8}>
                    {/* User Profile Header */}
                    <div className="flex gap-4 mb-5">
                        <Avatar className="w-14 h-14">
                            <AvatarImage src={user?.imageUrl} alt={initials} />
                            <AvatarFallback className="text-lg font-semibold">
                                {initials}
                            </AvatarFallback>
                        </Avatar>
                        <div className="flex-1 flex flex-col justify-center max-w-[210px] gap-y-1">
                            <span className="text-base font-medium block truncate">
                                {user?.fullName || user?.firstName}
                            </span>
                            <div>
                                <Button
                                    variant="ghost"
                                    size="sm"
                                    className="h-auto p-1 text-xs text-muted-foreground hover:text-foreground"
                                    onClick={copyUsername}
                                >
                                    <span>@{username}</span>
                                    {copied ? (
                                        <Check className="w-3 h-3 ml-1 text-green-500" />
                                    ) : (
                                        <Copy className="w-3 h-3 ml-1" />
                                    )}
                                </Button>
                            </div>
                        </div>
                    </div>

                    {/* Theme Switcher */}
                    {mounted && (
                        <div className="flex rounded-lg bg-muted p-1 mb-4" role="tablist">
                            {[
                                { key: 'system', icon: Monitor, label: 'Auto' },
                                { key: 'light', icon: Sun, label: 'Light' },
                                { key: 'dark', icon: Moon, label: 'Dark' }
                            ].map(({ key, icon: Icon, label }) => {
                                const isActive = theme === key;
                                return (
                                    <Button
                                        key={key}
                                        variant="ghost"
                                        size="sm"
                                        role="tab"
                                        aria-selected={isActive}
                                        className={cn(
                                            "flex-1 flex items-center justify-center gap-2 rounded-md px-3 py-1.5 text-sm font-medium transition-colors h-auto",
                                            isActive
                                                ? "bg-background text-foreground shadow-sm"
                                                : "text-muted-foreground hover:text-foreground"
                                        )}
                                        onClick={() => setTheme(key)}
                                    >
                                        <Icon className="w-4 h-4" />
                                        {label}
                                    </Button>
                                );
                            })}
                        </div>
                    )}

                    {/* Menu Items */}
                    <div className="space-y-1">
                        {menuItems.map(({ icon: Icon, label, href }) => (
                            <DropdownMenuItem key={label} asChild>
                                <Link
                                    href={href}
                                    className="flex items-center gap-3 w-full px-2 py-2 text-sm rounded-lg hover:bg-accent cursor-pointer"
                                >
                                    <Icon className="w-4 h-4" />
                                    {label}
                                </Link>
                            </DropdownMenuItem>
                        ))}
                    </div>

                    <DropdownMenuSeparator className="my-2" />

                    {/* Sign Out */}
                    <DropdownMenuItem asChild>
                        <Button
                            variant="ghost"
                            onClick={() => { signOut(() => { router.push("/") }) }}
                            className="flex items-center gap-3 w-full px-2 py-2 text-sm rounded-lg hover:bg-accent cursor-pointer text-foreground justify-start h-auto"
                        >
                            <LogOut className="w-4 h-4" />
                            Sign out
                        </Button>
                    </DropdownMenuItem>
                </DropdownMenuContent>
            </DropdownMenu>
        </div>
    )

}
