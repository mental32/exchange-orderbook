# _Greetings!_ 👋

exchange-orderbook is an implementation of a full-stack, [Auction-market](https://www.investopedia.com/terms/a/auctionmarket.asp), spot-exchange; like Coinbase, Kraken, or Binance
supporting Bitcoin (BTC), Ether (ETH), Solana (SOL), and USDC.

The frontend was built using [NextJS](https://nextjs.org/), [TailwindCSS](https://tailwindcss.com), [shadcn/ui](https://ui.shadcn.com/), and [Clerk](https://clerk.com/). The database is [Postgres](https://www.postgresql.org/), for the [matching engine](https://www.investopedia.com/articles/active-trading/042414/youd-better-know-your-highfrequency-trading-terminology.asp#toc-matching-engine), [settlement layer](https://groww.in/p/what-is-trade-settlement), and blockchain integration [Rust](https://www.rust-lang.org/) was used.  Check out the [screenshots](#screenshots)!

_**Do I have permission to run this?**_ - Yes, you have permission to run the software on your personal device for educational purposes only. This means you can use the software to learn and understand the coding practices and techniques employed.
However, you are not allowed to use the software for any commercial activities, nor can you modify or distribute the software in any form, whether modified or original.

_**Why make this?**_ - Fun mostly. I don't get to do a lot of full stack work at day job so this is a great way to stretch my virtual legs every once in a while.

_**How do I run this?**_ - A `docker-compose.yml` file is provided so running `docker compose up` should be enough; if it is not then please open an issue and let me know.

Email `mentalfoss+exob@gmail.com` for job offers otherwise use GitHub discussions for questions.

## Screenshots

### Landing Page

![landing-page](./screenshots/landing-page.jpeg)

### Sign-In Page

![Sign-In Page](./screenshots/sign-in-page.jpeg)
