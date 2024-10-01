/** @type {import('@sveltejs/kit').Reroute} */
export function reroute() {
  return import.meta.env.VITE_MODE_MAINTENANCE === "true"
    ? `${import.meta.env.VITE_BASE_PATH}/maintenance`
    : undefined;
}
