import type { PropsWithChildren } from "react";

export type ApiHealthState = "healthy" | "unhealthy" | "loading";

type AppShellProps = PropsWithChildren<{
	apiHealth: ApiHealthState;
}>;

export function AppShell({ apiHealth, children }: AppShellProps) {
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
