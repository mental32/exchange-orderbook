<script>
	import Navbar from '../../Navbar.svelte';
	import '../../app.css';

	let email = '';
	let password = '';
	let name = '';
	let agree = false;

	async function createAccount() {
		const data = { name, email, password };
		try {
			const response = await fetch('/api/user', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify(data)
			});

			if (response.ok) {
				const user = await response.json();
				console.log(user);
			} else {
				console.error('response', response);
			}
		} catch (error) {
			console.error(error);
		}
	}
</script>

<div class="min-h-screen bg-[#EEECFB]">
	<Navbar />

	<div class="flex justify-center items-center h-screen">
		<div class="bg-white rounded-lg shadow-lg p-6 md:p-8 lg:p-12 max-w-md w-full">
			<h2 class="text-2xl font-bold mb-6 text-gray-800 text-center">Create Account</h2>

			<div class="mb-4">
				<input
					class="w-full px-4 py-2 border rounded-lg focus:outline-none focus:ring-2 focus:ring-[#5841D8] transition duration-200"
					type="email"
					bind:value={email}
					placeholder="Email"
				/>
			</div>

			<div class="mb-4">
				<input
					class="w-full px-4 py-2 border rounded-lg focus:outline-none focus:ring-2 focus:ring-[#5841D8] transition duration-200"
					bind:value={name}
					placeholder="Name"
				/>
			</div>

			<div class="mb-4">
				<input
					class="w-full px-4 py-2 border rounded-lg focus:outline-none focus:ring-2 focus:ring-[#5841D8] transition duration-200"
					type="password"
					bind:value={password}
					placeholder="Password"
				/>
			</div>

			<div class="mb-6">
				<label class="flex items-center">
					<input
						type="checkbox"
						class="form-checkbox h-4 w-4 text-[#5841D8] rounded"
						bind:checked={agree}
					/>
					<span class="ml-2 text-sm text-gray-600"
						>Agree to the Terms of Service and Privacy Policy</span
					>
				</label>
			</div>

			<div>
				<button
					class="w-full bg-[#5841D8] text-white py-2 px-4 rounded-lg hover:bg-purple-700 disabled:opacity-50 disabled:cursor-not-allowed transition duration-200"
					disabled={!(agree && email && password && name)}
					on:click={createAccount}>Create account</button
				>
			</div>
		</div>
	</div>
</div>
