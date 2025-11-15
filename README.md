exchange-orderbook is an implementation of a full-stack, [Auction-market](https://www.investopedia.com/terms/a/auctionmarket.asp), spot-exchange; like Coinbase, Kraken, or Binance supporting Bitcoin (BTC), Ether (ETH), Solana (SOL), and USDC.

The frontend was built using [NextJS](https://nextjs.org/), [TailwindCSS](https://tailwindcss.com), [shadcn/ui](https://ui.shadcn.com/), and [Clerk](https://clerk.com/). The database is [Postgres](https://www.postgresql.org/), for the [matching engine](https://www.investopedia.com/articles/active-trading/042414/youd-better-know-your-highfrequency-trading-terminology.asp#toc-matching-engine), [settlement layer](https://groww.in/p/what-is-trade-settlement), and blockchain integration [Rust](https://www.rust-lang.org/) was used.  Check out the [screenshots](#screenshots)!

The reason you should pay attention to this is because its not demo code for some very specific sub-system, or a pretty looking UI; it is a complete end-to-end mix of: **code**, **UI**, **documentation**, **automated tests**... (ok whats the big deal?) the deal is I did not stop there I provide cross-cutting guides and decision records on operational, security, QE concerns for deploying, testing, etc... If you've never shipped software before this should look strange you might have gotten used to leetcode-like problems or gimmicky CLI tools.

_**How do I run this?**_ - Docker! `docker compose up` (See the `docker-compose.yml` file for more details.)

_**Why make this?**_ - Fun!

_**Do I have permission to run this?**_ - Absolutely! The code here is all MIT licensed. Do whatever the hell you want with it.

_**Where can I read about the design or architecture?**_ - There is an [ARCHITECTURE.md](./ARCHITECTURE.md) file in the root directory.

Use GitHub discussions for questions, otherwise e-mail `mentalfoss+exob@gmail.com`

## Screenshots

![landing-page](./screenshots/landing-page.jpeg)

![Home](./screenshots/home-page.jpeg)
