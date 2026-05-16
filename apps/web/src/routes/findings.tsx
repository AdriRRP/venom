import { useQuery } from "@tanstack/react-query";
import { useMemo } from "react";
import { AppShell } from "../app/app-shell";
import { fetchApiHealth } from "../lib/api";

export function FindingsPage() {
	const healthQuery = useQuery({
		queryKey: ["api-health"],
		queryFn: fetchApiHealth,
		refetchInterval: 15_000,
	});

	const healthLabel = useMemo(() => {
		if (healthQuery.isLoading) {
			return "loading";
		}
		return healthQuery.data === "healthy" ? "healthy" : "unhealthy";
	}, [healthQuery.data, healthQuery.isLoading]);

	return (
		<AppShell apiHealth={healthLabel}>
			<section className="panel">
				<div className="panel-header">
					<div>
						<p className="eyebrow">Operations</p>
						<h2>Operator Shell</h2>
					</div>
					<button
						className="secondary-button"
						onClick={() => {
							void healthQuery.refetch();
						}}
						type="button"
					>
						Refresh
					</button>
				</div>
				<p className="copy">
					The operator console is now wired to the Rust API and ready for the
					first active findings workflow.
				</p>
			</section>
		</AppShell>
	);
}
