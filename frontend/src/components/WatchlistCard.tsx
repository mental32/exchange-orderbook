'use client'

import { Card } from './ui/card'
import { MoveUpRight, MoveDownRight } from 'lucide-react'
import Image from 'next/image'
import ClickableStar from './clickable-star'
import Link from 'next/link'

interface WatchlistCardProps {
  title: string
  short: string
  price: string
  change24h: string
  change24hDirection: 'up' | 'down'
  image: {
    src: string
    alt: string
  }
}

export function WatchlistCard({
  title,
  short,
  price,
  change24h,
  change24hDirection,
  image
}: WatchlistCardProps) {
  const ChangeIcon = change24hDirection === 'up' ? MoveUpRight : MoveDownRight
  const changeColor = change24hDirection === 'up' ? '#139E62' : '#dc2626'

  return (
    <Link href={`/c/assets/${short.toLowerCase()}`} className="block">
      <Card className="h-[12rem] w-[12rem] p-4 border cursor-pointer hover:shadow-lg transition-shadow relative">
        <div className="absolute top-2 right-2 z-10">
          <ClickableStar symbol={short} size={14} />
        </div>

        <div className="flex flex-col h-full justify-between">
          <div className="flex items-center gap-2 mb-3">
            <Image
              src={image.src}
              alt={image.alt}
              height={24}
              width={24}
            />
            <div>
              <div className="font-semibold text-sm">{short}</div>
              <div className="text-xs text-muted-foreground truncate max-w-[5rem]">
                {title}
              </div>
            </div>
          </div>

          <div className="space-y-2">
            <div className="text-lg font-bold">{price}</div>
            <div className="flex items-center gap-1 text-xs">
              <ChangeIcon
                color={changeColor}
                strokeWidth={3}
                size={12}
              />
              <span style={{ color: changeColor }}>
                {change24h}
              </span>
              <span className="text-muted-foreground">24h</span>
            </div>
          </div>
        </div>
      </Card>
    </Link>
  )
}