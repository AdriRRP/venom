import { useQuery } from "@tanstack/react-query";
import { useMemo } from "react";
import { AppShell } from "../app/app-shell";
import { fetchApiHealth } from "../lib/api";

export function OperationsPage() {
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
		<AppShell apiHealth={healthLabel} currentView="operations">
			<section className="panel">
				<div className="panel-header">
					<div>
						<p className="eyebrow">Operations</p>
						<h2>Managed Components</h2>
					</div>
				</div>
				<p className="copy">
					Register components, bind immutable artifacts, configure one provider,
					and request canonical scans from the operator console.
				</p>
			</section>
		</AppShell>
	);
}
