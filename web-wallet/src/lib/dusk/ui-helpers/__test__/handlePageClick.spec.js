import {
	describe, expect, it, vi
} from "vitest";
import { handlePageClick } from "../handlePageClick";

describe("handlePageClick", () => {
	it("calls callback on click outside the node", () => {
		const node = document.createElement("div");
		const callback = vi.fn();

		document.body.appendChild(node);

		handlePageClick(node, { callback, enabled: true });

		document.body.click();
		expect(callback).toHaveBeenCalledTimes(1);

		document.body.removeChild(node);
	});

	it("does not call callback on click inside the node", () => {
		const node = document.createElement("div");
		const callback = vi.fn();

		document.body.appendChild(node);

		handlePageClick(node, { callback, enabled: true });

		node.click();
		expect(callback).toHaveBeenCalledTimes(0);

		document.body.removeChild(node);
	});

	it("toggles listener activation based on enabled property", () => {
		const node = document.createElement("div");
		const callback = vi.fn();

		document.body.appendChild(node);

		const action = handlePageClick(node, { callback, enabled: false });

		document.body.click();
		expect(callback).toHaveBeenCalledTimes(0);

		action.update({ enabled: true });

		document.body.click();
		expect(callback).toHaveBeenCalledTimes(1);

		document.body.removeChild(node);
	});

	it("removes event listener on destroy", () => {
		const node = document.createElement("div");
		const callback = vi.fn();

		document.body.appendChild(node);

		const action = handlePageClick(node, { callback, enabled: true });

		action.destroy();

		document.body.click();
		expect(callback).toHaveBeenCalledTimes(0);

		document.body.removeChild(node);
	});
});
