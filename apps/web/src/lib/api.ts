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
