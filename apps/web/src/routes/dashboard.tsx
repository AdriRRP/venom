import { useQuery } from "@tanstack/react-query";
import { AppShell } from "../app/app-shell";
import {
	fetchApiHealth,
	fetchReleaseDashboard,
	type ReleaseDashboardCollection,
} from "../lib/api";
import { describeCollectionHealth } from "../lib/collection-health";

function describeSchedule(collection: ReleaseDashboardCollection) {
	if (collection.scan_schedule === null) {
		return "manual only";
	}

	const cadence = `every ${collection.scan_schedule.cadence_minutes} minutes`;
	const due = collection.due_now ? "due now" : "due later";
	const lastRun =
		collection.scan_schedule.last_materialized_at_unix_ms === null
			? "last run never"
			: `last run ${collection.scan_schedule.last_materialized_at_unix_ms}`;

	return `${due} - ${cadence} (${collection.scan_schedule.freshness}) - ${lastRun}`;
}

export function DashboardPage() {
	const healthQuery = useQuery({
		queryKey: ["api-health"],
		queryFn: fetchApiHealth,
		refetchInterval: 15_000,
	});

	const dashboardQuery = useQuery({
		queryKey: ["release-dashboard"],
		queryFn: fetchReleaseDashboard,
		refetchInterval: 15_000,
	});

	const healthLabel = healthQuery.isLoading
		? "loading"
		: (healthQuery.data ?? "unhealthy");

	const summary = dashboardQuery.data?.summary;
	const collections = dashboardQuery.data?.collections ?? [];

	return (
		<AppShell apiHealth={healthLabel} currentView="dashboard">
			<div className="panel-stack">
				<section className="panel">
					<div className="panel-header">
						<div>
							<p className="eyebrow">Dashboard</p>
							<h2>Release Dashboard</h2>
						</div>
					</div>
					<p className="copy">
						One executive view over managed releases, scheduled work, active
						risk, and current governance posture.
					</p>
				</section>

				<section className="panel">
					<div className="panel-header">
						<div>
							<p className="eyebrow">Summary</p>
							<h2>Release Health Overview</h2>
						</div>
					</div>
					<div className="summary-grid">
						<div className="metric-card">
							<span className="metric-label">Collections</span>
							<strong>{summary?.managed_collections ?? 0}</strong>
							<p>
								{summary?.scheduled_collections ?? 0} scheduled,{" "}
								{summary?.due_now_collections ?? 0} due now
							</p>
						</div>
						<div className="metric-card">
							<span className="metric-label">Findings</span>
							<strong>{summary?.total_active_findings ?? 0}</strong>
							<p>
								{summary?.open_findings ?? 0} open,{" "}
								{summary?.suppressed_findings ?? 0} suppressed,{" "}
								{summary?.risk_accepted_findings ?? 0} risk accepted
							</p>
						</div>
						<div className="metric-card">
							<span className="metric-label">Contextual risk</span>
							<strong>{summary?.critical_risk_findings ?? 0}</strong>
							<p>{summary?.high_risk_findings ?? 0} high risk findings</p>
						</div>
					</div>
				</section>

				<section className="panel">
					<div className="panel-header">
						<div>
							<p className="eyebrow">Releases</p>
							<h2>Managed Collections</h2>
						</div>
					</div>
					{dashboardQuery.isLoading ? (
						<p>Loading release dashboard...</p>
					) : collections.length === 0 ? (
						<p>No managed collections yet.</p>
					) : (
						<div className="dashboard-grid">
							{collections.map((collection) => (
								<article
									className="dashboard-card"
									key={collection.collection_key}
								>
									<div className="dashboard-card-header">
										<div>
											<p className="eyebrow">Release</p>
											<h3>{collection.name}</h3>
										</div>
										<span
											className={
												collection.due_now
													? "status-pill due-now"
													: "status-pill"
											}
										>
											{collection.due_now ? "Due now" : "Stable"}
										</span>
									</div>
									<p className="mono-line">{collection.collection_key}</p>
									<p>{collection.members} members</p>
									<p>{describeSchedule(collection)}</p>
									<p>Health: {describeCollectionHealth(collection.health)}</p>
								</article>
							))}
						</div>
					)}
				</section>
			</div>
		</AppShell>
	);
}
