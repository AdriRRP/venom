import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen } from "@testing-library/react";
import type { ReactNode } from "react";
import { FindingsPage } from "./findings";

vi.mock("@tanstack/react-router", async () => ({
	Link: ({
		children,
		className,
		to,
	}: {
		children: ReactNode;
		className?: string;
		to: string;
	}) => (
		<a className={className} href={to}>
			{children}
		</a>
	),
}));

describe("FindingsPage", () => {
	it("renders the collection and artifact operator views", () => {
		globalThis.fetch = vi.fn(async (input: string | URL | Request) => {
			const url = String(input);
			if (url.includes("/health")) {
				return new Response("ok", { status: 200 });
			}
			if (url.includes("/collections/")) {
				return new Response(
					JSON.stringify({
						collection_key: "release:2026.05",
						min_severity: null,
						package_name: null,
						total_active_findings: 0,
						returned: 0,
						offset: 0,
						limit: 50,
						active_findings: [],
					}),
					{ status: 200, headers: { "Content-Type": "application/json" } },
				);
			}
			return new Response(
				JSON.stringify({
					component_key: "component:payments-api",
					artifact_kind: "container-image",
					artifact_identity: "registry.example/payments@sha256:111",
					min_severity: null,
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

		render(
			<QueryClientProvider client={new QueryClient()}>
				<FindingsPage />
			</QueryClientProvider>,
		);

		expect(
			screen.getByRole("heading", {
				level: 2,
				name: "Collection Active Findings",
			}),
		).toBeInTheDocument();
		expect(
			screen.getByRole("heading", {
				level: 2,
				name: "Artifact Active Findings",
			}),
		).toBeInTheDocument();
		expect(
			screen.getByText("No active findings for this collection yet."),
		).toBeInTheDocument();
		expect(screen.getByText("No active findings yet.")).toBeInTheDocument();
	});

	it("submits the collection query with package and governance filters", async () => {
		const calls: string[] = [];

		globalThis.fetch = vi.fn(async (input: string | URL | Request) => {
			const url = String(input);
			calls.push(url);
			if (url.includes("/health")) {
				return new Response("ok", { status: 200 });
			}
			if (url.includes("/collections/")) {
				return new Response(
					JSON.stringify({
						collection_key: "release:2026.05",
						min_severity: "high",
						governance_state: "suppressed",
						package_name: "openssl",
						total_active_findings: 1,
						returned: 1,
						offset: 0,
						limit: 50,
						active_findings: [
							{
								component_key: "component:payments-api",
								artifact_kind: "container-image",
								artifact_identity: "registry.example/payments@sha256:111",
								vulnerability_id: "CVE-2026-0001",
								package_name: "openssl",
								package_version: "3.0.0",
								package_purl: null,
								severity: "high",
								governance_state: "open",
								governance_reason: null,
								governance_until_unix_ms: null,
							},
						],
					}),
					{ status: 200, headers: { "Content-Type": "application/json" } },
				);
			}
			return new Response(
				JSON.stringify({
					component_key: "component:payments-api",
					artifact_kind: "container-image",
					artifact_identity: "registry.example/payments@sha256:111",
					min_severity: null,
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

		render(
			<QueryClientProvider client={new QueryClient()}>
				<FindingsPage />
			</QueryClientProvider>,
		);

		const packageInputs = screen.getAllByRole("textbox", {
			name: "Package name",
		});
		const collectionPackageInput = packageInputs[0];
		expect(collectionPackageInput).toBeDefined();
		if (!collectionPackageInput) {
			throw new Error("expected collection package input");
		}
		fireEvent.change(collectionPackageInput, {
			target: { value: "openssl" },
		});
		const governanceSelects = screen.getAllByRole("combobox", {
			name: "Governance",
		});
		const collectionGovernanceSelect = governanceSelects[0];
		expect(collectionGovernanceSelect).toBeDefined();
		if (!collectionGovernanceSelect) {
			throw new Error("expected collection governance select");
		}
		fireEvent.change(collectionGovernanceSelect, {
			target: { value: "suppressed" },
		});
		fireEvent.click(screen.getByRole("button", { name: "Query Collection" }));

		expect(await screen.findByText("Showing 1-1 of 1")).toBeInTheDocument();
		expect(
			calls.some(
				(call) =>
					call.includes(
						"/api/collections/release%3A2026.05/findings/active?",
					) &&
					call.includes("package_name=openssl") &&
					call.includes("governance_state=suppressed"),
			),
		).toBe(true);
		expect(
			await screen.findByText(
				"container-image:registry.example/payments@sha256:111",
			),
		).toBeInTheDocument();
	});

	it("moves between artifact pages with bounded controls", async () => {
		globalThis.fetch = vi.fn(async (input: string | URL | Request) => {
			const url = String(input);
			if (url.includes("/health")) {
				return new Response("ok", { status: 200 });
			}
			if (url.includes("/collections/")) {
				return new Response(
					JSON.stringify({
						collection_key: "release:2026.05",
						min_severity: null,
						package_name: null,
						total_active_findings: 0,
						returned: 0,
						offset: 0,
						limit: 50,
						active_findings: [],
					}),
					{ status: 200, headers: { "Content-Type": "application/json" } },
				);
			}
			if (url.includes("offset=1")) {
				return new Response(
					JSON.stringify({
						component_key: "component:payments-api",
						artifact_kind: "container-image",
						artifact_identity: "registry.example/payments@sha256:111",
						min_severity: null,
						package_name: null,
						total_active_findings: 2,
						returned: 1,
						offset: 1,
						limit: 1,
						active_findings: [
							{
								component_key: "component:payments-api",
								artifact_kind: "container-image",
								artifact_identity: "registry.example/payments@sha256:111",
								vulnerability_id: "CVE-2026-0002",
								package_name: "zlib",
								package_version: "1.3.1",
								package_purl: null,
								severity: "medium",
								governance_state: "open",
								governance_reason: null,
								governance_until_unix_ms: null,
							},
						],
					}),
					{ status: 200, headers: { "Content-Type": "application/json" } },
				);
			}
			return new Response(
				JSON.stringify({
					component_key: "component:payments-api",
					artifact_kind: "container-image",
					artifact_identity: "registry.example/payments@sha256:111",
					min_severity: null,
					package_name: null,
					total_active_findings: 2,
					returned: 1,
					offset: 0,
					limit: 1,
					active_findings: [
						{
							component_key: "component:payments-api",
							artifact_kind: "container-image",
							artifact_identity: "registry.example/payments@sha256:111",
							vulnerability_id: "CVE-2026-0001",
							package_name: "openssl",
							package_version: "3.0.0",
							package_purl: null,
							severity: "high",
							governance_state: "open",
							governance_reason: null,
							governance_until_unix_ms: null,
						},
					],
				}),
				{ status: 200, headers: { "Content-Type": "application/json" } },
			);
		}) as typeof fetch;

		render(
			<QueryClientProvider client={new QueryClient()}>
				<FindingsPage />
			</QueryClientProvider>,
		);

		const limitInputs = screen.getAllByRole("spinbutton", { name: "Limit" });
		const artifactLimitInput = limitInputs[1];
		expect(artifactLimitInput).toBeDefined();
		if (!artifactLimitInput) {
			throw new Error("expected artifact limit input");
		}
		fireEvent.change(artifactLimitInput, {
			target: { value: "1" },
		});
		fireEvent.click(screen.getByRole("button", { name: "Query Artifact" }));
		expect(await screen.findByText("Showing 1-1 of 2")).toBeInTheDocument();

		fireEvent.click(screen.getByRole("button", { name: "Next Artifact Page" }));
		expect(await screen.findByText("Showing 2-2 of 2")).toBeInTheDocument();
		expect(await screen.findByText("zlib@1.3.1")).toBeInTheDocument();

		fireEvent.click(
			screen.getByRole("button", { name: "Previous Artifact Page" }),
		);
		expect(await screen.findByText("openssl@3.0.0")).toBeInTheDocument();
	});

	it("accepts risk for one collection finding and refreshes governance state", async () => {
		let accepted = false;
		const methods: string[] = [];

		globalThis.fetch = vi.fn(
			async (input: string | URL | Request, init?: RequestInit) => {
				const url = String(input);
				methods.push(init?.method ?? "GET");
				if (url.includes("/health")) {
					return new Response("ok", { status: 200 });
				}
				if (url === "/api/findings/risk-acceptance") {
					accepted = true;
					return new Response(
						JSON.stringify({
							change: "accepted",
							governance_state: "risk-accepted",
							governance_reason: "Compensating control in place",
							governance_until_unix_ms: null,
						}),
						{ status: 200, headers: { "Content-Type": "application/json" } },
					);
				}
				if (url.includes("/collections/")) {
					return new Response(
						JSON.stringify({
							collection_key: "release:2026.05",
							min_severity: null,
							package_name: null,
							total_active_findings: 1,
							returned: 1,
							offset: 0,
							limit: 50,
							active_findings: [
								{
									component_key: "component:payments-api",
									artifact_kind: "container-image",
									artifact_identity: "registry.example/payments@sha256:111",
									vulnerability_id: "CVE-2026-0001",
									package_name: "openssl",
									package_version: "3.0.0",
									package_purl: null,
									severity: "high",
									governance_state: accepted ? "risk-accepted" : "open",
									governance_reason: accepted
										? "Compensating control in place"
										: null,
									governance_until_unix_ms: null,
								},
							],
						}),
						{ status: 200, headers: { "Content-Type": "application/json" } },
					);
				}
				return new Response(
					JSON.stringify({
						component_key: "component:payments-api",
						artifact_kind: "container-image",
						artifact_identity: "registry.example/payments@sha256:111",
						min_severity: null,
						package_name: null,
						total_active_findings: 1,
						returned: 1,
						offset: 0,
						limit: 50,
						active_findings: [
							{
								component_key: "component:payments-api",
								artifact_kind: "container-image",
								artifact_identity: "registry.example/payments@sha256:111",
								vulnerability_id: "CVE-2026-0001",
								package_name: "openssl",
								package_version: "3.0.0",
								package_purl: null,
								severity: "high",
								governance_state: accepted ? "risk-accepted" : "open",
								governance_reason: accepted
									? "Compensating control in place"
									: null,
								governance_until_unix_ms: null,
							},
						],
					}),
					{ status: 200, headers: { "Content-Type": "application/json" } },
				);
			},
		) as typeof fetch;

		render(
			<QueryClientProvider client={new QueryClient()}>
				<FindingsPage />
			</QueryClientProvider>,
		);

		fireEvent.click(await screen.findByRole("button", { name: "Accept Risk" }));
		fireEvent.change(screen.getByRole("textbox", { name: "Reason" }), {
			target: { value: "Compensating control in place" },
		});
		fireEvent.click(
			screen.getByRole("button", { name: "Submit Risk Acceptance" }),
		);

		expect(
			await screen.findByText("Governance: risk-accepted (accepted)."),
		).toBeInTheDocument();
		const governedFindings = await screen.findAllByText(
			"risk-accepted: Compensating control in place",
		);
		expect(governedFindings).toHaveLength(2);
		expect(methods).toContain("POST");
	});

	it("suppresses one collection finding and refreshes governance state", async () => {
		let suppressed = false;

		globalThis.fetch = vi.fn(
			async (input: string | URL | Request, _init?: RequestInit) => {
				const url = String(input);
				if (url.includes("/health")) {
					return new Response("ok", { status: 200 });
				}
				if (url === "/api/findings/suppression") {
					suppressed = true;
					return new Response(
						JSON.stringify({
							change: "suppressed",
							governance_state: "suppressed",
							governance_reason: "Known upstream false alarm",
							governance_until_unix_ms: null,
						}),
						{ status: 200, headers: { "Content-Type": "application/json" } },
					);
				}
				if (url.includes("/collections/")) {
					return new Response(
						JSON.stringify({
							collection_key: "release:2026.05",
							min_severity: null,
							package_name: null,
							total_active_findings: 1,
							returned: 1,
							offset: 0,
							limit: 50,
							active_findings: [
								{
									component_key: "component:payments-api",
									artifact_kind: "container-image",
									artifact_identity: "registry.example/payments@sha256:111",
									vulnerability_id: "CVE-2026-0001",
									package_name: "openssl",
									package_version: "3.0.0",
									package_purl: null,
									severity: "high",
									governance_state: suppressed ? "suppressed" : "open",
									governance_reason: suppressed
										? "Known upstream false alarm"
										: null,
									governance_until_unix_ms: null,
								},
							],
						}),
						{ status: 200, headers: { "Content-Type": "application/json" } },
					);
				}
				return new Response(
					JSON.stringify({
						component_key: "component:payments-api",
						artifact_kind: "container-image",
						artifact_identity: "registry.example/payments@sha256:111",
						min_severity: null,
						package_name: null,
						total_active_findings: 1,
						returned: 1,
						offset: 0,
						limit: 50,
						active_findings: [
							{
								component_key: "component:payments-api",
								artifact_kind: "container-image",
								artifact_identity: "registry.example/payments@sha256:111",
								vulnerability_id: "CVE-2026-0001",
								package_name: "openssl",
								package_version: "3.0.0",
								package_purl: null,
								severity: "high",
								governance_state: suppressed ? "suppressed" : "open",
								governance_reason: suppressed
									? "Known upstream false alarm"
									: null,
								governance_until_unix_ms: null,
							},
						],
					}),
					{ status: 200, headers: { "Content-Type": "application/json" } },
				);
			},
		) as typeof fetch;

		render(
			<QueryClientProvider client={new QueryClient()}>
				<FindingsPage />
			</QueryClientProvider>,
		);

		fireEvent.click(await screen.findByRole("button", { name: "Suppress" }));
		fireEvent.change(screen.getByRole("textbox", { name: "Reason" }), {
			target: { value: "Known upstream false alarm" },
		});
		fireEvent.click(screen.getByRole("button", { name: "Submit Suppression" }));

		expect(
			await screen.findByText("Governance: suppressed (suppressed)."),
		).toBeInTheDocument();
		const suppressedFindings = await screen.findAllByText(
			"suppressed: Known upstream false alarm",
		);
		expect(suppressedFindings).toHaveLength(2);
		expect(globalThis.fetch).toHaveBeenCalled();
	});
});
