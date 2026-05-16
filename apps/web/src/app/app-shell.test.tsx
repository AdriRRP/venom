import { render, screen } from "@testing-library/react";
import { AppShell } from "./app-shell";

describe("AppShell", () => {
	it("renders a visible shell status badge", () => {
		render(
			<AppShell statusLabel="Pending wiring">
				<p>content</p>
			</AppShell>,
		);

		expect(screen.getByText("Operator Console")).toBeInTheDocument();
		expect(screen.getByText("Pending wiring")).toBeInTheDocument();
		expect(screen.getByText("content")).toBeInTheDocument();
	});
});
