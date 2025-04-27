"use client"

import { useClerk, useUser } from "@clerk/nextjs"
import { DropdownMenu } from "@radix-ui/react-dropdown-menu";
import { DropdownMenuContent, DropdownMenuGroup, DropdownMenuItem, DropdownMenuLabel, DropdownMenuPortal, DropdownMenuSeparator, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { Button } from "@/components/ui/button";
import { useRouter } from "next/navigation";

export function UserButton() {
    const { isLoaded, user } = useUser();
    const { signOut, openUserProfile } = useClerk();
    const router = useRouter();

    // Make sure that the useUser() hook has loaded
    if (!isLoaded) return null
    // Make sure there is valid user data
    if (!user?.id) return null

    return (
        <DropdownMenu>
            <DropdownMenuTrigger asChild>
                <Avatar>
                    <AvatarImage src={user?.imageUrl}></AvatarImage>
                    <AvatarFallback>CN</AvatarFallback>
                </Avatar>
            </DropdownMenuTrigger>
            <DropdownMenuContent>
                <DropdownMenuItem className="font-bold">
                    {user?.fullName}
                </DropdownMenuItem>
                <DropdownMenuItem className="text-muted-foreground">
                    {user?.emailAddresses[0].emailAddress}
                </DropdownMenuItem>
                <DropdownMenuLabel>
                    Label
                </DropdownMenuLabel>
                <DropdownMenuSeparator></DropdownMenuSeparator>
                <DropdownMenuItem asChild>
                    <Button
                        onClick={() => { signOut(() => { router.push("/") }) }}
                    >
                        Sign Out
                    </Button>
                </DropdownMenuItem>
            </DropdownMenuContent>
        </DropdownMenu>
    )

}
