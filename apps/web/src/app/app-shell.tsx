import type { PropsWithChildren } from "react";

type AppShellProps = PropsWithChildren<{
	statusLabel: string;
}>;

export function AppShell({ statusLabel, children }: AppShellProps) {
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
						<dt>Status</dt>
						<dd>{statusLabel}</dd>
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
