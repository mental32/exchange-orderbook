<script>
	import Navbar from '../../Navbar.svelte';
	import '../../app.css';

	let email = '';
	let password = '';

	async function signIn() {
		//   const data = { email, password };
		//   try {
		//     const response = await fetch('/api/auth/signin', {
		//       method: 'POST',
		//       headers: { 'Content-Type': 'application/json' },
		//       body: JSON.stringify(data)
		//     });
		//     if (response.ok) {
		//       // Handle successful sign-in here
		//       const user = await response.json();
		//       console.log(user);
		//     } else {
		//       console.error('response', response);
		//     }
		//   } catch (error) {
		//     console.error(error);
		//   }
	}
</script>

<div class="min-h-screen bg-[#EEECFB]">
	<Navbar />

	<div class="flex justify-center items-center h-screen">
		<div class="bg-white rounded-lg shadow-lg p-6 md:p-8 lg:p-12 max-w-md w-full">
			<h2 class="text-2xl font-bold mb-6 text-gray-800 text-center">Sign In</h2>

			<div class="mb-4">
				<input
					class="w-full px-4 py-2 border rounded-lg focus:outline-none focus:ring-2 focus:ring-[#5841D8] transition duration-200"
					type="email"
					bind:value={email}
					placeholder="Email"
				/>
			</div>

			<div class="mb-6">
				<input
					class="w-full px-4 py-2 border rounded-lg focus:outline-none focus:ring-2 focus:ring-[#5841D8] transition duration-200"
					type="password"
					bind:value={password}
					placeholder="Password"
				/>
			</div>

			<div>
				<button
					class="w-full bg-[#5841D8] text-white py-2 px-4 rounded-lg hover:bg-purple-700 transition duration-200"
					on:click={signIn}>Sign In</button
				>
			</div>
		</div>
	</div>
</div>
