import { useMutation, useQuery } from "@tanstack/react-query";
import { useMemo, useState } from "react";
import { AppShell } from "../app/app-shell";
import {
	bindArtifact,
	configureProvider,
	drainScanWorker,
	fetchApiHealth,
	fetchScanCommandStatus,
	registerComponent,
	requestScan,
} from "../lib/api";

export function OperationsPage() {
	const [operatorState, setOperatorState] = useState({
		componentKey: "component:payments-api",
		name: "Payments API",
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
