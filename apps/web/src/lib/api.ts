export type ApiHealthState = "healthy" | "unhealthy";

export async function fetchApiHealth(): Promise<ApiHealthState> {
	const response = await fetch("/api/health");
	return response.ok ? "healthy" : "unhealthy";
}
