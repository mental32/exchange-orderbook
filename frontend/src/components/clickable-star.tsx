"use client"

import { useState, useRef } from "react"
import { Star } from "lucide-react"
import { cn } from "@/lib/utils"
import { Button } from "@/components/ui/button"
import confetti from "canvas-confetti"

interface ClickableStarProps {
    initialState?: boolean
    size?: number
    onChange?: (isStarred: boolean) => void
    className?: string
}

export default function ClickableStar({ initialState = false, size = 24, onChange, className }: ClickableStarProps) {
    const [isStarred, setIsStarred] = useState(initialState)
    const buttonRef = useRef<HTMLButtonElement>(null)

    const triggerConfetti = () => {
        if (!buttonRef.current) return

        // Get the position of the star
        const rect = buttonRef.current.getBoundingClientRect()
        const x = (rect.left + rect.width / 2) / window.innerWidth
        const y = (rect.top + rect.height / 2) / window.innerHeight

        // Create a more localized confetti effect
        confetti({
            particleCount: 30,
            spread: 40,
            origin: { x, y },
            colors: ["#FFD700", "#FFC107", "#FFEB3B", "#FFD600"],
            disableForReducedMotion: true,
            zIndex: 1000,
            startVelocity: 15,
            scalar: 0.7,
            gravity: 1.2,
            ticks: 50,
        })
    }

    const handleClick = () => {
        const newState = !isStarred
        setIsStarred(newState)
        onChange?.(newState)

        // Only trigger confetti when starring (not when unstarring)
        if (newState) {
            triggerConfetti()
        }
    }

    return (
        <Button
            ref={buttonRef}
            variant="ghost"
            size="icon"
            onClick={handleClick}
            className={cn("p-0 h-auto w-auto hover:bg-transparent", className)}
            aria-label={isStarred ? "Unstar" : "Star"}
        >
            <Star
                size={size}
                className={cn(
                    "!w-auto !h-auto transition-colors duration-200",
                    isStarred ? "fill-yellow-400 text-yellow-400" : "fill-none text-muted-foreground hover:text-foreground",
                )}
            />
        </Button>
    )
}
