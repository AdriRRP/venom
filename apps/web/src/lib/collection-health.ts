import type { CollectionHealth } from "./api";

export function describeCollectionHealth(health: CollectionHealth) {
	return `${health.total} active - ${health.open} open - ${health.risk_accepted} risk accepted - ${health.suppressed} suppressed - ${health.critical_risk} critical risk - ${health.high_risk} high risk`;
}
