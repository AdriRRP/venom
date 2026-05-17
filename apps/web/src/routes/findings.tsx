import { useQuery } from "@tanstack/react-query";
import {
	type ColumnDef,
	flexRender,
	getCoreRowModel,
	useReactTable,
} from "@tanstack/react-table";
import { useMemo, useState } from "react";
import { AppShell } from "../app/app-shell";
import {
	type ActiveFinding,
	fetchActiveFindings,
	fetchApiHealth,
} from "../lib/api";

const defaultRequest = {
	componentKey: "component:payments-api",
	artifactKind: "container-image",
	artifactIdentity: "registry.example/payments@sha256:111",
	minSeverity: "all",
	packageName: "",
	limit: 50,
	offset: 0,
};

const columns: ColumnDef<ActiveFinding>[] = [
	{
		header: "Severity",
		accessorKey: "severity",
	},
	{
		header: "Vulnerability",
		accessorKey: "vulnerability_id",
	},
	{
		header: "Package",
		cell: ({ row }) =>
			`${row.original.package_name}@${row.original.package_version}`,
	},
];

export function FindingsPage() {
	const [request, setRequest] = useState(defaultRequest);

	const healthQuery = useQuery({
		queryKey: ["api-health"],
		queryFn: fetchApiHealth,
		refetchInterval: 15_000,
	});

	const findingsQuery = useQuery({
		queryKey: ["active-findings", request],
		queryFn: () => fetchActiveFindings(request),
	});

	const table = useReactTable({
		data: findingsQuery.data?.active_findings ?? [],
		columns,
		getCoreRowModel: getCoreRowModel(),
	});

	const healthLabel = useMemo(() => {
		if (healthQuery.isLoading) {
			return "loading";
		}
		return healthQuery.data === "healthy" ? "healthy" : "unhealthy";
	}, [healthQuery.data, healthQuery.isLoading]);

	const findingsWindowLabel = useMemo(() => {
		const total = findingsQuery.data?.total_active_findings ?? 0;
		const returned = findingsQuery.data?.returned ?? 0;
		const offset = findingsQuery.data?.offset ?? request.offset;

		if (total === 0 || returned === 0) {
			return "Showing 0 of 0";
		}

		const start = offset + 1;
		const end = offset + returned;
		return `Showing ${start}-${end} of ${total}`;
	}, [findingsQuery.data, request.offset]);

	const canGoPrevious = request.offset > 0 && !findingsQuery.isFetching;
	const canGoNext =
		(findingsQuery.data?.offset ?? request.offset) +
			(findingsQuery.data?.returned ?? 0) <
			(findingsQuery.data?.total_active_findings ?? 0) &&
		!findingsQuery.isFetching;

	return (
		<AppShell apiHealth={healthLabel} currentView="findings">
			<section className="panel">
				<div className="panel-header">
					<div>
						<p className="eyebrow">Operations</p>
						<h2>Active Findings</h2>
					</div>
					<button
						className="secondary-button"
						onClick={() => {
							void findingsQuery.refetch();
							void healthQuery.refetch();
						}}
						type="button"
					>
						Refresh
					</button>
				</div>

				<form
					className="filters"
					onSubmit={(event) => {
						event.preventDefault();
						const formData = new FormData(event.currentTarget);
						setRequest({
							componentKey: String(formData.get("componentKey") ?? ""),
							artifactKind: String(
								formData.get("artifactKind") ?? "container-image",
							),
							artifactIdentity: String(formData.get("artifactIdentity") ?? ""),
							minSeverity: String(formData.get("minSeverity") ?? "all"),
							packageName: String(formData.get("packageName") ?? ""),
							limit: Number(formData.get("limit") ?? 50),
							offset: 0,
						});
					}}
				>
					<label>
						Component
						<input defaultValue={request.componentKey} name="componentKey" />
					</label>
					<label>
						Artifact kind
						<select defaultValue={request.artifactKind} name="artifactKind">
							<option value="container-image">container-image</option>
							<option value="sbom-document">sbom-document</option>
						</select>
					</label>
					<label>
						Artifact identity
						<input
							defaultValue={request.artifactIdentity}
							name="artifactIdentity"
						/>
					</label>
					<label>
						Package name
						<input defaultValue={request.packageName} name="packageName" />
					</label>
					<label>
						Minimum severity
						<select defaultValue={request.minSeverity} name="minSeverity">
							<option value="all">all</option>
							<option value="low">low</option>
							<option value="medium">medium</option>
							<option value="high">high</option>
							<option value="critical">critical</option>
						</select>
					</label>
					<label>
						Limit
						<input
							defaultValue={request.limit}
							min={1}
							name="limit"
							type="number"
						/>
					</label>
					<button className="primary-button" type="submit">
						Query
					</button>
				</form>

				{findingsQuery.isError ? (
					<p className="error-banner">
						{findingsQuery.error instanceof Error
							? findingsQuery.error.message
							: "failed to load active findings"}
					</p>
				) : null}

				<div className="results-meta">
					<span>Total: {findingsQuery.data?.total_active_findings ?? 0}</span>
					<span>Returned: {findingsQuery.data?.returned ?? 0}</span>
					<span>Offset: {findingsQuery.data?.offset ?? 0}</span>
					<span>Limit: {findingsQuery.data?.limit ?? request.limit}</span>
					<span>{findingsWindowLabel}</span>
				</div>

				<div className="results-meta">
					<button
						className="secondary-button"
						disabled={!canGoPrevious}
						onClick={() => {
							setRequest((current) => ({
								...current,
								offset: Math.max(0, current.offset - current.limit),
							}));
						}}
						type="button"
					>
						Previous Page
					</button>
					<button
						className="secondary-button"
						disabled={!canGoNext}
						onClick={() => {
							setRequest((current) => ({
								...current,
								offset: current.offset + current.limit,
							}));
						}}
						type="button"
					>
						Next Page
					</button>
				</div>

				<div className="table-wrap">
					<table>
						<thead>
							{table.getHeaderGroups().map((headerGroup) => (
								<tr key={headerGroup.id}>
									{headerGroup.headers.map((header) => (
										<th key={header.id}>
											{header.isPlaceholder
												? null
												: flexRender(
														header.column.columnDef.header,
														header.getContext(),
													)}
										</th>
									))}
								</tr>
							))}
						</thead>
						<tbody>
							{table.getRowModel().rows.length === 0 ? (
								<tr>
									<td colSpan={columns.length}>No active findings yet.</td>
								</tr>
							) : (
								table.getRowModel().rows.map((row) => (
									<tr key={row.id}>
										{row.getVisibleCells().map((cell) => (
											<td key={cell.id}>
												{flexRender(
													cell.column.columnDef.cell,
													cell.getContext(),
												)}
											</td>
										))}
									</tr>
								))
							)}
						</tbody>
					</table>
				</div>
			</section>
		</AppShell>
	);
}
