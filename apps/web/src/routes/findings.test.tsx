import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import { FindingsPage } from "./findings";

describe("FindingsPage", () => {
	it("renders the operator shell with API health wiring", () => {
		globalThis.fetch = vi.fn(
			async () => new Response("ok", { status: 200 }),
		) as typeof fetch;

		render(
			<QueryClientProvider client={new QueryClient()}>
				<FindingsPage />
			</QueryClientProvider>,
		);

		expect(screen.getByText("Operator Shell")).toBeInTheDocument();
		expect(screen.getByRole("button", { name: "Refresh" })).toBeInTheDocument();
		expect(
			screen.getByText(/ready for the first active findings workflow/i),
		).toBeInTheDocument();
	});
});
