import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
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
});
