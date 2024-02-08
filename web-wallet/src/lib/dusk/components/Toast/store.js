import { get, writable } from "svelte/store";

/**
 * @type {import('svelte/store').Writable<ToastItem[]>}
 * @description Stores each toast as an object in the array
 */
export const toastList = writable([]);

/**
 * @type {import('svelte/store').Writable<Number>}
 * @description Stores the timer fused by all toasts
 */
export const toastTimer = writable(0);

/**
     * @param {string} message
     * @param {string} icon
     * @param {TooltipType} type
     */

function addToast (type, message, icon) {
	const id = `dusk-toast-${ Math.random().toString(36)}`;

	toastList.update(store => {
		return [...store, {
			icon: icon, id: id, message: message, type: type
		}];
	});

	const timeoutID = setTimeout(() => {
		deleteToast(id, timeoutID);
	}, get(toastTimer));
}

/**
 * @param {string} id
 * @param {ReturnType<typeof setTimeout>} timeout
 */
function deleteToast (id, timeout) {
	clearTimeout(timeout);

	// Deletes toast from store queue
	toastList.update(store => {
		return store.filter(toast => toast.id !== id);
	});
}

export const toast = addToast;
