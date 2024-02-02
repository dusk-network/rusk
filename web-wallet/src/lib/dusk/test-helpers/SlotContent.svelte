<script>

	/** @type {Record<String, *>} */
	export let componentOptions;

	/** @type {ConstructorOfATypedSvelteComponent} */
	export let Component;

	/** @type {String} */
	export let text;
</script>

<svelte:component this={Component} {...componentOptions}>
	<span>{text}</span>
</svelte:component>
