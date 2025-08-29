'use client'

import { useEffect, useRef, useState } from 'react'
import { Search } from 'lucide-react'
import Image from 'next/image'
import { createPortal } from 'react-dom'
import { useRouter } from 'next/navigation'

// Asset data - using same data as mini-explore component
const assets = [
    {
        title: "Bitcoin",
        short: "BTC",
        price: "£72,846.59",
        image: {
            src: "/bitcoin-btc-logo.svg",
            alt: "Bitcoin logo"
        }
    },
    {
        title: "Ethereum",
        short: "ETH",
        price: "£1,381.07",
        image: {
            src: "/eth-glyph-colored.svg",
            alt: "Ether diamond colored logo"
        }
    },
    {
        title: "Tether USD",
        short: "USDT",
        price: "£0.7534",
        image: {
            src: "/tether-usdt-logo.svg",
            alt: "Tether USD"
        }
    },
    {
        title: "XRP",
        short: "XRP",
        price: "£1.67",
        image: {
            src: "/xrp-xrp-logo.svg",
            alt: "XRP logo"
        }
    },
]

export function SearchBar() {
    const router = useRouter()
    const [isActive, setIsActive] = useState(false)
    const [searchQuery, setSearchQuery] = useState('')
    const [selectedIndex, setSelectedIndex] = useState(0)
    const [popoverPosition, setPopoverPosition] = useState({ x: 0, y: 0, width: 0 })
    const inputRef = useRef<HTMLInputElement>(null)
    const containerRef = useRef<HTMLDivElement>(null)
    const searchBarRef = useRef<HTMLDivElement>(null)

    // Filter assets based on search query
    const filteredAssets = assets.filter(asset =>
        asset.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
        asset.short.toLowerCase().includes(searchQuery.toLowerCase())
    )

    // Reset selected index when search changes
    useEffect(() => {
        setSelectedIndex(0)
    }, [searchQuery])

    useEffect(() => {
        const handleKeyDown = (event: KeyboardEvent) => {
            if ((event.metaKey || event.ctrlKey) && event.key === 'k') {
                event.preventDefault()
                setIsActive(true)
                updatePopoverPosition()
                setTimeout(() => inputRef.current?.focus(), 100)
            }
            
            if (isActive) {
                if (event.key === 'Escape') {
                    setIsActive(false)
                    setSearchQuery('')
                    inputRef.current?.blur()
                } else if (event.key === 'ArrowDown') {
                    event.preventDefault()
                    setSelectedIndex((prev) => 
                        prev < filteredAssets.length - 1 ? prev + 1 : prev
                    )
                } else if (event.key === 'ArrowUp') {
                    event.preventDefault()
                    setSelectedIndex((prev) => prev > 0 ? prev - 1 : prev)
                } else if (event.key === 'Enter') {
                    event.preventDefault()
                    if (filteredAssets[selectedIndex]) {
                        handleAssetSelect(filteredAssets[selectedIndex])
                    }
                }
            }
        }

        document.addEventListener('keydown', handleKeyDown)
        return () => document.removeEventListener('keydown', handleKeyDown)
    }, [isActive, filteredAssets, selectedIndex])

    // Handle clicking outside to close dropdown
    useEffect(() => {
        const handleClickOutside = (event: MouseEvent) => {
            if (containerRef.current && !containerRef.current.contains(event.target as Node)) {
                setIsActive(false)
                setSearchQuery('')
            }
        }

        if (isActive) {
            document.addEventListener('mousedown', handleClickOutside)
        }

        return () => {
            document.removeEventListener('mousedown', handleClickOutside)
        }
    }, [isActive])

    const updatePopoverPosition = () => {
        if (searchBarRef.current) {
            const rect = searchBarRef.current.getBoundingClientRect()
            setPopoverPosition({
                x: rect.left,
                y: rect.bottom,
                width: rect.width
            })
        }
    }

    const handleClick = () => {
        setIsActive(true)
        updatePopoverPosition()
        setTimeout(() => inputRef.current?.focus(), 100)
    }

    const handleAssetSelect = (asset: typeof assets[0]) => {
        router.push(`/c/assets/${asset.short.toLowerCase()}`)
        setIsActive(false)
        setSearchQuery('')
        inputRef.current?.blur()
    }

    return (
        <>
            <div ref={containerRef} className="w-full max-w-lg mx-auto px-4 pt-3 pb-3 relative">
                {/* Inactive State - White Search Bar Trigger */}
                {!isActive && (
                    <div 
                        ref={searchBarRef}
                        onClick={handleClick}
                        className="bg-white hover:bg-gray-50 outline-offset-2 outline-2 focus-within:outline focus-within:outline-blue-500 box-border flex items-stretch gap-3 p-1 pr-3 h-8 rounded-lg pl-3 transition-colors cursor-pointer"
                    >
                        <div className="flex items-center">
                            <Search className="text-gray-400 h-4 w-4" />
                        </div>
                        <div className="flex items-center flex-1">
                            <span className="text-gray-500 text-sm">Search for assets, markets & more</span>
                        </div>
                        <div className="flex items-center">
                            <div className="bg-gray-200 text-gray-600 inline-flex items-center justify-center text-xs rounded px-2 py-1 font-medium">
                                ⌘ + K
                            </div>
                        </div>
                    </div>
                )}
            </div>

            {/* Overlay - Portal to body */}
            {isActive && createPortal(
                <div 
                    className="fixed inset-0 z-40"
                    style={{ backgroundColor: 'rgba(0, 0, 0, 0.6)' }}
                    onClick={() => setIsActive(false)}
                />,
                document.body
            )}

            {/* Unified Command Center Container - Portal to body */}
            {isActive && createPortal(
                <div 
                    className="fixed z-50 rounded-xl shadow-xl overflow-hidden"
                    style={{ 
                        backgroundColor: '#181522',
                        border: 'none',
                        left: `${popoverPosition.x}px`,
                        top: `${popoverPosition.y + 4}px`,
                        width: `${popoverPosition.width}px`,
                        padding: '14px'
                    }}
                >
                    {/* Search Input Section - Inside Unified Container */}
                    <div 
                        className="flex items-stretch gap-3 h-10 rounded-xl transition-colors mb-3"
                        style={{ 
                            backgroundColor: 'rgba(110, 104, 130, 0.3)',
                            padding: '10px 14px'
                        }}
                    >
                        <div className="flex items-center">
                            <Search className="h-4 w-4" style={{ color: '#9894a9' }} />
                        </div>
                        <div className="flex items-center flex-1">
                            <input
                                ref={inputRef}
                                type="text"
                                placeholder="Search for assets, markets & more"
                                value={searchQuery}
                                onChange={(e) => setSearchQuery(e.target.value)}
                                className="w-full h-full border-none bg-inherit outline-none text-sm px-2"
                                style={{ 
                                    color: '#f8f7fa',
                                    caretColor: '#f8f7fa',
                                    backgroundColor: 'transparent'
                                }}
                            />
                        </div>
                        <div className="flex items-center">
                            <div 
                                className="inline-flex items-center justify-center text-xs rounded px-2 py-1 font-medium"
                                style={{ 
                                    backgroundColor: '#6e6882',
                                    color: '#fff'
                                }}
                            >
                                ⌘ + K
                            </div>
                        </div>
                    </div>

                    {/* Results Section - In Same Unified Container */}
                    {filteredAssets.length > 0 && (
                        <div className="max-h-96 overflow-y-auto -mx-3">
                            {filteredAssets.map((asset, index) => (
                                <div
                                    key={asset.short}
                                    className="flex justify-between gap-3 overflow-hidden w-full px-3 py-2 cursor-pointer transition-colors"
                                    style={{
                                        backgroundColor: index === selectedIndex ? 'rgba(134, 97, 255, 0.08)' : 'transparent'
                                    }}
                                    onClick={() => handleAssetSelect(asset)}
                                    onMouseEnter={() => setSelectedIndex(index)}
                                >
                                    <div className="min-w-0 flex-1">
                                        <div className="flex items-center gap-2">
                                            <div className="relative h-5 w-5 flex-shrink-0">
                                                <Image 
                                                    src={asset.image.src} 
                                                    alt={asset.image.alt} 
                                                    height={20} 
                                                    width={20}
                                                    className="rounded-full"
                                                />
                                            </div>
                                            <div className="flex flex-col min-w-0">
                                                <span className="text-sm font-medium truncate" style={{ color: '#f8f7fa' }}>
                                                    {asset.title}
                                                </span>
                                                <span className="text-xs" style={{ color: '#9894a9' }}>{asset.short}</span>
                                            </div>
                                        </div>
                                    </div>
                                    <div className="flex items-center">
                                        <span className="text-sm font-semibold" style={{ color: '#f8f7fa' }}>
                                            {asset.price}
                                        </span>
                                    </div>
                                </div>
                            ))}
                        </div>
                    )}

                    {/* No results message - In Same Unified Container */}
                    {searchQuery && filteredAssets.length === 0 && (
                        <div className="py-4">
                            <p className="text-center" style={{ color: '#9894a9' }}>No assets found for "{searchQuery}"</p>
                        </div>
                    )}
                </div>,
                document.body
            )}
        </>
    )
}