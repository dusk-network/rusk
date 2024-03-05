import { redirect } from "$lib/navigation";

/** @type {import('./$types').PageLoad} */
export function load() {
  redirect(301, "/setup");
}
