<svelte:options immutable={true}/>

<script>
	import { makeClassName } from "$lib/dusk/string";
	import "./Anchor.css";

	/** @type {String | Undefined} */
	export let className = undefined;

	/** @type {String} */
	export let href;
</script>

<a
	{...$$restProps}
	class={makeClassName(["dusk-anchor", className])}
	{href}
	on:click
>
	<slot/>
</a>
