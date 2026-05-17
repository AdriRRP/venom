import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen } from "@testing-library/react";
import type { ReactNode } from "react";
import { OperationsPage } from "./operations";

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

describe("OperationsPage", () => {
	it("renders the operator mutations landing page", () => {
		globalThis.fetch = vi.fn(
			async () => new Response("ok", { status: 200 }),
		) as typeof fetch;

		render(
			<QueryClientProvider client={new QueryClient()}>
				<OperationsPage />
			</QueryClientProvider>,
		);

		expect(screen.getByText("Managed Components")).toBeInTheDocument();
		expect(
			screen.getByText(/request canonical scans from the operator console/i),
		).toBeInTheDocument();
	});

	it("registers one component through the operator form", async () => {
		globalThis.fetch = vi.fn(async (input: string | URL | Request) => {
			const url = String(input);
			if (url === "/api/health") {
				return new Response("ok", { status: 200 });
			}
			if (url === "/api/components") {
				return new Response(
					JSON.stringify({
						change: "registered",
						managed_components: 1,
					}),
					{ status: 200, headers: { "Content-Type": "application/json" } },
				);
			}
			return new Response(null, { status: 404 });
		}) as typeof fetch;

		render(
			<QueryClientProvider client={new QueryClient()}>
				<OperationsPage />
			</QueryClientProvider>,
		);

		fireEvent.click(screen.getByRole("button", { name: "Register" }));

		expect(await screen.findByText(/Change: registered/i)).toBeInTheDocument();
	});

	it("creates one collection and adds one managed component", async () => {
		globalThis.fetch = vi.fn(async (input: string | URL | Request) => {
			const url = String(input);
			if (url === "/api/health") {
				return new Response("ok", { status: 200 });
			}
			if (url === "/api/collections") {
				return new Response(
					JSON.stringify({
						managed_collections: 1,
						collections: [
							{
								collection_key: "release:2026.05",
								name: "May Release",
								members: 1,
								scan_schedule: null,
								due_now: false,
							},
						],
						change: "created",
					}),
					{ status: 200, headers: { "Content-Type": "application/json" } },
				);
			}
			if (url === "/api/collections/release%3A2026.05/components") {
				return new Response(
					JSON.stringify({
						change: "added",
						members: 1,
					}),
					{ status: 200, headers: { "Content-Type": "application/json" } },
				);
			}
			if (url === "/api/collections/release%3A2026.05") {
				return new Response(
					JSON.stringify({
						collection_key: "release:2026.05",
						name: "May Release",
						scan_schedule: null,
						members: [{ component_key: "component:payments-api" }],
					}),
					{ status: 200, headers: { "Content-Type": "application/json" } },
				);
			}
			return new Response(null, { status: 404 });
		}) as typeof fetch;

		render(
			<QueryClientProvider client={new QueryClient()}>
				<OperationsPage />
			</QueryClientProvider>,
		);

		fireEvent.click(screen.getByRole("button", { name: "Create Collection" }));
		fireEvent.click(screen.getByRole("button", { name: "Add Component" }));

		expect(
			await screen.findByText(/Managed collections: 1\./i),
		).toBeInTheDocument();
		expect(
			await screen.findByText(/Change: added\. Members: 1\./i),
		).toBeInTheDocument();
		expect(
			await screen.findByText(/component:payments-api/i),
		).toBeInTheDocument();
		expect(
			await screen.findByText(/Total: 1\. Scheduled: 0\. Due now: 0\./i),
		).toBeInTheDocument();
	});

	it("configures one collection scan schedule and runs the scheduler", async () => {
		globalThis.fetch = vi.fn(async (input: string | URL | Request) => {
			const url = String(input);
			if (url === "/api/health") {
				return new Response("ok", { status: 200 });
			}
			if (url === "/api/collections/release%3A2026.05/scan-schedule") {
				return new Response(
					JSON.stringify({
						change: "configured",
						collection_key: "release:2026.05",
						cadence_minutes: 60,
						freshness: "deterministic",
						next_due_at_unix_ms: 1000,
					}),
					{ status: 200, headers: { "Content-Type": "application/json" } },
				);
			}
			if (url === "/api/collections") {
				return new Response(
					JSON.stringify({
						managed_collections: 1,
						collections: [
							{
								collection_key: "release:2026.05",
								name: "May Release",
								members: 1,
								scan_schedule: {
									cadence_minutes: 60,
									freshness: "deterministic",
									next_due_at_unix_ms: 1000,
									last_materialized_at_unix_ms: 1500,
									last_enqueued_commands: 1,
								},
								due_now: false,
							},
						],
					}),
					{ status: 200, headers: { "Content-Type": "application/json" } },
				);
			}
			if (url === "/api/collections/release%3A2026.05") {
				return new Response(
					JSON.stringify({
						collection_key: "release:2026.05",
						name: "May Release",
						scan_schedule: {
							cadence_minutes: 60,
							freshness: "deterministic",
							next_due_at_unix_ms: 1000,
							last_materialized_at_unix_ms: 1500,
							last_enqueued_commands: 1,
						},
						members: [{ component_key: "component:payments-api" }],
					}),
					{ status: 200, headers: { "Content-Type": "application/json" } },
				);
			}
			if (url === "/api/collection-scan-workers/drain") {
				return new Response(
					JSON.stringify({
						outcome: "drained",
						processed_collections: 1,
						enqueued_commands: 1,
						pending_due_remaining: 0,
						last_collection_key: "release:2026.05",
					}),
					{ status: 200, headers: { "Content-Type": "application/json" } },
				);
			}
			return new Response(null, { status: 404 });
		}) as typeof fetch;

		render(
			<QueryClientProvider client={new QueryClient()}>
				<OperationsPage />
			</QueryClientProvider>,
		);

		fireEvent.click(
			screen.getByRole("button", { name: "Configure Collection Schedule" }),
		);
		fireEvent.click(
			screen.getByRole("button", { name: "Run Collection Scheduler" }),
		);

		expect(
			await screen.findByText(/Cadence: 60 minutes\./i),
		).toBeInTheDocument();
		expect(
			await screen.findByText(
				/Processed collections: 1\. Enqueued commands: 1\./i,
			),
		).toBeInTheDocument();
		expect(
			await screen.findByText(/Scheduled: 1\. Due now: 0\./i),
		).toBeInTheDocument();
		expect(
			await screen.findByText(
				/due later - every 60 minutes \(deterministic\) - last run 1500 - last enqueued 1/i,
			),
		).toBeInTheDocument();
		expect(
			await screen.findByText(
				/Last run at 1500\. Last enqueued commands: 1\./i,
			),
		).toBeInTheDocument();
	});

	it("requests one canonical scan from the operator flow", async () => {
		globalThis.fetch = vi.fn(async (input: string | URL | Request) => {
			const url = String(input);
			if (url === "/api/health") {
				return new Response("ok", { status: 200 });
			}
			if (url === "/api/scan-requests") {
				return new Response(
					JSON.stringify({
						command_id: "cmd-1",
						status: "pending",
						component_key: "component:payments-api",
						artifact_kind: "container-image",
						artifact_identity: "registry.example/payments@sha256:111",
						freshness: "deterministic",
					}),
					{ status: 200, headers: { "Content-Type": "application/json" } },
				);
			}
			return new Response(null, { status: 404 });
		}) as typeof fetch;

		render(
			<QueryClientProvider client={new QueryClient()}>
				<OperationsPage />
			</QueryClientProvider>,
		);

		fireEvent.click(screen.getByRole("button", { name: "Request Scan" }));

		expect(await screen.findByText(/Command: cmd-1/i)).toBeInTheDocument();
	});

	it("requests one canonical collection scan from the operator flow", async () => {
		globalThis.fetch = vi.fn(async (input: string | URL | Request) => {
			const url = String(input);
			if (url === "/api/health") {
				return new Response("ok", { status: 200 });
			}
			if (url === "/api/collections/release%3A2026.05/scan-requests") {
				return new Response(
					JSON.stringify({
						collection_key: "release:2026.05",
						freshness: "deterministic",
						enqueued: 1,
						command_ids: ["cmd-2"],
					}),
					{ status: 200, headers: { "Content-Type": "application/json" } },
				);
			}
			return new Response(null, { status: 404 });
		}) as typeof fetch;

		render(
			<QueryClientProvider client={new QueryClient()}>
				<OperationsPage />
			</QueryClientProvider>,
		);

		fireEvent.click(
			screen.getByRole("button", { name: "Request Collection Scan" }),
		);

		expect(
			await screen.findByText(/Collection: release:2026.05\. Enqueued: 1\./i),
		).toBeInTheDocument();
	});

	it("refreshes one command status and runs the fixture worker", async () => {
		globalThis.fetch = vi.fn(async (input: string | URL | Request) => {
			const url = String(input);
			if (url === "/api/health") {
				return new Response("ok", { status: 200 });
			}
			if (url === "/api/scan-commands/cmd-1") {
				return new Response(
					JSON.stringify({
						command_id: "cmd-1",
						status: "pending",
					}),
					{ status: 200, headers: { "Content-Type": "application/json" } },
				);
			}
			if (url === "/api/scan-workers/drain") {
				return new Response(
					JSON.stringify({
						outcome: "completed",
						processed: 1,
						completed: 1,
						failed: 0,
						pending_remaining: 0,
						last_command_id: "cmd-1",
						last_command_status: "completed",
						last_error_code: null,
						last_retryable: null,
					}),
					{ status: 200, headers: { "Content-Type": "application/json" } },
				);
			}
			return new Response(null, { status: 404 });
		}) as typeof fetch;

		render(
			<QueryClientProvider client={new QueryClient()}>
				<OperationsPage />
			</QueryClientProvider>,
		);

		fireEvent.change(screen.getByRole("textbox", { name: "Command id" }), {
			target: { value: "cmd-1" },
		});
		fireEvent.click(screen.getByRole("button", { name: "Refresh Status" }));
		fireEvent.click(screen.getByRole("button", { name: "Run Worker" }));

		expect(await screen.findByText(/Status: pending/i)).toBeInTheDocument();
		expect(await screen.findByText(/Outcome: completed/i)).toBeInTheDocument();
	});
});
