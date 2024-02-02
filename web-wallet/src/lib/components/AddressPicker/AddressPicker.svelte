<svelte:options immutable={true}/>

<script>
	import { createEventDispatcher, onMount	} from "svelte";
	import { calculateAdaptiveCharCount, makeClassName, middleEllipsis } from "$lib/dusk/string";
	import {
		mdiContentCopy, mdiPlusBoxOutline, mdiSwapHorizontal, mdiTimerSand
	} from "@mdi/js";
	import {
		Button, CircularIcon, Icon, ProgressBar
	} from "$lib/dusk/components";
	import { toast } from "$lib/dusk/components/Toast/store";
	import { handlePageClick } from "$lib/dusk/ui-helpers/handlePageClick";

	import Overlay from "./Overlay.svelte";

	import "./AddressPicker.css";

	/** @type {string} */
	export let currentAddress;

	/** @type {string[]} */
	export let addresses = [currentAddress];

	/** @type {boolean} */
	export let isAddingAddress = false;

	/** @type {string|undefined} */
	export let className = undefined;

	$: classes = makeClassName(["address-picker", className]);

	const dispatch = createEventDispatcher();

	let expanded = false;

	function toggle () {
		expanded = !expanded;
	}

	function closeDropDown () {
		expanded = false;
	}

	// Scrolls the address options menu to top on addresses change
	$: if (addresses && addressOptionsMenu) {
		addressOptionsMenu.scrollTo(0, 0);
	}

	/** @type {import("svelte/elements").KeyboardEventHandler<HTMLDivElement>} */
	function handleDropDownKeyDown	(event) {
		if (event.key === "Enter" || event.key === " ") {
			toggle();
		}

		if (event.key === "Escape") {
			closeDropDown();
		}
	}

	/** @type {number} */
	let screenWidth = window.innerWidth;

	onMount(() => {
		const resizeObserver = new ResizeObserver(entries => {
			const entry = entries[0];

			screenWidth = entry.contentRect.width;
		});

		resizeObserver.observe(document.body);

		return () => resizeObserver.disconnect();
	});

	/** @type {HTMLMenuElement} */
	let addressOptionsMenu;
</script>

{#if expanded}
	<Overlay/>
{/if}

<div
	use:handlePageClick={{ callback: closeDropDown, enabled: expanded }}
	class={classes}
	class:address-picker--expanded={expanded}>

	<div
		class="address-picker__trigger"
		role="button"
		tabindex="0"
		aria-haspopup="true"
		aria-expanded={expanded}
		on:keydown={handleDropDownKeyDown}>
		<CircularIcon color="var(--background-color)" bgColor="var(--primary-color)">
			<Icon path={mdiSwapHorizontal} size="large"/>
		</CircularIcon>
		<p class="address-picker__current-address">{middleEllipsis(
			currentAddress,
			calculateAdaptiveCharCount(screenWidth)
		)}</p>
		<span class="address-picker__copy-address-button-wrapper">
			<Button
				aria-label="Copy Address"
				className="address-picker__copy-address-button"
				icon={{ path: mdiContentCopy }}
				on:click={() => {
					navigator.clipboard.writeText(currentAddress);
					toast("success", "Address copied", mdiContentCopy);
				}}
				variant="quaternary"
			/>
		</span>
	</div>

	{#if expanded}
		<div class="address-picker__drop-down">
			<hr/>
			<menu class="address-picker__address-options" bind:this={addressOptionsMenu}>
				{#each addresses as address (address)}
					<li
						class="address-picker__address"
						class:address-picker__address--selected={address === currentAddress}>
						<button
							class="address-picker__address-option-button"
							tabindex="0"
							type="button"
							role="menuitem"
							on:click={() => {
								currentAddress = address;
								closeDropDown();
							}}>{address}</button>
					</li>
				{/each}
			</menu>
			<hr/>
			{#if isAddingAddress}
				<div class="address-picker__generating-address-wrapper">
					<Icon path={mdiTimerSand}/>
					<p>Generating <b>Address</b></p>
				</div>
				<ProgressBar/>
			{:else}
				<Button
					tabindex="0"
					className="address-picker__generate-address-button"
					variant="secondary"
					icon={{ path: mdiPlusBoxOutline }}
					text="Generate Address"
					on:click={(event) => {
						event.preventDefault();
						dispatch("generateAddress");
					}}/>
			{/if}
		</div>
	{/if}
</div>
