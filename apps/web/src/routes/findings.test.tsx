import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import { FindingsPage } from "./findings";

describe("FindingsPage", () => {
	it("renders the first operator screen shape", () => {
		globalThis.fetch = vi.fn(async (input: string | URL | Request) => {
			const url = String(input);
			if (url.includes("/health")) {
				return new Response("ok", { status: 200 });
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

		expect(screen.getByText("Active Findings")).toBeInTheDocument();
		expect(screen.getByRole("button", { name: "Query" })).toBeInTheDocument();
		expect(screen.getByText("No active findings yet.")).toBeInTheDocument();
	});
});
