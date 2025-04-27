import Link from "next/link";

export default function SiteFooter() {
    return (
        <footer className="border-t bg-muted">
            <div className="px-4 py-8 md:px-6 container mx-auto">
                <div className="grid grid-cols-1 gap-8 md:grid-cols-4">
                    <div className="space-y-2">
                        <div className="flex items-center space-x-2">
                            {/* <Hashtag className="h-6 w-6" /> */}
                            <span className="text-lg font-bold">Crypto Exchange Ltd.</span>
                        </div>
                        <p className="text-sm text-muted-foreground">Diamond hands tbh</p>
                    </div>
                    <div className="space-y-3">
                        <h3 className="text-sm font-medium uppercase tracking-wider">Company</h3>
                        <ul className="space-y-2">
                            <li>
                                <Link href="#" className="text-sm text-muted-foreground hover:text-foreground">
                                    About us
                                </Link>
                            </li>
                            <li>
                                <Link href="#" className="text-sm text-muted-foreground hover:text-foreground">
                                    Contact
                                </Link>
                            </li>
                            <li>
                                <Link href="#" className="text-sm text-muted-foreground hover:text-foreground">
                                    Jobs
                                </Link>
                            </li>
                            <li>
                                <Link href="#" className="text-sm text-muted-foreground hover:text-foreground">
                                    Press kit
                                </Link>
                            </li>
                        </ul>
                    </div>
                    <div className="space-y-3">
                        <h3 className="text-sm font-medium uppercase tracking-wider">Legal</h3>
                        <ul className="space-y-2">
                            <li>
                                <Link href="#" className="text-sm text-muted-foreground hover:text-foreground">
                                    Terms of use
                                </Link>
                            </li>
                            <li>
                                <Link href="#" className="text-sm text-muted-foreground hover:text-foreground">
                                    Privacy policy
                                </Link>
                            </li>
                            <li>
                                <Link href="#" className="text-sm text-muted-foreground hover:text-foreground">
                                    Cookie policy
                                </Link>
                            </li>
                        </ul>
                    </div>
                </div>
            </div>
        </footer>
    )
}