import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import type { ReactNode } from "react";
import { DashboardPage } from "./dashboard";

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

describe("DashboardPage", () => {
	it("renders one executive release view over managed collections", async () => {
		globalThis.fetch = vi.fn(async (input: string | URL | Request) => {
			const url = String(input);
			if (url.includes("/health")) {
				return new Response("ok", { status: 200 });
			}
			if (url.includes("/dashboard/releases")) {
				return new Response(
					JSON.stringify({
						summary: {
							managed_collections: 2,
							scheduled_collections: 1,
							due_now_collections: 1,
							total_active_findings: 2,
							open_findings: 1,
							risk_accepted_findings: 0,
							suppressed_findings: 1,
							critical_risk_findings: 1,
							high_risk_findings: 1,
						},
						collections: [
							{
								collection_key: "release:2026.05",
								name: "May Release",
								members: 1,
								due_now: true,
								scan_schedule: {
									cadence_minutes: 60,
									freshness: "deterministic",
									next_due_at_unix_ms: 1000,
									last_materialized_at_unix_ms: 900,
									last_enqueued_commands: 1,
								},
								health: {
									total: 2,
									open: 1,
									risk_accepted: 0,
									suppressed: 1,
									critical_risk: 1,
									high_risk: 1,
								},
							},
							{
								collection_key: "release:2026.06",
								name: "June Release",
								members: 1,
								due_now: false,
								scan_schedule: null,
								health: {
									total: 0,
									open: 0,
									risk_accepted: 0,
									suppressed: 0,
									critical_risk: 0,
									high_risk: 0,
								},
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
				<DashboardPage />
			</QueryClientProvider>,
		);

		expect(
			await screen.findByRole("heading", {
				level: 2,
				name: "Release Dashboard",
			}),
		).toBeInTheDocument();
		expect(await screen.findByText("Collections")).toBeInTheDocument();
		expect(
			await screen.findByText(/1 scheduled,\s*1 due now/i),
		).toBeInTheDocument();
		expect(await screen.findByText("May Release")).toBeInTheDocument();
		expect(await screen.findByText("release:2026.05")).toBeInTheDocument();
		expect(await screen.findByText("Due now")).toBeInTheDocument();
		expect(
			await screen.findByText(
				"Health: 2 active - 1 open - 0 risk accepted - 1 suppressed - 1 critical risk - 1 high risk",
			),
		).toBeInTheDocument();
	});
});
