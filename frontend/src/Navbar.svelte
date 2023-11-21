<script>
	import { goto } from '$app/navigation';
</script>

<nav class="top-nav bg-[#5841D8]">
	<div class="nav-container max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
		<div class="flex justify-between items-center py-3">
			<div class="logo text-[#D9F0FF] font-bold text-xl lg:text-2xl">Crypto Exchange</div>
			<div class="nav-actions space-x-4">
				<button
					class="sign-in bg-transparent text-[#D9F0FF] py-2 px-4 border border-[#D9F0FF] rounded-full hover:bg-[#D9F0FF] hover:text-[#50514F] focus:outline-none transition duration-150 ease-in-out"
					on:click={() => goto('/sign-in')}>Sign in</button
				>
				<button
					class="sign-up bg-[#D9F0FF] text-[#50514F] py-2 px-4 border border-transparent rounded-full hover:bg-opacity-75 focus:outline-none transition duration-150 ease-in-out"
					on:click={() => goto('/sign-up')}>Sign up</button
				>
			</div>
		</div>
	</div>
</nav>
