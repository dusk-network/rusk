<script>
	import { mdiHome } from "@mdi/js";

	import { Tabs } from "$lib/dusk/components";

	/** @param {CustomEvent} event */
	function handleTabChange (event) {
		// eslint-disable-next-line no-console
		console.log(`tab changed to ${event.detail}`);
	}

	const tabs1 = [
		{ id: "tab-1", label: "Tab label 1" },
		{ id: "tab-2", label: "Tab label 2" },
		{ id: "tab-3", label: "Tab label 3" },
		{ id: "tab-4", label: "Tab label 4" },
		{ id: "tab-5", label: "Tab label 5" },
		{ id: "tab-6", label: "Tab label 6" }
	];
	const tabs2 = tabs1.slice(0, 4);
	const tabs3 = tabs1.map(tab => ({ ...tab, icon: { path: mdiHome } }));
	const tabs4 = tabs3.map(({ icon, id }) => ({ icon, id }));

	let selected1 = "tab-6";
	let selected2 = "tab-2";
	let selected3 = "tab-5";
	let selected4 = "tab-3";
</script>

<section>
	<Tabs
		bind:selectedTab={selected1}
		items={tabs1}
		on:change={handleTabChange}
	/>
</section>
<section>
	<Tabs
		bind:selectedTab={selected2}
		items={tabs2}
		on:change={handleTabChange}
	/>
</section>
<section>
	<Tabs
		bind:selectedTab={selected3}
		items={tabs3}
		on:change={handleTabChange}
	/>
</section>
<section>
	<Tabs
		bind:selectedTab={selected4}
		items={tabs4}
		on:change={handleTabChange}
	/>
</section>
