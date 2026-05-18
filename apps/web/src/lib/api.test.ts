import {
	acceptFindingRisk,
	addCollectionComponent,
	bindArtifact,
	configureCollectionScanSchedule,
	configureProvider,
	drainCollectionScanWorker,
	drainScanWorker,
	fetchActiveFindings,
	fetchApiHealth,
	fetchCollectionActiveFindings,
	fetchCollectionDetail,
	fetchCollections,
	fetchScanCommandStatus,
	registerCollection,
	registerComponent,
	requestCollectionScan,
	requestScan,
	suppressFinding,
} from "./api";

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
			packageName: "openssl",
		});

		expect(calls[0]).toContain("/api/findings/active?");
		expect(calls[0]).toContain("component_key=component%3Apayments-api");
		expect(calls[0]).toContain("artifact_kind=container-image");
		expect(calls[0]).toContain("min_severity=high");
		expect(calls[0]).toContain("package_name=openssl");
	});

	it("serializes register, bind, configure, and request-scan mutations", async () => {
		const calls: Array<{ input: string; init?: RequestInit }> = [];
		globalThis.fetch = vi.fn(
			async (input: string | URL | Request, init?: RequestInit) => {
				calls.push({ input: String(input), init });
				return new Response(JSON.stringify({ ok: true }), {
					status: 200,
					headers: { "Content-Type": "application/json" },
				});
			},
		) as typeof fetch;

		await registerComponent({
			componentKey: "component:payments-api",
			name: "Payments API",
		});
		await bindArtifact("component:payments-api", {
			artifactKind: "container-image",
			artifactIdentity: "registry.example/payments@sha256:111",
		});
		await configureProvider("component:payments-api", {
			providerKey: "fixture-provider",
		});
		await requestScan({
			componentKey: "component:payments-api",
			artifactKind: "container-image",
			artifactIdentity: "registry.example/payments@sha256:111",
			freshness: "deterministic",
		});

		expect(calls[0]?.input).toBe("/api/components");
		expect(calls[1]?.input).toContain(
			"/api/components/component%3Apayments-api/artifacts",
		);
		expect(calls[2]?.input).toContain(
			"/api/components/component%3Apayments-api/provider-runtime",
		);
		expect(calls[3]?.input).toBe("/api/scan-requests");
		expect(calls[3]?.init?.body).toContain('"freshness":"deterministic"');
	});

	it("serializes collection creation, membership, scheduling, scan targeting, and read queries", async () => {
		const calls: Array<{ input: string; init?: RequestInit }> = [];
		globalThis.fetch = vi.fn(
			async (input: string | URL | Request, init?: RequestInit) => {
				calls.push({ input: String(input), init });
				return new Response(JSON.stringify({ ok: true }), {
					status: 200,
					headers: { "Content-Type": "application/json" },
				});
			},
		) as typeof fetch;

		await registerCollection({
			collectionKey: "release:2026.05",
			name: "May Release",
		});
		await addCollectionComponent("release:2026.05", {
			componentKey: "component:payments-api",
		});
		await configureCollectionScanSchedule({
			collectionKey: "release:2026.05",
			cadenceMinutes: 60,
			freshness: "deterministic",
		});
		await requestCollectionScan({
			collectionKey: "release:2026.05",
			freshness: "deterministic",
		});
		await fetchCollections();
		await fetchCollectionDetail("release:2026.05");
		await fetchCollectionActiveFindings({
			collectionKey: "release:2026.05",
			minSeverity: "high",
			packageName: "openssl",
		});

		expect(calls[0]?.input).toBe("/api/collections");
		expect(calls[0]?.init?.body).toContain(
			'"collection_key":"release:2026.05"',
		);
		expect(calls[1]?.input).toBe(
			"/api/collections/release%3A2026.05/components",
		);
		expect(calls[1]?.init?.body).toContain(
			'"component_key":"component:payments-api"',
		);
		expect(calls[2]?.input).toBe(
			"/api/collections/release%3A2026.05/scan-schedule",
		);
		expect(calls[2]?.init?.body).toContain('"cadence_minutes":60');
		expect(calls[3]?.input).toBe(
			"/api/collections/release%3A2026.05/scan-requests",
		);
		expect(calls[3]?.init?.body).toContain('"freshness":"deterministic"');
		expect(calls[4]?.input).toBe("/api/collections");
		expect(calls[5]?.input).toBe("/api/collections/release%3A2026.05");
		expect(calls[6]?.input).toContain(
			"/api/collections/release%3A2026.05/findings/active?",
		);
		expect(calls[6]?.input).toContain("min_severity=high");
		expect(calls[6]?.input).toContain("package_name=openssl");
	});

	it("serializes risk acceptance over the canonical finding identity", async () => {
		const calls: Array<{ input: string; init?: RequestInit }> = [];
		globalThis.fetch = vi.fn(
			async (input: string | URL | Request, init?: RequestInit) => {
				calls.push({ input: String(input), init });
				return new Response(
					JSON.stringify({
						change: "accepted",
						governance_state: "risk-accepted",
						governance_reason: "Compensating control in place",
						governance_until_unix_ms: 1760000000000,
					}),
					{ status: 200, headers: { "Content-Type": "application/json" } },
				);
			},
		) as typeof fetch;

		await acceptFindingRisk({
			componentKey: "component:payments-api",
			artifactKind: "container-image",
			artifactIdentity: "registry.example/payments@sha256:111",
			vulnerabilityId: "CVE-2026-0001",
			packageName: "openssl",
			packageVersion: "3.0.0",
			reason: "Compensating control in place",
			untilUnixMs: 1760000000000,
		});

		expect(calls[0]?.input).toBe("/api/findings/risk-acceptance");
		expect(calls[0]?.init?.body).toContain(
			'"vulnerability_id":"CVE-2026-0001"',
		);
		expect(calls[0]?.init?.body).toContain(
			'"reason":"Compensating control in place"',
		);
		expect(calls[0]?.init?.body).toContain('"until_unix_ms":1760000000000');
	});

	it("serializes finding suppression over the canonical finding identity", async () => {
		const calls: Array<{ input: string; init?: RequestInit }> = [];
		globalThis.fetch = vi.fn(
			async (input: string | URL | Request, init?: RequestInit) => {
				calls.push({ input: String(input), init });
				return new Response(
					JSON.stringify({
						change: "suppressed",
						governance_state: "suppressed",
						governance_reason: "Known upstream false alarm",
						governance_until_unix_ms: null,
					}),
					{ status: 200, headers: { "Content-Type": "application/json" } },
				);
			},
		) as typeof fetch;

		await suppressFinding({
			componentKey: "component:payments-api",
			artifactKind: "container-image",
			artifactIdentity: "registry.example/payments@sha256:111",
			vulnerabilityId: "CVE-2026-0001",
			packageName: "openssl",
			packageVersion: "3.0.0",
			reason: "Known upstream false alarm",
		});

		expect(calls[0]?.input).toBe("/api/findings/suppression");
		expect(calls[0]?.init?.body).toContain(
			'"vulnerability_id":"CVE-2026-0001"',
		);
		expect(calls[0]?.init?.body).toContain(
			'"reason":"Known upstream false alarm"',
		);
	});

	it("serializes scan command lookup and worker drain payloads", async () => {
		const calls: Array<{ input: string; init?: RequestInit }> = [];
		globalThis.fetch = vi.fn(
			async (input: string | URL | Request, init?: RequestInit) => {
				calls.push({ input: String(input), init });
				return new Response(JSON.stringify({ ok: true }), {
					status: 200,
					headers: { "Content-Type": "application/json" },
				});
			},
		) as typeof fetch;

		await fetchScanCommandStatus("cmd-1");
		await drainScanWorker({
			maxCommands: 1,
			knowledgeRevision: "fixture-rev-1",
			findings: [
				{
					vulnerabilityId: "CVE-2026-0001",
					packageName: "openssl",
					packageVersion: "3.0.0",
					severity: "high",
				},
			],
		});
		await drainCollectionScanWorker({
			maxCollections: 8,
		});

		expect(calls[0]?.input).toBe("/api/scan-commands/cmd-1");
		expect(calls[1]?.input).toBe("/api/scan-workers/drain");
		expect(calls[1]?.init?.body).toContain(
			'"knowledge_revision":"fixture-rev-1"',
		);
		expect(calls[1]?.init?.body).toContain(
			'"vulnerability_id":"CVE-2026-0001"',
		);
		expect(calls[2]?.input).toBe("/api/collection-scan-workers/drain");
		expect(calls[2]?.init?.body).toContain('"max_collections":8');
	});
});
