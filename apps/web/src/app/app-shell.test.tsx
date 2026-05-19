import { render, screen } from "@testing-library/react";
import type { ReactNode } from "react";
import { AppShell } from "./app-shell";

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

describe("AppShell", () => {
	it("renders a visible health status badge", () => {
		render(
			<AppShell apiHealth="healthy" currentView="dashboard">
				<p>content</p>
			</AppShell>,
		);

		expect(screen.getByText("Operator Console")).toBeInTheDocument();
		expect(screen.getByText("Healthy")).toBeInTheDocument();
		expect(
			screen.getByRole("link", { name: "Release Dashboard" }),
		).toBeInTheDocument();
		expect(
			screen.getByRole("link", { name: "Active Findings" }),
		).toBeInTheDocument();
		expect(
			screen.getByRole("link", { name: "Operations" }),
		).toBeInTheDocument();
		expect(screen.getByText("content")).toBeInTheDocument();
	});
});
