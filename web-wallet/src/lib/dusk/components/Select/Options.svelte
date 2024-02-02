<svelte:options immutable={true}/>

<script>
	import { isType } from "lamb";

	/** @type {(v: any) => v is string} */
	const isString = isType("String");

	/** @type SelectOption[] | string[] */
	export let options;
</script>

{#each options as option (isString(option) ? option : option.value)}
	<option
		disabled={isString(option) ? false : option.disabled}
		value={isString(option) ? option : option.value}
	>
		{isString(option) ? option : option.label ?? option.value}
	</option>
{/each}
