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

		expect(
			screen.getByRole("heading", { level: 2, name: "Active Findings" }),
		).toBeInTheDocument();
		expect(screen.getByRole("button", { name: "Query" })).toBeInTheDocument();
		expect(screen.getByText("No active findings yet.")).toBeInTheDocument();
	});

	it("submits the optional package-name filter to the findings query", async () => {
		const calls: string[] = [];

		globalThis.fetch = vi.fn(async (input: string | URL | Request) => {
			const url = String(input);
			calls.push(url);
			if (url.includes("/health")) {
				return new Response("ok", { status: 200 });
			}
			return new Response(
				JSON.stringify({
					component_key: "component:payments-api",
					artifact_kind: "container-image",
					artifact_identity: "registry.example/payments@sha256:111",
					min_severity: "high",
					package_name: "openssl",
					total_active_findings: 1,
					returned: 1,
					offset: 0,
					limit: 50,
					active_findings: [
						{
							vulnerability_id: "CVE-2026-0001",
							package_name: "openssl",
							package_version: "3.0.0",
							severity: "high",
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

		fireEvent.change(screen.getByRole("textbox", { name: "Package name" }), {
			target: { value: "openssl" },
		});
		fireEvent.click(screen.getByRole("button", { name: "Query" }));

		expect(await screen.findByText("Showing 1-1 of 1")).toBeInTheDocument();
		expect(calls.some((call) => call.includes("package_name=openssl"))).toBe(
			true,
		);
	});

	it("moves between pages with bounded next and previous controls", async () => {
		globalThis.fetch = vi.fn(async (input: string | URL | Request) => {
			const url = String(input);
			if (url.includes("/health")) {
				return new Response("ok", { status: 200 });
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
								vulnerability_id: "CVE-2026-0002",
								package_name: "zlib",
								package_version: "1.3.1",
								severity: "medium",
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
							vulnerability_id: "CVE-2026-0001",
							package_name: "openssl",
							package_version: "3.0.0",
							severity: "high",
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

		fireEvent.change(screen.getByRole("spinbutton", { name: "Limit" }), {
			target: { value: "1" },
		});
		fireEvent.click(screen.getByRole("button", { name: "Query" }));
		expect(await screen.findByText("Showing 1-1 of 2")).toBeInTheDocument();

		fireEvent.click(screen.getByRole("button", { name: "Next Page" }));
		expect(await screen.findByText("Showing 2-2 of 2")).toBeInTheDocument();
		expect(await screen.findByText("zlib@1.3.1")).toBeInTheDocument();

		fireEvent.click(screen.getByRole("button", { name: "Previous Page" }));
		expect(await screen.findByText("openssl@3.0.0")).toBeInTheDocument();
	});
});
