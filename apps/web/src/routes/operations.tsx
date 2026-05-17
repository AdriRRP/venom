import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useMemo, useState } from "react";
import { AppShell } from "../app/app-shell";
import {
	addCollectionComponent,
	bindArtifact,
	configureCollectionScanSchedule,
	configureProvider,
	drainCollectionScanWorker,
	drainScanWorker,
	fetchApiHealth,
	fetchCollectionDetail,
	fetchCollections,
	fetchScanCommandStatus,
	registerCollection,
	registerComponent,
	requestCollectionScan,
	requestScan,
} from "../lib/api";

export function OperationsPage() {
	const queryClient = useQueryClient();
	const [operatorState, setOperatorState] = useState({
		componentKey: "component:payments-api",
		name: "Payments API",
		collectionKey: "release:2026.05",
		collectionName: "May Release",
		collectionComponentKey: "component:payments-api",
		cadenceMinutes: "60",
		maxCollections: "8",
		artifactKind: "container-image",
		artifactIdentity: "registry.example/payments@sha256:111",
		providerKey: "fixture-provider",
		freshness: "deterministic",
		commandId: "",
		knowledgeRevision: "fixture-rev-1",
		findingVulnerabilityId: "CVE-2026-0001",
		findingPackageName: "openssl",
		findingPackageVersion: "3.0.0",
		findingSeverity: "high",
	});

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

	const registerComponentMutation = useMutation({
		mutationFn: registerComponent,
		onSuccess: () => {
			void queryClient.invalidateQueries({ queryKey: ["collections"] });
			void queryClient.invalidateQueries({
				queryKey: ["collection-detail", operatorState.collectionKey],
			});
		},
	});

	const registerCollectionMutation = useMutation({
		mutationFn: registerCollection,
		onSuccess: () => {
			void queryClient.invalidateQueries({ queryKey: ["collections"] });
			void queryClient.invalidateQueries({
				queryKey: ["collection-detail", operatorState.collectionKey],
			});
		},
	});

	const addCollectionComponentMutation = useMutation({
		mutationFn: (request: { collectionKey: string; componentKey: string }) =>
			addCollectionComponent(request.collectionKey, {
				componentKey: request.componentKey,
			}),
		onSuccess: () => {
			void queryClient.invalidateQueries({ queryKey: ["collections"] });
			void queryClient.invalidateQueries({
				queryKey: ["collection-detail", operatorState.collectionKey],
			});
		},
	});

	const configureCollectionScanScheduleMutation = useMutation({
		mutationFn: (request: {
			collectionKey: string;
			cadenceMinutes: number;
			freshness: string;
		}) => configureCollectionScanSchedule(request),
		onSuccess: () => {
			void queryClient.invalidateQueries({ queryKey: ["collections"] });
			void queryClient.invalidateQueries({
				queryKey: ["collection-detail", operatorState.collectionKey],
			});
		},
	});

	const bindArtifactMutation = useMutation({
		mutationFn: (request: {
			componentKey: string;
			artifactKind: string;
			artifactIdentity: string;
		}) =>
			bindArtifact(request.componentKey, {
				artifactKind: request.artifactKind,
				artifactIdentity: request.artifactIdentity,
			}),
	});

	const configureProviderMutation = useMutation({
		mutationFn: (request: { componentKey: string; providerKey: string }) =>
			configureProvider(request.componentKey, {
				providerKey: request.providerKey,
			}),
	});

	const requestScanMutation = useMutation({
		mutationFn: requestScan,
		onSuccess: (data) => {
			setOperatorState((current) => ({
				...current,
				commandId: data.command_id,
			}));
		},
	});

	const requestCollectionScanMutation = useMutation({
		mutationFn: requestCollectionScan,
		onSuccess: (data) => {
			setOperatorState((current) => ({
				...current,
				commandId: data.command_ids[0] ?? current.commandId,
			}));
		},
	});

	const scanCommandStatusMutation = useMutation({
		mutationFn: fetchScanCommandStatus,
	});

	const drainWorkerMutation = useMutation({
		mutationFn: drainScanWorker,
		onSuccess: (data) => {
			if (data.last_command_id) {
				setOperatorState((current) => ({
					...current,
					commandId: data.last_command_id ?? current.commandId,
				}));
			}
		},
	});

	const drainCollectionScanWorkerMutation = useMutation({
		mutationFn: drainCollectionScanWorker,
		onSuccess: () => {
			void queryClient.invalidateQueries({ queryKey: ["collections"] });
			void queryClient.invalidateQueries({
				queryKey: ["collection-detail", operatorState.collectionKey],
			});
		},
	});

	const collectionsQuery = useQuery({
		queryKey: ["collections"],
		queryFn: fetchCollections,
		refetchInterval: 15_000,
	});

	const collectionDetailQuery = useQuery({
		queryKey: ["collection-detail", operatorState.collectionKey],
		queryFn: () => fetchCollectionDetail(operatorState.collectionKey),
		enabled: operatorState.collectionKey.length > 0,
		refetchInterval: 15_000,
	});

	const scheduledCollectionSummary = useMemo(() => {
		const collections = collectionsQuery.data?.collections ?? [];
		const scheduled = collections.filter(
			(collection) => collection.scan_schedule !== null,
		);
		const dueNow = scheduled.filter((collection) => collection.due_now);
		return {
			total: collections.length,
			scheduled: scheduled.length,
			dueNow: dueNow.length,
		};
	}, [collectionsQuery.data]);

	return (
		<AppShell apiHealth={healthLabel} currentView="operations">
			<div className="panel-stack">
				<section className="panel">
					<div className="panel-header">
						<div>
							<p className="eyebrow">Operations</p>
							<h2>Managed Components</h2>
						</div>
					</div>
					<p className="copy">
						Register components, bind immutable artifacts, configure one
						provider, and request canonical scans from the operator console.
					</p>
				</section>

				<section className="panel">
					<div className="panel-header">
						<div>
							<p className="eyebrow">Inventory</p>
							<h2>Register Component</h2>
						</div>
					</div>
					<form
						className="filters mutation-grid"
						onSubmit={(event) => {
							event.preventDefault();
							void registerComponentMutation.mutateAsync({
								componentKey: operatorState.componentKey,
								name: operatorState.name,
							});
						}}
					>
						<label>
							Component key
							<input
								name="componentKey"
								onChange={(event) =>
									setOperatorState((current) => ({
										...current,
										componentKey: event.target.value,
									}))
								}
								value={operatorState.componentKey}
							/>
						</label>
						<label>
							Name
							<input
								name="name"
								onChange={(event) =>
									setOperatorState((current) => ({
										...current,
										name: event.target.value,
									}))
								}
								value={operatorState.name}
							/>
						</label>
						<button className="primary-button" type="submit">
							Register
						</button>
					</form>
					{registerComponentMutation.data ? (
						<div className="result-card">
							<strong>Last registration</strong>
							<p>
								Change: {registerComponentMutation.data.change}. Managed
								components: {registerComponentMutation.data.managed_components}.
							</p>
						</div>
					) : null}
				</section>

				<section className="panel">
					<div className="panel-header">
						<div>
							<p className="eyebrow">Release Scope</p>
							<h2>Create Collection</h2>
						</div>
					</div>
					<form
						className="filters mutation-grid"
						onSubmit={(event) => {
							event.preventDefault();
							void registerCollectionMutation.mutateAsync({
								collectionKey: operatorState.collectionKey,
								name: operatorState.collectionName,
							});
						}}
					>
						<label>
							Collection key
							<input
								name="collectionKey"
								onChange={(event) =>
									setOperatorState((current) => ({
										...current,
										collectionKey: event.target.value,
									}))
								}
								value={operatorState.collectionKey}
							/>
						</label>
						<label>
							Name
							<input
								name="collectionName"
								onChange={(event) =>
									setOperatorState((current) => ({
										...current,
										collectionName: event.target.value,
									}))
								}
								value={operatorState.collectionName}
							/>
						</label>
						<button className="primary-button" type="submit">
							Create Collection
						</button>
					</form>
					{registerCollectionMutation.data ? (
						<div className="result-card">
							<strong>Last collection change</strong>
							<p>
								Change: {registerCollectionMutation.data.change}. Managed
								collections:{" "}
								{registerCollectionMutation.data.managed_collections}.
							</p>
						</div>
					) : null}
				</section>

				<section className="panel">
					<div className="panel-header">
						<div>
							<p className="eyebrow">Release Scope</p>
							<h2>Manage Collection Membership</h2>
						</div>
					</div>
					<form
						className="filters mutation-grid"
						onSubmit={(event) => {
							event.preventDefault();
							void addCollectionComponentMutation.mutateAsync({
								collectionKey: operatorState.collectionKey,
								componentKey: operatorState.collectionComponentKey,
							});
						}}
					>
						<label>
							Collection key
							<input
								name="membershipCollectionKey"
								onChange={(event) =>
									setOperatorState((current) => ({
										...current,
										collectionKey: event.target.value,
									}))
								}
								value={operatorState.collectionKey}
							/>
						</label>
						<label>
							Component key
							<input
								name="collectionComponentKey"
								onChange={(event) =>
									setOperatorState((current) => ({
										...current,
										collectionComponentKey: event.target.value,
									}))
								}
								value={operatorState.collectionComponentKey}
							/>
						</label>
						<button className="primary-button" type="submit">
							Add Component
						</button>
					</form>
					{addCollectionComponentMutation.data ? (
						<div className="result-card">
							<strong>Last collection membership</strong>
							<p>
								Change: {addCollectionComponentMutation.data.change}. Members:{" "}
								{addCollectionComponentMutation.data.members}.
							</p>
						</div>
					) : null}
					<div className="result-card">
						<strong>Collections</strong>
						<p>
							Total: {scheduledCollectionSummary.total}. Scheduled:{" "}
							{scheduledCollectionSummary.scheduled}. Due now:{" "}
							{scheduledCollectionSummary.dueNow}.
						</p>
						{collectionsQuery.data ? (
							<ul>
								{collectionsQuery.data.collections.map((collection) => (
									<li key={collection.collection_key}>
										{collection.collection_key} ({collection.name}) -{" "}
										{collection.members} members -{" "}
										{collection.scan_schedule
											? `${collection.due_now ? "due now" : "due later"} - every ${collection.scan_schedule.cadence_minutes} minutes (${collection.scan_schedule.freshness})`
											: "manual only"}
									</li>
								))}
							</ul>
						) : (
							<p>No collections loaded yet.</p>
						)}
					</div>
					<div className="result-card">
						<strong>Current collection detail</strong>
						{collectionDetailQuery.data ? (
							<>
								{collectionDetailQuery.data.scan_schedule ? (
									<p>
										Schedule: every{" "}
										{collectionDetailQuery.data.scan_schedule.cadence_minutes}{" "}
										minutes (
										{collectionDetailQuery.data.scan_schedule.freshness}). Next
										due at{" "}
										{
											collectionDetailQuery.data.scan_schedule
												.next_due_at_unix_ms
										}
										.
									</p>
								) : (
									<p>No schedule configured.</p>
								)}
								<ul>
									{collectionDetailQuery.data.members.map((member) => (
										<li key={member.component_key}>{member.component_key}</li>
									))}
								</ul>
							</>
						) : (
							<p>No collection detail loaded yet.</p>
						)}
					</div>
				</section>

				<section className="panel">
					<div className="panel-header">
						<div>
							<p className="eyebrow">Scheduling</p>
							<h2>Configure Collection Schedule</h2>
						</div>
					</div>
					<form
						className="filters mutation-grid"
						onSubmit={(event) => {
							event.preventDefault();
							void configureCollectionScanScheduleMutation.mutateAsync({
								collectionKey: operatorState.collectionKey,
								cadenceMinutes: Number.parseInt(
									operatorState.cadenceMinutes,
									10,
								),
								freshness: operatorState.freshness,
							});
						}}
					>
						<label>
							Collection key
							<input
								name="scheduleCollectionKey"
								onChange={(event) =>
									setOperatorState((current) => ({
										...current,
										collectionKey: event.target.value,
									}))
								}
								value={operatorState.collectionKey}
							/>
						</label>
						<label>
							Cadence minutes
							<input
								name="cadenceMinutes"
								onChange={(event) =>
									setOperatorState((current) => ({
										...current,
										cadenceMinutes: event.target.value,
									}))
								}
								value={operatorState.cadenceMinutes}
							/>
						</label>
						<label>
							Freshness
							<select
								name="scheduleFreshness"
								onChange={(event) =>
									setOperatorState((current) => ({
										...current,
										freshness: event.target.value,
									}))
								}
								value={operatorState.freshness}
							>
								<option value="deterministic">deterministic</option>
								<option value="live">live</option>
							</select>
						</label>
						<button className="primary-button" type="submit">
							Configure Collection Schedule
						</button>
					</form>
					{configureCollectionScanScheduleMutation.data ? (
						<div className="result-card">
							<strong>Last collection schedule</strong>
							<p>
								Change: {configureCollectionScanScheduleMutation.data.change}.
								Cadence:{" "}
								{configureCollectionScanScheduleMutation.data.cadence_minutes}{" "}
								minutes. Next due at{" "}
								{
									configureCollectionScanScheduleMutation.data
										.next_due_at_unix_ms
								}
								.
							</p>
						</div>
					) : null}
				</section>

				<section className="panel">
					<div className="panel-header">
						<div>
							<p className="eyebrow">Inventory</p>
							<h2>Bind Managed Artifact</h2>
						</div>
					</div>
					<form
						className="filters mutation-grid"
						onSubmit={(event) => {
							event.preventDefault();
							void bindArtifactMutation.mutateAsync({
								componentKey: operatorState.componentKey,
								artifactKind: operatorState.artifactKind,
								artifactIdentity: operatorState.artifactIdentity,
							});
						}}
					>
						<label>
							Component key
							<input
								name="bindComponentKey"
								onChange={(event) =>
									setOperatorState((current) => ({
										...current,
										componentKey: event.target.value,
									}))
								}
								value={operatorState.componentKey}
							/>
						</label>
						<label>
							Artifact kind
							<select
								name="artifactKind"
								onChange={(event) =>
									setOperatorState((current) => ({
										...current,
										artifactKind: event.target.value,
									}))
								}
								value={operatorState.artifactKind}
							>
								<option value="container-image">container-image</option>
								<option value="sbom-document">sbom-document</option>
							</select>
						</label>
						<label>
							Artifact identity
							<input
								name="artifactIdentity"
								onChange={(event) =>
									setOperatorState((current) => ({
										...current,
										artifactIdentity: event.target.value,
									}))
								}
								value={operatorState.artifactIdentity}
							/>
						</label>
						<button className="primary-button" type="submit">
							Bind Artifact
						</button>
					</form>
					{bindArtifactMutation.data ? (
						<div className="result-card">
							<strong>Last artifact binding</strong>
							<p>
								Change: {bindArtifactMutation.data.change}. Bound artifacts:{" "}
								{bindArtifactMutation.data.bound_artifacts}.
							</p>
						</div>
					) : null}
				</section>

				<section className="panel">
					<div className="panel-header">
						<div>
							<p className="eyebrow">Scanning</p>
							<h2>Configure Provider Runtime</h2>
						</div>
					</div>
					<form
						className="filters mutation-grid"
						onSubmit={(event) => {
							event.preventDefault();
							void configureProviderMutation.mutateAsync({
								componentKey: operatorState.componentKey,
								providerKey: operatorState.providerKey,
							});
						}}
					>
						<label>
							Component key
							<input
								name="providerComponentKey"
								onChange={(event) =>
									setOperatorState((current) => ({
										...current,
										componentKey: event.target.value,
									}))
								}
								value={operatorState.componentKey}
							/>
						</label>
						<label>
							Provider key
							<input
								name="providerKey"
								onChange={(event) =>
									setOperatorState((current) => ({
										...current,
										providerKey: event.target.value,
									}))
								}
								value={operatorState.providerKey}
							/>
						</label>
						<button className="primary-button" type="submit">
							Configure Provider
						</button>
					</form>
					{configureProviderMutation.data ? (
						<div className="result-card">
							<strong>Last provider configuration</strong>
							<p>
								Change: {configureProviderMutation.data.change}. Provider:{" "}
								{configureProviderMutation.data.provider_key ?? "none"}.
							</p>
						</div>
					) : null}
				</section>

				<section className="panel">
					<div className="panel-header">
						<div>
							<p className="eyebrow">Scanning</p>
							<h2>Request Canonical Scan</h2>
						</div>
					</div>
					<form
						className="filters mutation-grid"
						onSubmit={(event) => {
							event.preventDefault();
							void requestScanMutation.mutateAsync({
								componentKey: operatorState.componentKey,
								artifactKind: operatorState.artifactKind,
								artifactIdentity: operatorState.artifactIdentity,
								freshness: operatorState.freshness,
							});
						}}
					>
						<label>
							Component key
							<input
								name="scanComponentKey"
								onChange={(event) =>
									setOperatorState((current) => ({
										...current,
										componentKey: event.target.value,
									}))
								}
								value={operatorState.componentKey}
							/>
						</label>
						<label>
							Artifact kind
							<select
								name="scanArtifactKind"
								onChange={(event) =>
									setOperatorState((current) => ({
										...current,
										artifactKind: event.target.value,
									}))
								}
								value={operatorState.artifactKind}
							>
								<option value="container-image">container-image</option>
								<option value="sbom-document">sbom-document</option>
							</select>
						</label>
						<label>
							Artifact identity
							<input
								name="scanArtifactIdentity"
								onChange={(event) =>
									setOperatorState((current) => ({
										...current,
										artifactIdentity: event.target.value,
									}))
								}
								value={operatorState.artifactIdentity}
							/>
						</label>
						<label>
							Freshness
							<select
								name="freshness"
								onChange={(event) =>
									setOperatorState((current) => ({
										...current,
										freshness: event.target.value,
									}))
								}
								value={operatorState.freshness}
							>
								<option value="deterministic">deterministic</option>
								<option value="live">live</option>
							</select>
						</label>
						<button className="primary-button" type="submit">
							Request Scan
						</button>
					</form>
					{requestScanMutation.data ? (
						<div className="result-card">
							<strong>Last scan request</strong>
							<p>
								Command: {requestScanMutation.data.command_id}. Status:{" "}
								{requestScanMutation.data.status}. Freshness:{" "}
								{requestScanMutation.data.freshness}.
							</p>
						</div>
					) : null}
				</section>

				<section className="panel">
					<div className="panel-header">
						<div>
							<p className="eyebrow">Scanning</p>
							<h2>Request Collection Scan</h2>
						</div>
					</div>
					<form
						className="filters mutation-grid"
						onSubmit={(event) => {
							event.preventDefault();
							void requestCollectionScanMutation.mutateAsync({
								collectionKey: operatorState.collectionKey,
								freshness: operatorState.freshness,
							});
						}}
					>
						<label>
							Collection key
							<input
								name="scanCollectionKey"
								onChange={(event) =>
									setOperatorState((current) => ({
										...current,
										collectionKey: event.target.value,
									}))
								}
								value={operatorState.collectionKey}
							/>
						</label>
						<label>
							Freshness
							<select
								name="collectionFreshness"
								onChange={(event) =>
									setOperatorState((current) => ({
										...current,
										freshness: event.target.value,
									}))
								}
								value={operatorState.freshness}
							>
								<option value="deterministic">deterministic</option>
								<option value="live">live</option>
							</select>
						</label>
						<button className="primary-button" type="submit">
							Request Collection Scan
						</button>
					</form>
					{requestCollectionScanMutation.data ? (
						<div className="result-card">
							<strong>Last collection scan request</strong>
							<p>
								Collection: {requestCollectionScanMutation.data.collection_key}.
								Enqueued: {requestCollectionScanMutation.data.enqueued}. First
								command:{" "}
								{requestCollectionScanMutation.data.command_ids[0] ?? "none"}.
							</p>
						</div>
					) : null}
				</section>

				<section className="panel">
					<div className="panel-header">
						<div>
							<p className="eyebrow">Scheduling</p>
							<h2>Run Collection Scheduler</h2>
						</div>
					</div>
					<form
						className="filters mutation-grid"
						onSubmit={(event) => {
							event.preventDefault();
							void drainCollectionScanWorkerMutation.mutateAsync({
								maxCollections: Number.parseInt(
									operatorState.maxCollections,
									10,
								),
							});
						}}
					>
						<label>
							Max collections
							<input
								name="maxCollections"
								onChange={(event) =>
									setOperatorState((current) => ({
										...current,
										maxCollections: event.target.value,
									}))
								}
								value={operatorState.maxCollections}
							/>
						</label>
						<button className="primary-button" type="submit">
							Run Collection Scheduler
						</button>
					</form>
					{drainCollectionScanWorkerMutation.data ? (
						<div className="result-card">
							<strong>Last scheduler run</strong>
							<p>
								Outcome: {drainCollectionScanWorkerMutation.data.outcome}.
								Processed collections:{" "}
								{drainCollectionScanWorkerMutation.data.processed_collections}.
								Enqueued commands:{" "}
								{drainCollectionScanWorkerMutation.data.enqueued_commands}.
								Pending due remaining:{" "}
								{drainCollectionScanWorkerMutation.data.pending_due_remaining}.
								Last collection:{" "}
								{drainCollectionScanWorkerMutation.data.last_collection_key ??
									"none"}
								.
							</p>
						</div>
					) : null}
				</section>

				<section className="panel">
					<div className="panel-header">
						<div>
							<p className="eyebrow">Scanning</p>
							<h2>Command Status</h2>
						</div>
					</div>
					<form
						className="filters mutation-grid"
						onSubmit={(event) => {
							event.preventDefault();
							void scanCommandStatusMutation.mutateAsync(
								operatorState.commandId,
							);
						}}
					>
						<label>
							Command id
							<input
								name="commandId"
								onChange={(event) =>
									setOperatorState((current) => ({
										...current,
										commandId: event.target.value,
									}))
								}
								value={operatorState.commandId}
							/>
						</label>
						<button className="secondary-button" type="submit">
							Refresh Status
						</button>
					</form>
					{scanCommandStatusMutation.data ? (
						<div className="result-card">
							<strong>Current command status</strong>
							<p>
								Command: {scanCommandStatusMutation.data.command_id}. Status:{" "}
								{scanCommandStatusMutation.data.status}.
							</p>
						</div>
					) : null}
				</section>

				<section className="panel">
					<div className="panel-header">
						<div>
							<p className="eyebrow">Worker</p>
							<h2>Run Fixture Worker</h2>
						</div>
					</div>
					<form
						className="filters mutation-grid"
						onSubmit={(event) => {
							event.preventDefault();
							void drainWorkerMutation.mutateAsync({
								maxCommands: 1,
								knowledgeRevision: operatorState.knowledgeRevision,
								findings: [
									{
										vulnerabilityId: operatorState.findingVulnerabilityId,
										packageName: operatorState.findingPackageName,
										packageVersion: operatorState.findingPackageVersion,
										severity: operatorState.findingSeverity,
									},
								],
							});
						}}
					>
						<label>
							Knowledge revision
							<input
								name="knowledgeRevision"
								onChange={(event) =>
									setOperatorState((current) => ({
										...current,
										knowledgeRevision: event.target.value,
									}))
								}
								value={operatorState.knowledgeRevision}
							/>
						</label>
						<label>
							Vulnerability id
							<input
								name="findingVulnerabilityId"
								onChange={(event) =>
									setOperatorState((current) => ({
										...current,
										findingVulnerabilityId: event.target.value,
									}))
								}
								value={operatorState.findingVulnerabilityId}
							/>
						</label>
						<label>
							Package name
							<input
								name="findingPackageName"
								onChange={(event) =>
									setOperatorState((current) => ({
										...current,
										findingPackageName: event.target.value,
									}))
								}
								value={operatorState.findingPackageName}
							/>
						</label>
						<label>
							Package version
							<input
								name="findingPackageVersion"
								onChange={(event) =>
									setOperatorState((current) => ({
										...current,
										findingPackageVersion: event.target.value,
									}))
								}
								value={operatorState.findingPackageVersion}
							/>
						</label>
						<label>
							Severity
							<select
								name="findingSeverity"
								onChange={(event) =>
									setOperatorState((current) => ({
										...current,
										findingSeverity: event.target.value,
									}))
								}
								value={operatorState.findingSeverity}
							>
								<option value="low">low</option>
								<option value="medium">medium</option>
								<option value="high">high</option>
								<option value="critical">critical</option>
							</select>
						</label>
						<button className="primary-button" type="submit">
							Run Worker
						</button>
					</form>
					{drainWorkerMutation.data ? (
						<div className="result-card">
							<strong>Last worker run</strong>
							<p>
								Outcome: {drainWorkerMutation.data.outcome}. Processed:{" "}
								{drainWorkerMutation.data.processed}. Completed:{" "}
								{drainWorkerMutation.data.completed}. Failed:{" "}
								{drainWorkerMutation.data.failed}. Pending remaining:{" "}
								{drainWorkerMutation.data.pending_remaining}.
							</p>
						</div>
					) : null}
				</section>
			</div>
		</AppShell>
	);
}
