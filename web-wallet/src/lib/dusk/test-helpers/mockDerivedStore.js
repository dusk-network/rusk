import { get, writable } from "svelte/store";

/**
 * @param {*} initialValue
 * @param {*} fn
 */
function mockDerivedStore (initialValue, fn) {
	const store = writable(initialValue);
	const { set, subscribe } = store;
	const getMockedStoreValue = () => get(store);

	/** @param {*} value */
	const setMockedStoreValue = value => set(value);

	const derivedMockedStoreValue = () => fn(initialValue);

	return {
		derivedMockedStoreValue,
		getMockedStoreValue,
		setMockedStoreValue,
		subscribe
	};
}

export default mockDerivedStore;
