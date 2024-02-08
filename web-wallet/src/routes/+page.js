import { redirect } from "@sveltejs/kit";

/** @type {import('./$types').PageLoad} */
export function load () {
	redirect(301, "/setup");
}
