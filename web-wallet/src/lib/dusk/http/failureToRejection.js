/**
 * Transforms a Response into a Promise that resolves
 * with the Reponse itself if its status is ok, and
 * rejects with an Error otherwise.
 *
 * @param {Response} response
 * @returns {Promise<Response>}
 */
const failureToRejection = response => (
	response.ok
		? Promise.resolve(response)
		: Promise.reject(new Error(
			`HTTP Request failed - ${response.statusText}`,
			{ cause: response }
		))
);

export default failureToRejection;
