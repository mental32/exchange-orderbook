<script>
	import { goto } from '$app/navigation';
	import Navbar from '../Navbar.svelte';
	import '../app.css';
</script>

<div id="app" class="bg-white">
	<div class="relative z-[1000]">
		<div
			class=" bg-[#CFCCFF] flex justify-center px-[20px] py-[8px] sm:leading-[20px] sm:text-[length:var(--body-font-size)] leading-[16px] text-[length:var(--body-font-size-sm)] [&amp;_strong]:font-bold [&amp;_a]:underline hover:[&amp;_a]:underline text-[#000000] [&amp;_a]:text-[#7132F5] border-b-[#CDCDCD] border-b border-solid font-['GT_Standard_S'] fixed top-0 left-0 right-0"
			style="--body-font-size:15px;--body-font-size-sm:14"
		>
			<div>
				<span>
					<strong
						>Don't invest unless you're prepared to lose all the money you invest. This is a
						high-risk investment and you should not expect to be protected if something goes wrong. <a
							href="/"
							class="underline"
							style="color: rgb(113 50 245/var(--tw-text-opacity))">Take 2 minutes to learn more</a
						>.</strong
					></span
				>
			</div>
		</div>
		<div style="height: 57px;"></div>
	</div>
	<div class="px-0">
		<Navbar />
	</div>
	<div class="relative bg-[#EEECFB] h-[744px]" style="padding: 50px 0px;">
		<div class="w-full flex box-border px-[30px] min-[1040px]:px-[calc((100%-980px)/2)]">
			<div class="justify-center flex-col flex flex-auto text-center" style="padding-right: 160px">
				<div class="w-full h-0 bg-[#0E0C2]"></div>
				<div class="bg-[#0E0C2]">
					<h1 class="text-left text-black text-[32px]"><span>Buy Bitcoin & Crypto 🇬🇧</span></h1>
				</div>
				<div class="w-full h-[32px]"></div>
				<div class="text-left font-normal">
					<p>
						Grow your portfolio in a more fair and open financial system. <strong
							>Start trading crypto with just £10</strong
						>
					</p>
				</div>
				<div class="w-full h-[32px]"></div>
				<div class="w-full text-left">
					<button
						class="sign-up bg-[#462FC2] text-white py-2 px-4 border border-solid rounded-full hover:bg-opacity-75 focus:outline-none transition duration-150 absolute box-border shadow-2xl"
						on:click={() => goto('/sign-up')}>Sign up</button
					>
				</div>
			</div>
		</div>
	</div>

	<div class="text-[#0E0C28]">
		<div class=" w-full box-border px-[30px] min-[1234px]:px-[calc((100%-1174px)/2)]">
			<div class="font-medium">
				<div class=""><div class="w-full h-[84px]"></div></div>
				<div class="">
					<h2 class="text-center w-full h-[84px] text-[36px]">
						<span>A crypto exchange for everyone</span>
					</h2>
				</div>
				<div class=""><div class="w-full h-[24px]"></div></div>
				<div class="text-center">
					<div class="grid gap-0" style="grid-template-columns: 1fr 1fr 1fr;">
						<div class="items-center flex flex-col flex-wrap">
							<h3 class="text-[36px]">
								<span>10M+</span>
							</h3>
							<div class="text-[16px]">
								<p>Clients</p>
							</div>
						</div>
						<div class="">
							<h3 class="text-[36px]">190+</h3>
							<div class="text-[16px]">
								<p>Countries supported</p>
							</div>
						</div>
						<div class="">
							<h3 class="text-[36px]">$207B+</h3>
							<div class="text-[16px]">
								<p>Quarterly trading volume</p>
							</div>
						</div>
					</div>
				</div>
				<div class=""><div class="w-full h-[84px]"></div></div>
			</div>
		</div>
	</div>

	<footer class="text-white bg-[#7132f5]" style="padding: 82px 0 160px;">
		<div class="w-full box-border px-[30px] min-[1040px]:px-[calc((100%-908px)/2)]">
			<div class="block">
				<ul class="footer-primary-list columns-2 list-none">
					<li>
						<span class="text-[#C0B9FF]">Features</span>
						<ul>
							<li><a href="/features/margin-trading">Margin Trading</a></li>
							<li><a href="/features/otc-exchange">OTC Trading</a></li>
							<li><a href="/institutions">Institutions</a></li>
							<li><a href="/features/api-trading">API Trading</a></li>
							<li><a href="/features">All features</a></li>
						</ul>
					</li>
					<li>
						<span class="text-[#C0B9FF]">Popular Markets</span>
						<ul>
							<li><a href="/convert/btc/usd">BTC to USD</a></li>
							<li><a href="/convert/eth/usd">ETH to USD</a></li>
							<li><a href="/convert">All crypto markets</a></li>
						</ul>
					</li>
					<li>
						<span class="text-[#C0B9FF]"> Buying Guides </span>
						<ul>
							<li><a href="/learn/buy-bitcoin-btc">Buy Bitcoin</a></li>
							<li><a href="/learn/buy-ethereum-eth">Buy Ethereum</a></li>
						</ul>
					</li>
					<li>
						<span class="text-[#C0B9FF]"> Browse Prices </span>
						<ul>
							<li><a href="/prices/bitcoin">Bitcoin Price</a></li>
							<li><a href="/prices/ethereum">Ethereum Price</a></li>
							<li><a href="/prices">All crypto prices</a></li>
						</ul>
					</li>
					<li>
						<span class="text-[#C0B9FF]"> Crypto Education </span>
						<ul>
							<li><a href="/learn/what-is-cryptocurrency">What is cryptocurrency?</a></li>
							<li><a href="/learn/types-of-cryptocurrency">Types of cryptocurrency</a></li>
							<li><a href="/learn/what-is-blockchain-technology">What is a blockchain?</a></li>
							<li><a href="/learn/what-is-bitcoin-btc">What is Bitcoin?</a></li>
							<li><a href="/learn/what-is-ethereum-eth">What is Ethereum?</a></li>
							<li><a href="/learn">Learn crypto</a></li>
						</ul>
					</li>
				</ul>
			</div>
		</div>
	</footer>
</div>
