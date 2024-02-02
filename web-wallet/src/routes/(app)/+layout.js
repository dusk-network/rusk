import { redirect } from "@sveltejs/kit";
import { get } from "svelte/store";

import walletStore from "$lib/stores/walletStore";

/** @type {import("./$types").LayoutLoad} */
export async function load () {
	if (!get(walletStore).initialized) {
		throw redirect(307, "/");
	}
}
