import { redirect } from "$lib/navigation";
import { get } from "svelte/store";

import { walletStore } from "$lib/stores";

/** @type {import("./$types").LayoutLoad} */
export async function load() {
  if (import.meta.env.VITE_MODE_MAINTENANCE === "true") {
    redirect(307, "/maintenance");
  }
  if (!get(walletStore).initialized) {
    redirect(307, "/");
  }
}
