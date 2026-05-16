import { render, screen } from "@testing-library/react";
import { AppShell } from "./app-shell";

describe("AppShell", () => {
	it("renders a visible health status badge", () => {
		render(
			<AppShell apiHealth="healthy">
				<p>content</p>
			</AppShell>,
		);

		expect(screen.getByText("Operator Console")).toBeInTheDocument();
		expect(screen.getByText("Healthy")).toBeInTheDocument();
		expect(screen.getByText("content")).toBeInTheDocument();
	});
});
