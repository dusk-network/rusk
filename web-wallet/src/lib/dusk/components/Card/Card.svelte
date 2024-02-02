<script>
	import Icon from "../Icon/Icon.svelte";
	import Switch from "../Switch/Switch.svelte";
	import { makeClassName } from "$lib/dusk/string";

	/** @type {string} */
	export let tag = "div";

	/** @type {string | Undefined} */
	export let className = undefined;

	/** @type {string | Undefined} */
	export let iconPath = undefined;

	/** @type {string} */
	export let heading;

	export let hasToggle = false;
	export let isToggled = false;

	$: classes = makeClassName(["dusk-card", className]);
</script>

<svelte:element this={tag} {...$$restProps} class={classes}>
	<header
		class="dusk-card__header"
		class:dusk-card__header--toggle-off={hasToggle && !isToggled}>
		{#if iconPath}
			<Icon className="dusk-card__icon" path={iconPath}/>
		{/if}
		<h3 class="h4">{heading}</h3>
		{#if hasToggle}
			<Switch onSurface bind:value={isToggled}/>
		{/if}
	</header>
	{#if !hasToggle || isToggled}
		<slot/>
	{/if}
</svelte:element>
