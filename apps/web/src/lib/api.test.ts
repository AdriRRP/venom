import { fetchApiHealth } from "./api";

describe("fetchApiHealth", () => {
	it("maps a successful health response to the healthy state", async () => {
		globalThis.fetch = vi.fn(async (input: string | URL | Request) => {
			expect(String(input)).toBe("/api/health");
			return new Response("ok", { status: 200 });
		}) as typeof fetch;

		await expect(fetchApiHealth()).resolves.toBe("healthy");
	});
});
