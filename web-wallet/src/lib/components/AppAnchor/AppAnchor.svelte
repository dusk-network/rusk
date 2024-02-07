<svelte:options immutable={true}/>

<script>
	import { Anchor } from "$lib/dusk/components";
	import { addBasePath } from "$lib/navigation";

	/** @type {string} */
	export let href;
</script>

<Anchor
	{...$$restProps}
	href={addBasePath(href)}
	on:click
>
	<slot/>
</Anchor>
