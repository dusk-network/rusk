import { redirect } from "$lib/navigation";

/** @type {import('./$types').PageLoad} */
export function load () {
	throw redirect(301, "/setup");
}
