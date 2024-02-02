<script>
	import { makeClassName } from "$lib/dusk/string";

	/** @type {String} */
	export let name;

	/** @type {String} */
	export let id = `dusk-checkbox-${Math.random().toString(36)}`;

	/** @type {Boolean} */
	export let disabled = false;

	/** @type {Boolean} */
	export let checked = false;

	/** @type {String|Undefined} */
	export let className = undefined;

	/** @type {Number|Undefined} */
	export let tabindex = undefined;

	const classes = makeClassName(["dusk-checkbox", className]);
</script>

<input
	{...$$restProps}
	type="checkbox"
	{id}
	{tabindex}
	{name}
	{disabled}
	class={classes}
	bind:checked
	on:change
/>
