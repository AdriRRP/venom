import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { AppShell } from "../app/app-shell";
import { fetchApiHealth, fetchSystemEvents } from "../lib/api";

export function EventsPage() {
	const [category, setCategory] = useState("all");

	const healthQuery = useQuery({
		queryKey: ["api-health"],
		queryFn: fetchApiHealth,
		refetchInterval: 15_000,
	});

	const eventsQuery = useQuery({
		queryKey: ["system-events", category],
		queryFn: () => fetchSystemEvents({ category, limit: 50 }),
		refetchInterval: 15_000,
	});

	const apiHealth = healthQuery.isLoading
		? "loading"
		: (healthQuery.data ?? "unhealthy");

	const events = eventsQuery.data?.events ?? [];

	return (
		<AppShell apiHealth={apiHealth} currentView="events">
			<div className="panel-stack">
				<section className="panel">
					<div className="panel-header">
						<div>
							<p className="eyebrow">Observability</p>
							<h2>System Event Trace</h2>
						</div>
					</div>
					<p className="copy">
						One recent operator-facing timeline over scheduler, command,
						governance, and publication activity.
					</p>
				</section>

				<section className="panel">
					<div className="panel-header">
						<div>
							<p className="eyebrow">Filters</p>
							<h2>Recent Events</h2>
						</div>
					</div>
					<label className="field">
						<span>Category</span>
						<select
							value={category}
							onChange={(event) => setCategory(event.target.value)}
						>
							<option value="all">All</option>
							<option value="scheduler">Scheduler</option>
							<option value="command">Command</option>
							<option value="governance">Governance</option>
							<option value="publication">Publication</option>
						</select>
					</label>

					{eventsQuery.isLoading ? (
						<p>Loading system events...</p>
					) : events.length === 0 ? (
						<p>No recent system events yet.</p>
					) : (
						<div className="table-shell">
							<table>
								<thead>
									<tr>
										<th>Time</th>
										<th>Category</th>
										<th>Kind</th>
										<th>Collection</th>
										<th>Component</th>
										<th>Command</th>
										<th>Detail</th>
									</tr>
								</thead>
								<tbody>
									{events.map((event) => (
										<tr key={event.event_id}>
											<td>{event.occurred_at_unix_ms}</td>
											<td>{event.category}</td>
											<td>{event.kind}</td>
											<td>{event.collection_key ?? "n/a"}</td>
											<td>{event.component_key ?? "n/a"}</td>
											<td>{event.command_id ?? "n/a"}</td>
											<td>{describeSystemEvent(event)}</td>
										</tr>
									))}
								</tbody>
							</table>
						</div>
					)}
				</section>
			</div>
		</AppShell>
	);
}

function describeSystemEvent(event: {
	finding_count: number | null;
	retryable: boolean | null;
	detail: string | null;
	integration_event_id: string | null;
}) {
	const parts = [];
	if (event.finding_count !== null) {
		parts.push(`${event.finding_count} findings`);
	}
	if (event.retryable !== null) {
		parts.push(event.retryable ? "retryable" : "terminal");
	}
	if (event.integration_event_id) {
		parts.push(event.integration_event_id);
	}
	if (event.detail) {
		parts.push(event.detail);
	}
	return parts.length === 0 ? "recent activity" : parts.join(" - ");
}
