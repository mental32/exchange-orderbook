<script>
	import { goto } from '$app/navigation';
</script>

<header class="bg-[#7132F5]">
	<div class="w-full py-4 mx-auto px-4 sm:px-6 lg:px-8">
		<div class="flex justify-between items-center py-3">
			<a class="logo text-white font-bold text-xl lg:text-2xl mr-1" href="/">Crypto Exchange</a>
			<div class="nav-actions space-x-4 text-white">
				<nav class="flex grow z-[5] items-center">
					<ul class="flex grow list-none">
						<li class="flex relative items-center mr-[32px] text-gray-900 hover:text-gray-900 dark:text-gray-50 dark:hover:text-gray-50 transition-all duration-300 ease-in-out before:absolute before:bottom-0 before:left-0 before:w-0 before:h-[2px] before:bg-gray-900 before:transition-all before:duration-300 before:ease-in-out hover:before:w-full dark:before:bg-gray-50">
							<a href="/"> Explore </a>
						</li>
						<li class="flex relative items-center mr-[32px] text-gray-900 hover:text-gray-900 dark:text-gray-50 dark:hover:text-gray-50 transition-all duration-300 ease-in-out before:absolute before:bottom-0 before:left-0 before:w-0 before:h-[2px] before:bg-gray-900 before:transition-all before:duration-300 before:ease-in-out hover:before:w-full dark:before:bg-gray-50">
							<a href="/prices"> Prices </a>
						</li>
						<li class="flex relative items-center mr-[32px] text-gray-900 hover:text-gray-900 dark:text-gray-50 dark:hover:text-gray-50 transition-all duration-300 ease-in-out before:absolute before:bottom-0 before:left-0 before:w-0 before:h-[2px] before:bg-gray-900 before:transition-all before:duration-300 before:ease-in-out hover:before:w-full dark:before:bg-gray-50">
							<a href="/"> Why us?</a>
						</li>
						<li class="flex relative items-center mr-[32px] text-gray-900 hover:text-gray-900 dark:text-gray-50 dark:hover:text-gray-50 transition-all duration-300 ease-in-out before:absolute before:bottom-0 before:left-0 before:w-0 before:h-[2px] before:bg-gray-900 before:transition-all before:duration-300 before:ease-in-out hover:before:w-full dark:before:bg-gray-50">
							<a href="/learn"> Learn </a>
						</li>
						<li class="flex relative items-center mr-[32px] text-gray-900 hover:text-gray-900 dark:text-gray-50 dark:hover:text-gray-50 transition-all duration-300 ease-in-out before:absolute before:bottom-0 before:left-0 before:w-0 before:h-[2px] before:bg-gray-900 before:transition-all before:duration-300 before:ease-in-out hover:before:w-full dark:before:bg-gray-50">
							<a href="/"> Support </a>
						</li>
					</ul>
					<ul class="list-none flex">
						<li class="self-baseline ml-ml-[16px]">
							<button
								class="sign-in bg-transparent text-white py-2 px-4 border border-[#D9F0FF] rounded-full hover:bg-[#D9F0FF] hover:text-[#7132F5] focus:outline-none transition duration-150 ease-in-out border-solid"
								on:click={() => goto('/sign-in')}>Sign in</button
							>
						</li>
						<li class="self-baseline ml-[16px]">
							<button
								class="sign-up bg-white text-[#7132F5] py-2 px-4 border border-solid rounded-full hover:bg-opacity-75 focus:outline-none transition duration-150 ease-in-out"
								on:click={() => goto('/sign-up')}>Sign up</button
							>
						</li>
					</ul>
				</nav>
			</div>
		</div>
	</div>
</header>
