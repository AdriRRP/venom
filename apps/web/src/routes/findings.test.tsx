import { QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, within } from "@testing-library/react";
import type { ReactNode } from "react";
import { createTestQueryClient } from "../test/query-client";
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
	it("renders the collection and artifact operator views", async () => {
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
						health: {
							total: 0,
							open: 0,
							risk_accepted: 0,
							suppressed: 0,
							critical_risk: 0,
							high_risk: 0,
						},
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
			<QueryClientProvider client={createTestQueryClient()}>
				<FindingsPage />
			</QueryClientProvider>,
		);

		expect(
			await screen.findByRole("heading", {
				level: 2,
				name: "Collection Active Findings",
			}),
		).toBeInTheDocument();
		expect(
			await screen.findByRole("heading", {
				level: 2,
				name: "Artifact Active Findings",
			}),
		).toBeInTheDocument();
		expect(
			await screen.findByText("No active findings for this collection yet."),
		).toBeInTheDocument();
		expect(
			await screen.findByText("No active findings yet."),
		).toBeInTheDocument();
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
						health: {
							total: 3,
							open: 1,
							risk_accepted: 1,
							suppressed: 1,
							critical_risk: 1,
							high_risk: 1,
						},
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
								contextual_risk: "critical",
								contextual_posture: "public-edge",
								contextual_rule:
									"internet-exposed + production + mission-critical",
								contextual_factor_provenance: [
									{
										factor: "internet-exposed:true",
										source: "component",
										identity: "context:internet-prod",
									},
									{
										factor: "production:true",
										source: "component",
										identity: "context:internet-prod",
									},
								],
								context_profile_key: "context:internet-prod",
								context_profile_name: "Internet Production",
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
			<QueryClientProvider client={createTestQueryClient()}>
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
		expect(
			await screen.findByRole("cell", { name: "critical" }),
		).toBeInTheDocument();
		expect(await screen.findByText("Internet Production")).toBeInTheDocument();
		expect(await screen.findByText("Posture")).toBeInTheDocument();
		expect(await screen.findByText("public-edge")).toBeInTheDocument();
		expect(await screen.findByText("Rule")).toBeInTheDocument();
		expect(
			await screen.findByText(
				"internet-exposed + production + mission-critical",
			),
		).toBeInTheDocument();
		expect(await screen.findByText("Effective Factors")).toBeInTheDocument();
		expect(
			await screen.findByRole("columnheader", { name: "Factor" }),
		).toBeInTheDocument();
		expect(
			await screen.findByRole("cell", { name: "Internet Exposed" }),
		).toBeInTheDocument();
		expect(
			await screen.findByRole("cell", { name: "true" }),
		).toBeInTheDocument();
		expect(
			await screen.findByRole("cell", { name: "component" }),
		).toBeInTheDocument();
		expect(
			await screen.findByRole("cell", { name: "context:internet-prod" }),
		).toBeInTheDocument();
		expect(
			await screen.findByText(
				"Health: 3 active - 1 open - 1 risk accepted - 1 suppressed - 1 critical risk - 1 high risk",
			),
		).toBeInTheDocument();
		expect(
			await screen.findByRole("button", { name: "Suppressed (1)" }),
		).toBeInTheDocument();
	});

	it("accepts filtered open collection findings in bulk", async () => {
		const calls: Array<{ url: string; init?: RequestInit }> = [];

		globalThis.fetch = vi.fn(
			async (input: string | URL | Request, init?: RequestInit) => {
				const url = String(input);
				calls.push({ url, init });
				if (url.includes("/health")) {
					return new Response("ok", { status: 200 });
				}
				if (url.includes("/findings/risk-acceptance")) {
					return new Response(
						JSON.stringify({
							collection_key: "release:2026.05",
							min_severity: "high",
							package_name: "openssl",
							targeted: 1,
							accepted: 1,
							unchanged: 0,
							governance_state: "risk-accepted",
							governance_reason: "Accepted for this release",
							governance_until_unix_ms: 1760000000000,
						}),
						{ status: 200, headers: { "Content-Type": "application/json" } },
					);
				}
				if (url.includes("/collections/")) {
					return new Response(
						JSON.stringify({
							collection_key: "release:2026.05",
							min_severity: "high",
							governance_state: "open",
							package_name: "openssl",
							health: {
								total: 1,
								open: 1,
								risk_accepted: 0,
								suppressed: 0,
								critical_risk: 1,
								high_risk: 0,
							},
							bulk_governance: {
								targeted: 1,
								critical_risk: 1,
								high_risk: 0,
							},
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
									severity: "critical",
									contextual_risk: "critical",
									context_profile_key: "context:internet-prod",
									context_profile_name: "Internet Production",
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
			},
		) as typeof fetch;

		render(
			<QueryClientProvider client={createTestQueryClient()}>
				<FindingsPage />
			</QueryClientProvider>,
		);

		const packageInputs = screen.getAllByRole("textbox", {
			name: "Package name",
		});
		fireEvent.change(packageInputs[0], { target: { value: "openssl" } });

		const severitySelects = screen.getAllByRole("combobox", {
			name: "Minimum severity",
		});
		fireEvent.change(severitySelects[0], { target: { value: "high" } });

		const governanceSelects = screen.getAllByRole("combobox", {
			name: "Governance",
		});
		fireEvent.change(governanceSelects[0], { target: { value: "open" } });
		fireEvent.click(screen.getByRole("button", { name: "Query Collection" }));

		expect(
			await screen.findByText(
				"Target cohort: 1 open findings, 1 critical risk, 0 high risk.",
			),
		).toBeInTheDocument();
		fireEvent.change(
			screen.getByRole("textbox", { name: "Governance reason" }),
			{
				target: { value: "Accepted for this release" },
			},
		);
		fireEvent.change(
			screen.getByRole("spinbutton", { name: "Until unix ms" }),
			{
				target: { value: "1760000000000" },
			},
		);
		fireEvent.change(
			screen.getByRole("combobox", { name: "Bulk governance action" }),
			{ target: { value: "accept-risk" } },
		);
		fireEvent.click(
			await screen.findByRole("button", { name: "Apply Bulk Governance" }),
		);

		expect(
			await screen.findByText("Governance: risk-accepted (1/1 accepted)."),
		).toBeInTheDocument();
		expect(
			calls.some(
				(call) =>
					call.url ===
						"/api/collections/release%3A2026.05/findings/risk-acceptance" &&
					String(call.init?.body).includes('"min_severity":"high"') &&
					String(call.init?.body).includes('"package_name":"openssl"') &&
					String(call.init?.body).includes(
						'"reason":"Accepted for this release"',
					),
			),
		).toBe(true);
	});

	it("suppresses filtered open collection findings in bulk", async () => {
		const calls: Array<{ url: string; init?: RequestInit }> = [];

		globalThis.fetch = vi.fn(
			async (input: string | URL | Request, init?: RequestInit) => {
				const url = String(input);
				calls.push({ url, init });
				if (url.includes("/health")) {
					return new Response("ok", { status: 200 });
				}
				if (url.includes("/findings/suppression")) {
					return new Response(
						JSON.stringify({
							collection_key: "release:2026.05",
							min_severity: "high",
							package_name: "openssl",
							targeted: 1,
							suppressed: 1,
							unchanged: 0,
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
							min_severity: "high",
							governance_state: "open",
							package_name: "openssl",
							health: {
								total: 1,
								open: 1,
								risk_accepted: 0,
								suppressed: 0,
								critical_risk: 1,
								high_risk: 0,
							},
							bulk_governance: {
								targeted: 1,
								critical_risk: 1,
								high_risk: 0,
							},
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
									severity: "critical",
									contextual_risk: "critical",
									context_profile_key: "context:internet-prod",
									context_profile_name: "Internet Production",
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
			},
		) as typeof fetch;

		render(
			<QueryClientProvider client={createTestQueryClient()}>
				<FindingsPage />
			</QueryClientProvider>,
		);

		const packageInputs = screen.getAllByRole("textbox", {
			name: "Package name",
		});
		fireEvent.change(packageInputs[0], { target: { value: "openssl" } });

		const severitySelects = screen.getAllByRole("combobox", {
			name: "Minimum severity",
		});
		fireEvent.change(severitySelects[0], { target: { value: "high" } });

		const governanceSelects = screen.getAllByRole("combobox", {
			name: "Governance",
		});
		fireEvent.change(governanceSelects[0], { target: { value: "open" } });
		fireEvent.click(screen.getByRole("button", { name: "Query Collection" }));

		expect(
			await screen.findByText(
				"Target cohort: 1 open findings, 1 critical risk, 0 high risk.",
			),
		).toBeInTheDocument();
		fireEvent.change(
			screen.getByRole("combobox", { name: "Bulk governance action" }),
			{ target: { value: "suppress" } },
		);
		fireEvent.change(
			screen.getByRole("textbox", { name: "Governance reason" }),
			{
				target: { value: "Known upstream false alarm" },
			},
		);
		fireEvent.click(
			await screen.findByRole("button", { name: "Apply Bulk Governance" }),
		);

		expect(
			await screen.findByText("Governance: suppressed (1/1 suppressed)."),
		).toBeInTheDocument();
		expect(
			calls.some(
				(call) =>
					call.url ===
						"/api/collections/release%3A2026.05/findings/suppression" &&
					String(call.init?.body).includes('"min_severity":"high"') &&
					String(call.init?.body).includes('"package_name":"openssl"') &&
					String(call.init?.body).includes(
						'"reason":"Known upstream false alarm"',
					),
			),
		).toBe(true);
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
						health: {
							total: 0,
							open: 0,
							risk_accepted: 0,
							suppressed: 0,
							critical_risk: 0,
							high_risk: 0,
						},
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
								contextual_risk: "medium",
								context_profile_key: null,
								context_profile_name: null,
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
							contextual_risk: "high",
							context_profile_key: null,
							context_profile_name: null,
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
			<QueryClientProvider client={createTestQueryClient()}>
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
							health: {
								total: 1,
								open: accepted ? 0 : 1,
								risk_accepted: accepted ? 1 : 0,
								suppressed: 0,
								critical_risk: 1,
								high_risk: 0,
							},
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
									contextual_risk: accepted ? "critical" : "critical",
									context_profile_key: "context:internet-prod",
									context_profile_name: "Internet Production",
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
								contextual_risk: accepted ? "critical" : "critical",
								context_profile_key: "context:internet-prod",
								context_profile_name: "Internet Production",
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
			<QueryClientProvider client={createTestQueryClient()}>
				<FindingsPage />
			</QueryClientProvider>,
		);

		fireEvent.click(await screen.findByRole("button", { name: "Accept Risk" }));
		const acceptRiskForm = screen
			.getByRole("button", { name: "Submit Risk Acceptance" })
			.closest("form");
		expect(acceptRiskForm).not.toBeNull();
		fireEvent.change(
			within(acceptRiskForm as HTMLElement).getByRole("textbox", {
				name: "Reason",
			}),
			{
				target: { value: "Compensating control in place" },
			},
		);
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
							health: {
								total: 1,
								open: suppressed ? 0 : 1,
								risk_accepted: 0,
								suppressed: suppressed ? 1 : 0,
								critical_risk: 1,
								high_risk: 0,
							},
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
									contextual_risk: suppressed ? "critical" : "critical",
									context_profile_key: "context:internet-prod",
									context_profile_name: "Internet Production",
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
								contextual_risk: suppressed ? "critical" : "critical",
								context_profile_key: "context:internet-prod",
								context_profile_name: "Internet Production",
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
			<QueryClientProvider client={createTestQueryClient()}>
				<FindingsPage />
			</QueryClientProvider>,
		);

		fireEvent.click(await screen.findByRole("button", { name: "Suppress" }));
		const suppressForm = screen
			.getByRole("button", { name: "Submit Suppression" })
			.closest("form");
		expect(suppressForm).not.toBeNull();
		fireEvent.change(
			within(suppressForm as HTMLElement).getByRole("textbox", {
				name: "Reason",
			}),
			{
				target: { value: "Known upstream false alarm" },
			},
		);
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

	it("reopens one governed collection finding and refreshes governance state", async () => {
		let suppressed = true;

		globalThis.fetch = vi.fn(
			async (input: string | URL | Request, _init?: RequestInit) => {
				const url = String(input);
				if (url.includes("/health")) {
					return new Response("ok", { status: 200 });
				}
				if (url === "/api/findings/reopen") {
					suppressed = false;
					return new Response(
						JSON.stringify({
							change: "reopened",
							governance_state: "open",
							governance_reason: null,
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
							health: {
								total: 1,
								open: suppressed ? 0 : 1,
								risk_accepted: 0,
								suppressed: suppressed ? 1 : 0,
								critical_risk: 1,
								high_risk: 0,
							},
							bulk_governance: {
								targeted: suppressed ? 1 : 1,
								critical_risk: 1,
								high_risk: 0,
							},
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
									contextual_risk: "critical",
									context_profile_key: "context:internet-prod",
									context_profile_name: "Internet Production",
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
								contextual_risk: "critical",
								context_profile_key: "context:internet-prod",
								context_profile_name: "Internet Production",
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
			<QueryClientProvider client={createTestQueryClient()}>
				<FindingsPage />
			</QueryClientProvider>,
		);

		const suppressedFindings = await screen.findAllByText(
			"suppressed: Known upstream false alarm",
		);
		expect(suppressedFindings).toHaveLength(2);
		fireEvent.click(await screen.findByRole("button", { name: "Reopen" }));
		fireEvent.click(screen.getByRole("button", { name: "Submit Reopen" }));

		expect(
			await screen.findByText("Governance: open (reopened)."),
		).toBeInTheDocument();
		expect(
			await screen.findByText(
				"Health: 1 active - 1 open - 0 risk accepted - 0 suppressed - 1 critical risk - 0 high risk",
			),
		).toBeInTheDocument();
	});
});
