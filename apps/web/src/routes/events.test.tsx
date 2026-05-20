import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import type { ReactNode } from "react";
import { EventsPage } from "./events";

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

describe("EventsPage", () => {
	it("renders one operator-facing system event timeline", async () => {
		globalThis.fetch = vi.fn(async (input: string | URL | Request) => {
			const url = String(input);
			if (url.includes("/health")) {
				return new Response("ok", { status: 200 });
			}
			if (url.includes("/system-events")) {
				return new Response(
					JSON.stringify({
						category: "command",
						total: 1,
						returned: 1,
						limit: 50,
						events: [
							{
								event_id: "event-1",
								occurred_at_unix_ms: 1000,
								category: "command",
								kind: "scan-command-completed",
								collection_key: "release:2026.05",
								component_key: "component:payments-api",
								command_id: "scan-command-1",
								integration_event_id: null,
								finding_count: 1,
								retryable: null,
								detail: "discovered 1, repeated 0, withdrawn 0, active 1",
							},
						],
					}),
					{ status: 200, headers: { "Content-Type": "application/json" } },
				);
			}
			throw new Error(`unexpected fetch: ${url}`);
		}) as typeof fetch;

		render(
			<QueryClientProvider client={new QueryClient()}>
				<EventsPage />
			</QueryClientProvider>,
		);

		expect(
			await screen.findByRole("heading", {
				level: 2,
				name: "System Event Trace",
			}),
		).toBeInTheDocument();
		expect(
			await screen.findByRole("cell", { name: "scan-command-completed" }),
		).toBeInTheDocument();
		expect(
			await screen.findByRole("cell", { name: "release:2026.05" }),
		).toBeInTheDocument();
	});
});
