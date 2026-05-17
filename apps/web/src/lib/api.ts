export type ApiHealthState = "healthy" | "unhealthy";

export type ActiveFinding = {
	vulnerability_id: string;
	package_name: string;
	package_version: string;
	severity: string;
};

export type ActiveFindingsResponse = {
	component_key: string;
	artifact_kind: string;
	artifact_identity: string;
	min_severity: string | null;
	package_name: string | null;
	total_active_findings: number;
	returned: number;
	offset: number;
	limit: number;
	active_findings: ActiveFinding[];
};

export type ActiveFindingsRequest = {
	componentKey: string;
	artifactKind: string;
	artifactIdentity: string;
	minSeverity?: string;
	packageName?: string;
	limit?: number;
	offset?: number;
};

export type ComponentRegistrationRequest = {
	componentKey: string;
	name: string;
};

export type RegisterComponentResponse = {
	change: string;
	managed_components: number;
};

export type CollectionRegistrationRequest = {
	collectionKey: string;
	name: string;
};

export type RegisterCollectionResponse = {
	change: string;
	managed_collections: number;
};

export type CollectionMembershipRequest = {
	componentKey: string;
};

export type CollectionMembershipResponse = {
	change: string;
	members: number;
};

export type CollectionSummary = {
	collection_key: string;
	name: string;
	members: number;
	scan_schedule: CollectionScanSchedule | null;
	due_now: boolean;
};

export type ListCollectionsResponse = {
	managed_collections: number;
	collections: CollectionSummary[];
};

export type CollectionDetailResponse = {
	collection_key: string;
	name: string;
	scan_schedule: CollectionScanSchedule | null;
	members: Array<{ component_key: string }>;
};

export type CollectionScanSchedule = {
	cadence_minutes: number;
	freshness: string;
	next_due_at_unix_ms: number;
	last_materialized_at_unix_ms: number | null;
	last_enqueued_commands: number | null;
};

export type BindArtifactRequest = {
	artifactKind: string;
	artifactIdentity: string;
};

export type BindArtifactResponse = {
	change: string;
	bound_artifacts: number;
};

export type ConfigureProviderRequest = {
	providerKey: string;
};

export type ConfigureProviderResponse = {
	change: string;
	provider_key: string | null;
};

export type ConfigureCollectionScanSchedulePayload = {
	collectionKey: string;
	cadenceMinutes: number;
	freshness: string;
};

export type ConfigureCollectionScanScheduleResponse = {
	change: string;
	collection_key: string;
	cadence_minutes: number;
	freshness: string;
	next_due_at_unix_ms: number;
};

export type RequestScanPayload = {
	componentKey: string;
	artifactKind: string;
	artifactIdentity: string;
	freshness: string;
};

export type RequestScanResponse = {
	command_id: string;
	status: string;
	component_key: string;
	artifact_kind: string;
	artifact_identity: string;
	freshness: string;
};

export type RequestCollectionScanPayload = {
	collectionKey: string;
	freshness: string;
};

export type RequestCollectionScanResponse = {
	collection_key: string;
	freshness: string;
	enqueued: number;
	command_ids: string[];
};

export type ScanCommandStatusResponse = {
	command_id: string;
	status: string;
};

export type DrainWorkerFindingInput = {
	vulnerabilityId: string;
	packageName: string;
	packageVersion: string;
	severity: string;
};

export type DrainWorkerPayload = {
	maxCommands?: number;
	knowledgeRevision?: string;
	findings?: DrainWorkerFindingInput[];
	errorKind?: string;
	errorMessage?: string;
	retryable?: boolean;
};

export type DrainWorkerResponse = {
	outcome: string;
	processed: number;
	completed: number;
	failed: number;
	pending_remaining: number;
	last_command_id: string | null;
	last_command_status: string | null;
	last_error_code: string | null;
	last_retryable: boolean | null;
};

export type DrainCollectionScanWorkerPayload = {
	maxCollections?: number;
};

export type DrainCollectionScanWorkerResponse = {
	outcome: string;
	processed_collections: number;
	enqueued_commands: number;
	pending_due_remaining: number;
	last_collection_key: string | null;
};

export async function fetchApiHealth(): Promise<ApiHealthState> {
	const response = await fetch("/api/health");
	return response.ok ? "healthy" : "unhealthy";
}

export async function fetchActiveFindings(
	request: ActiveFindingsRequest,
): Promise<ActiveFindingsResponse> {
	const params = new URLSearchParams({
		component_key: request.componentKey,
		artifact_kind: request.artifactKind,
		artifact_identity: request.artifactIdentity,
		limit: String(request.limit ?? 50),
		offset: String(request.offset ?? 0),
	});

	if (request.minSeverity && request.minSeverity !== "all") {
		params.set("min_severity", request.minSeverity);
	}

	if (request.packageName) {
		params.set("package_name", request.packageName);
	}

	const response = await fetch(`/api/findings/active?${params.toString()}`);
	if (!response.ok) {
		throw new Error(
			`active findings request failed with status ${response.status}`,
		);
	}

	return (await response.json()) as ActiveFindingsResponse;
}

export async function registerComponent(
	request: ComponentRegistrationRequest,
): Promise<RegisterComponentResponse> {
	const response = await fetch("/api/components", {
		method: "POST",
		headers: { "Content-Type": "application/json" },
		body: JSON.stringify({
			component_key: request.componentKey,
			name: request.name,
		}),
	});
	if (!response.ok) {
		throw new Error(
			`component registration failed with status ${response.status}`,
		);
	}
	return (await response.json()) as RegisterComponentResponse;
}

export async function registerCollection(
	request: CollectionRegistrationRequest,
): Promise<RegisterCollectionResponse> {
	const response = await fetch("/api/collections", {
		method: "POST",
		headers: { "Content-Type": "application/json" },
		body: JSON.stringify({
			collection_key: request.collectionKey,
			name: request.name,
		}),
	});
	if (!response.ok) {
		throw new Error(
			`collection creation failed with status ${response.status}`,
		);
	}
	return (await response.json()) as RegisterCollectionResponse;
}

export async function fetchCollections(): Promise<ListCollectionsResponse> {
	const response = await fetch("/api/collections");
	if (!response.ok) {
		throw new Error(`collections query failed with status ${response.status}`);
	}
	return (await response.json()) as ListCollectionsResponse;
}

export async function fetchCollectionDetail(
	collectionKey: string,
): Promise<CollectionDetailResponse> {
	const response = await fetch(
		`/api/collections/${encodeURIComponent(collectionKey)}`,
	);
	if (!response.ok) {
		throw new Error(
			`collection detail query failed with status ${response.status}`,
		);
	}
	return (await response.json()) as CollectionDetailResponse;
}

export async function addCollectionComponent(
	collectionKey: string,
	request: CollectionMembershipRequest,
): Promise<CollectionMembershipResponse> {
	const response = await fetch(
		`/api/collections/${encodeURIComponent(collectionKey)}/components`,
		{
			method: "POST",
			headers: { "Content-Type": "application/json" },
			body: JSON.stringify({
				component_key: request.componentKey,
			}),
		},
	);
	if (!response.ok) {
		throw new Error(
			`collection membership creation failed with status ${response.status}`,
		);
	}
	return (await response.json()) as CollectionMembershipResponse;
}

export async function configureCollectionScanSchedule(
	request: ConfigureCollectionScanSchedulePayload,
): Promise<ConfigureCollectionScanScheduleResponse> {
	const response = await fetch(
		`/api/collections/${encodeURIComponent(request.collectionKey)}/scan-schedule`,
		{
			method: "POST",
			headers: { "Content-Type": "application/json" },
			body: JSON.stringify({
				cadence_minutes: request.cadenceMinutes,
				freshness: request.freshness,
			}),
		},
	);
	if (!response.ok) {
		throw new Error(
			`collection scan schedule failed with status ${response.status}`,
		);
	}
	return (await response.json()) as ConfigureCollectionScanScheduleResponse;
}

export async function requestCollectionScan(
	request: RequestCollectionScanPayload,
): Promise<RequestCollectionScanResponse> {
	const response = await fetch(
		`/api/collections/${encodeURIComponent(request.collectionKey)}/scan-requests`,
		{
			method: "POST",
			headers: { "Content-Type": "application/json" },
			body: JSON.stringify({
				freshness: request.freshness,
			}),
		},
	);
	if (!response.ok) {
		throw new Error(
			`collection scan request failed with status ${response.status}`,
		);
	}
	return (await response.json()) as RequestCollectionScanResponse;
}

export async function drainCollectionScanWorker(
	request: DrainCollectionScanWorkerPayload,
): Promise<DrainCollectionScanWorkerResponse> {
	const response = await fetch("/api/collection-scan-workers/drain", {
		method: "POST",
		headers: { "Content-Type": "application/json" },
		body: JSON.stringify({
			max_collections: request.maxCollections,
		}),
	});
	if (!response.ok) {
		throw new Error(
			`collection scan worker drain failed with status ${response.status}`,
		);
	}
	return (await response.json()) as DrainCollectionScanWorkerResponse;
}

export async function bindArtifact(
	componentKey: string,
	request: BindArtifactRequest,
): Promise<BindArtifactResponse> {
	const response = await fetch(
		`/api/components/${encodeURIComponent(componentKey)}/artifacts`,
		{
			method: "POST",
			headers: { "Content-Type": "application/json" },
			body: JSON.stringify({
				artifact_kind: request.artifactKind,
				artifact_identity: request.artifactIdentity,
			}),
		},
	);
	if (!response.ok) {
		throw new Error(`artifact binding failed with status ${response.status}`);
	}
	return (await response.json()) as BindArtifactResponse;
}

export async function configureProvider(
	componentKey: string,
	request: ConfigureProviderRequest,
): Promise<ConfigureProviderResponse> {
	const response = await fetch(
		`/api/components/${encodeURIComponent(componentKey)}/provider-runtime`,
		{
			method: "POST",
			headers: { "Content-Type": "application/json" },
			body: JSON.stringify({
				provider_key: request.providerKey,
			}),
		},
	);
	if (!response.ok) {
		throw new Error(
			`provider configuration failed with status ${response.status}`,
		);
	}
	return (await response.json()) as ConfigureProviderResponse;
}

export async function requestScan(
	request: RequestScanPayload,
): Promise<RequestScanResponse> {
	const response = await fetch("/api/scan-requests", {
		method: "POST",
		headers: { "Content-Type": "application/json" },
		body: JSON.stringify({
			component_key: request.componentKey,
			artifact_kind: request.artifactKind,
			artifact_identity: request.artifactIdentity,
			freshness: request.freshness,
		}),
	});
	if (!response.ok) {
		throw new Error(`scan request failed with status ${response.status}`);
	}
	return (await response.json()) as RequestScanResponse;
}

export async function fetchScanCommandStatus(
	commandId: string,
): Promise<ScanCommandStatusResponse> {
	const response = await fetch(
		`/api/scan-commands/${encodeURIComponent(commandId)}`,
	);
	if (!response.ok) {
		throw new Error(
			`scan command status failed with status ${response.status}`,
		);
	}
	return (await response.json()) as ScanCommandStatusResponse;
}

export async function drainScanWorker(
	request: DrainWorkerPayload,
): Promise<DrainWorkerResponse> {
	const response = await fetch("/api/scan-workers/drain", {
		method: "POST",
		headers: { "Content-Type": "application/json" },
		body: JSON.stringify({
			max_commands: request.maxCommands,
			knowledge_revision: request.knowledgeRevision,
			findings: request.findings?.map((finding) => ({
				vulnerability_id: finding.vulnerabilityId,
				package_name: finding.packageName,
				package_version: finding.packageVersion,
				severity: finding.severity,
			})),
			error_kind: request.errorKind,
			error_message: request.errorMessage,
			retryable: request.retryable,
		}),
	});
	if (!response.ok) {
		throw new Error(`worker drain failed with status ${response.status}`);
	}
	return (await response.json()) as DrainWorkerResponse;
}
