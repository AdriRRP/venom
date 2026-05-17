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
