import { Link } from "@tanstack/react-router";
import type { PropsWithChildren } from "react";

export type ApiHealthState = "healthy" | "unhealthy" | "loading";
export type AppShellView = "dashboard" | "findings" | "operations";

type AppShellProps = PropsWithChildren<{
	apiHealth: ApiHealthState;
	currentView: AppShellView;
}>;

export function AppShell({ apiHealth, currentView, children }: AppShellProps) {
	return (
		<div className="shell">
			<aside className="sidebar">
				<div>
					<p className="eyebrow">VENOM</p>
					<h1>Operator Console</h1>
					<p className="copy">
						Thin operator-facing UI over the canonical Rust API.
					</p>
				</div>
				<nav aria-label="Primary" className="sidebar-nav">
					<Link
						className={
							currentView === "dashboard" ? "nav-link active" : "nav-link"
						}
						to="/dashboard"
					>
						Release Dashboard
					</Link>
					<Link
						className={
							currentView === "findings" ? "nav-link active" : "nav-link"
						}
						to="/findings"
					>
						Active Findings
					</Link>
					<Link
						className={
							currentView === "operations" ? "nav-link active" : "nav-link"
						}
						to="/operations"
					>
						Operations
					</Link>
				</nav>
				<dl className="status-card">
					<div>
						<dt>API Health</dt>
						<dd data-health={apiHealth}>{healthLabel(apiHealth)}</dd>
					</div>
					<div>
						<dt>Scope</dt>
						<dd>Operator shell</dd>
					</div>
				</dl>
			</aside>
			<main className="content">{children}</main>
		</div>
	);
}

function healthLabel(value: ApiHealthState): string {
	switch (value) {
		case "healthy":
			return "Healthy";
		case "unhealthy":
			return "Unhealthy";
		case "loading":
			return "Checking";
	}
}
