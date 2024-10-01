import { redirect } from "$lib/navigation";

/** @type {import("./$types").LayoutLoad} */
export async function load() {
  if (import.meta.env.VITE_MODE_MAINTENANCE === "true") {
    redirect(307, "/maintenance");
  }
}
