<nav class="top-nav">
	<div class="nav-container">
		<div class="logo">Exchange</div>
		<div class="nav-actions">
			<button class="sign-in">Sign in</button>
			<button class="sign-up">Sign up</button>
		</div>
	</div>
</nav>

<style>
	.top-nav {
		position: absolute;
		top: 0;
		left: 0;
		width: 100%;
		background-color: #5841d8;
		z-index: 1000;
		font-family: 'Arial', sans-serif;
		/* letter-spacing: 0.01em; */
	}

	.logo {
		color: white;
		font-weight: bold;
		font-size: 1.2rem;
		margin-right: 1rem;
	}

	.nav-container {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0.5rem 1rem;
	}

	.nav-actions button {
		border-radius: 20px;
		text-decoration: none;
		color: white;
		padding: 0.5rem 1rem; /* Adjust the padding to reduce the space between buttons */
		margin-left: 0.5rem; /* Adjust the margin to reduce the space between buttons */
		border: none;
		background-color: transparent;
		cursor: pointer;
		box-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
		border: 0.5px solid rgba(255, 255, 255, 0.1);
		border-color: white;
	}

	.nav-actions button.sign-in:hover {
		background-color: rgba(255, 255, 255, 0.1); /* Light up slightly on hover */
	}

	.nav-actions button.sign-up:hover {
		opacity: 0.8; /* Dim slightly on hover */
	}

	.nav-actions button.sign-up {
		background-color: white;
		color: #5841d8;
	}
</style>
