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
