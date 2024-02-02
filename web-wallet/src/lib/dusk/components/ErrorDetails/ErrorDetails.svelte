<svelte:options immutable={true}/>

<script>
	import { makeClassName } from "$lib/dusk/string";

	/** @type {String | Undefined} */
	export let className = undefined;

	/** @type {Error | null} */
	export let error;

	/** @type {String} */
	export let summary;

	$: classes = makeClassName([
		"dusk-error-details",
		"dusk-error-details__details",
		className
	]);
</script>

{#if error}
	<details class={classes}>
		<summary class="dusk-error-details__summary">
			{summary}
		</summary>
		<pre class="dusk-error-details__error">
			{error.message}
		</pre>
	</details>
{/if}
