/**
 * Validates a Dusk address, with feedback on failure or success.
 * @param {String} address The public spent key to validate.
 * @returns {{isValid: boolean, reason: string}} An object with two keys:
 *  - `isValid` {Boolean} - true if the address is valid, false if invalid.
 *  - `reason` {String} - describes why the address is invalid or confirms if it is valid.
 */
export default function validateAddress (address) {
	const regex = /[\W_0OIl]/;

	if (address.length < 87 || address.length > 88) {
		return { isValid: false, reason: "Invalid length. Addresses must be 87 or 88 characters long." };
	}

	if (address.search(regex) !== -1) {
		return { isValid: false, reason: "Invalid character set. Address contains forbidden characters." };
	}

	return { isValid: true, reason: "Valid address." };
}
