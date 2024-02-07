
import { goto } from "$lib/navigation";
import { walletStore } from "$lib/stores";

/**
 * Logs out the user.
 * If `isForced` is set to `true`, a querystring
 * parameter is added to instruct the home page
 * to add an explanation about the forced logout.
 *
 * @param {boolean} isForced
 * @returns {ReturnType<goto>}
 */
const logout = isForced => {
	walletStore.abortSync();
	walletStore.reset();

	return goto(`/${isForced ? "forced-logout" : ""}`);
};

export default logout;
