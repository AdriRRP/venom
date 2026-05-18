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
	type CollectionActiveFinding,
	fetchActiveFindings,
	fetchApiHealth,
	fetchCollectionActiveFindings,
} from "../lib/api";

const defaultCollectionRequest = {
	collectionKey: "release:2026.05",
	minSeverity: "all",
	packageName: "",
	limit: 50,
	offset: 0,
};

const defaultArtifactRequest = {
	componentKey: "component:payments-api",
	artifactKind: "container-image",
	artifactIdentity: "registry.example/payments@sha256:111",
	minSeverity: "all",
	packageName: "",
	limit: 50,
	offset: 0,
};

const collectionColumns: ColumnDef<CollectionActiveFinding>[] = [
	{
		header: "Component",
		accessorKey: "component_key",
	},
	{
		header: "Artifact",
		cell: ({ row }) =>
			`${row.original.artifact_kind}:${row.original.artifact_identity}`,
	},
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

const artifactColumns: ColumnDef<ActiveFinding>[] = [
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

function findingsWindowLabel(total: number, returned: number, offset: number) {
	if (total === 0 || returned === 0) {
		return "Showing 0 of 0";
	}

	const start = offset + 1;
	const end = offset + returned;
	return `Showing ${start}-${end} of ${total}`;
}

export function FindingsPage() {
	const [collectionRequest, setCollectionRequest] = useState(
		defaultCollectionRequest,
	);
	const [artifactRequest, setArtifactRequest] = useState(
		defaultArtifactRequest,
	);

	const healthQuery = useQuery({
		queryKey: ["api-health"],
		queryFn: fetchApiHealth,
		refetchInterval: 15_000,
	});

	const collectionFindingsQuery = useQuery({
		queryKey: ["collection-active-findings", collectionRequest],
		queryFn: () => fetchCollectionActiveFindings(collectionRequest),
	});

	const artifactFindingsQuery = useQuery({
		queryKey: ["active-findings", artifactRequest],
		queryFn: () => fetchActiveFindings(artifactRequest),
	});

	const collectionTable = useReactTable({
		data: collectionFindingsQuery.data?.active_findings ?? [],
		columns: collectionColumns,
		getCoreRowModel: getCoreRowModel(),
	});

	const artifactTable = useReactTable({
		data: artifactFindingsQuery.data?.active_findings ?? [],
		columns: artifactColumns,
		getCoreRowModel: getCoreRowModel(),
	});

	const healthLabel = useMemo(() => {
		if (healthQuery.isLoading) {
			return "loading";
		}
		return healthQuery.data === "healthy" ? "healthy" : "unhealthy";
	}, [healthQuery.data, healthQuery.isLoading]);

	const collectionWindow = useMemo(
		() =>
			findingsWindowLabel(
				collectionFindingsQuery.data?.total_active_findings ?? 0,
				collectionFindingsQuery.data?.returned ?? 0,
				collectionFindingsQuery.data?.offset ?? collectionRequest.offset,
			),
		[collectionFindingsQuery.data, collectionRequest.offset],
	);

	const artifactWindow = useMemo(
		() =>
			findingsWindowLabel(
				artifactFindingsQuery.data?.total_active_findings ?? 0,
				artifactFindingsQuery.data?.returned ?? 0,
				artifactFindingsQuery.data?.offset ?? artifactRequest.offset,
			),
		[artifactFindingsQuery.data, artifactRequest.offset],
	);

	const canGoCollectionPrevious =
		collectionRequest.offset > 0 && !collectionFindingsQuery.isFetching;
	const canGoCollectionNext =
		(collectionFindingsQuery.data?.offset ?? collectionRequest.offset) +
			(collectionFindingsQuery.data?.returned ?? 0) <
			(collectionFindingsQuery.data?.total_active_findings ?? 0) &&
		!collectionFindingsQuery.isFetching;

	const canGoArtifactPrevious =
		artifactRequest.offset > 0 && !artifactFindingsQuery.isFetching;
	const canGoArtifactNext =
		(artifactFindingsQuery.data?.offset ?? artifactRequest.offset) +
			(artifactFindingsQuery.data?.returned ?? 0) <
			(artifactFindingsQuery.data?.total_active_findings ?? 0) &&
		!artifactFindingsQuery.isFetching;

	return (
		<AppShell apiHealth={healthLabel} currentView="findings">
			<section className="panel">
				<div className="panel-header">
					<div>
						<p className="eyebrow">Operations</p>
						<h2>Collection Active Findings</h2>
					</div>
					<button
						className="secondary-button"
						onClick={() => {
							void collectionFindingsQuery.refetch();
							void artifactFindingsQuery.refetch();
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
						setCollectionRequest({
							collectionKey: String(formData.get("collectionKey") ?? ""),
							minSeverity: String(formData.get("minSeverity") ?? "all"),
							packageName: String(formData.get("packageName") ?? ""),
							limit: Number(formData.get("limit") ?? 50),
							offset: 0,
						});
					}}
				>
					<label>
						Collection key
						<input
							defaultValue={collectionRequest.collectionKey}
							name="collectionKey"
						/>
					</label>
					<label>
						Package name
						<input
							defaultValue={collectionRequest.packageName}
							name="packageName"
						/>
					</label>
					<label>
						Minimum severity
						<select
							defaultValue={collectionRequest.minSeverity}
							name="minSeverity"
						>
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
							defaultValue={collectionRequest.limit}
							min={1}
							name="limit"
							type="number"
						/>
					</label>
					<button className="primary-button" type="submit">
						Query Collection
					</button>
				</form>

				{collectionFindingsQuery.isError ? (
					<p className="error-banner">
						{collectionFindingsQuery.error instanceof Error
							? collectionFindingsQuery.error.message
							: "failed to load collection active findings"}
					</p>
				) : null}

				<div className="results-meta">
					<span>
						Collection:{" "}
						{collectionFindingsQuery.data?.collection_key ??
							collectionRequest.collectionKey}
					</span>
					<span>
						Total: {collectionFindingsQuery.data?.total_active_findings ?? 0}
					</span>
					<span>Returned: {collectionFindingsQuery.data?.returned ?? 0}</span>
					<span>Offset: {collectionFindingsQuery.data?.offset ?? 0}</span>
					<span>
						Limit:{" "}
						{collectionFindingsQuery.data?.limit ?? collectionRequest.limit}
					</span>
					<span>{collectionWindow}</span>
				</div>

				<div className="results-meta">
					<button
						className="secondary-button"
						disabled={!canGoCollectionPrevious}
						onClick={() => {
							setCollectionRequest((current) => ({
								...current,
								offset: Math.max(0, current.offset - current.limit),
							}));
						}}
						type="button"
					>
						Previous Collection Page
					</button>
					<button
						className="secondary-button"
						disabled={!canGoCollectionNext}
						onClick={() => {
							setCollectionRequest((current) => ({
								...current,
								offset: current.offset + current.limit,
							}));
						}}
						type="button"
					>
						Next Collection Page
					</button>
				</div>

				<div className="table-wrap">
					<table>
						<thead>
							{collectionTable.getHeaderGroups().map((headerGroup) => (
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
							{collectionTable.getRowModel().rows.length === 0 ? (
								<tr>
									<td colSpan={collectionColumns.length}>
										No active findings for this collection yet.
									</td>
								</tr>
							) : (
								collectionTable.getRowModel().rows.map((row) => (
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

			<section className="panel">
				<div className="panel-header">
					<div>
						<p className="eyebrow">Diagnostics</p>
						<h2>Artifact Active Findings</h2>
					</div>
				</div>

				<form
					className="filters"
					onSubmit={(event) => {
						event.preventDefault();
						const formData = new FormData(event.currentTarget);
						setArtifactRequest({
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
						<input
							defaultValue={artifactRequest.componentKey}
							name="componentKey"
						/>
					</label>
					<label>
						Artifact kind
						<select
							defaultValue={artifactRequest.artifactKind}
							name="artifactKind"
						>
							<option value="container-image">container-image</option>
							<option value="sbom-document">sbom-document</option>
						</select>
					</label>
					<label>
						Artifact identity
						<input
							defaultValue={artifactRequest.artifactIdentity}
							name="artifactIdentity"
						/>
					</label>
					<label>
						Package name
						<input
							defaultValue={artifactRequest.packageName}
							name="packageName"
						/>
					</label>
					<label>
						Minimum severity
						<select
							defaultValue={artifactRequest.minSeverity}
							name="minSeverity"
						>
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
							defaultValue={artifactRequest.limit}
							min={1}
							name="limit"
							type="number"
						/>
					</label>
					<button className="primary-button" type="submit">
						Query Artifact
					</button>
				</form>

				{artifactFindingsQuery.isError ? (
					<p className="error-banner">
						{artifactFindingsQuery.error instanceof Error
							? artifactFindingsQuery.error.message
							: "failed to load active findings"}
					</p>
				) : null}

				<div className="results-meta">
					<span>
						Total: {artifactFindingsQuery.data?.total_active_findings ?? 0}
					</span>
					<span>Returned: {artifactFindingsQuery.data?.returned ?? 0}</span>
					<span>Offset: {artifactFindingsQuery.data?.offset ?? 0}</span>
					<span>
						Limit: {artifactFindingsQuery.data?.limit ?? artifactRequest.limit}
					</span>
					<span>{artifactWindow}</span>
				</div>

				<div className="results-meta">
					<button
						className="secondary-button"
						disabled={!canGoArtifactPrevious}
						onClick={() => {
							setArtifactRequest((current) => ({
								...current,
								offset: Math.max(0, current.offset - current.limit),
							}));
						}}
						type="button"
					>
						Previous Artifact Page
					</button>
					<button
						className="secondary-button"
						disabled={!canGoArtifactNext}
						onClick={() => {
							setArtifactRequest((current) => ({
								...current,
								offset: current.offset + current.limit,
							}));
						}}
						type="button"
					>
						Next Artifact Page
					</button>
				</div>

				<div className="table-wrap">
					<table>
						<thead>
							{artifactTable.getHeaderGroups().map((headerGroup) => (
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
							{artifactTable.getRowModel().rows.length === 0 ? (
								<tr>
									<td colSpan={artifactColumns.length}>
										No active findings yet.
									</td>
								</tr>
							) : (
								artifactTable.getRowModel().rows.map((row) => (
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
