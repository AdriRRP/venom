import { fetchActiveFindings, fetchApiHealth } from "./api";

describe("fetchApiHealth", () => {
	it("maps a successful health response to the healthy state", async () => {
		globalThis.fetch = vi.fn(async (input: string | URL | Request) => {
			expect(String(input)).toBe("/api/health");
			return new Response("ok", { status: 200 });
		}) as typeof fetch;

		await expect(fetchApiHealth()).resolves.toBe("healthy");
	});

	it("serializes the canonical query shape expected by the API", async () => {
		const calls: string[] = [];
		globalThis.fetch = vi.fn(async (input: string | URL | Request) => {
			calls.push(String(input));
			return new Response(
				JSON.stringify({
					component_key: "component:payments-api",
					artifact_kind: "container-image",
					artifact_identity: "registry.example/payments@sha256:111",
					min_severity: "high",
					package_name: null,
					total_active_findings: 0,
					returned: 0,
					offset: 0,
					limit: 50,
					active_findings: [],
				}),
				{ status: 200, headers: { "Content-Type": "application/json" } },
			);
		}) as typeof fetch;

		await fetchActiveFindings({
			componentKey: "component:payments-api",
			artifactKind: "container-image",
			artifactIdentity: "registry.example/payments@sha256:111",
			minSeverity: "high",
		});

		expect(calls[0]).toContain("/api/findings/active?");
		expect(calls[0]).toContain("component_key=component%3Apayments-api");
		expect(calls[0]).toContain("artifact_kind=container-image");
		expect(calls[0]).toContain("min_severity=high");
	});
});
