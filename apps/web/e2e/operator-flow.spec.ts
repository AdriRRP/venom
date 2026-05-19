import { type APIRequestContext, expect, test } from "@playwright/test";

async function expectOk(
	response: Awaited<ReturnType<APIRequestContext["post"]>>,
) {
	expect(response.ok(), await response.text()).toBeTruthy();
}

async function seedReleaseCollection(request: APIRequestContext) {
	await expectOk(
		await request.post("/api/components", {
			data: {
				component_key: "component:payments-api",
				name: "Payments API",
			},
		}),
	);

	await expectOk(
		await request.post("/api/context-profiles", {
			data: {
				profile_key: "context:internet-prod",
				name: "Internet Production",
				internet_exposed: true,
				production: true,
				mission_critical: true,
			},
		}),
	);

	await expectOk(
		await request.post(
			"/api/components/component%3Apayments-api/context-profile",
			{
				data: {
					profile_key: "context:internet-prod",
				},
			},
		),
	);

	await expectOk(
		await request.post("/api/collections", {
			data: {
				collection_key: "release:2026.05",
				name: "May Release",
			},
		}),
	);

	await expectOk(
		await request.post("/api/collections/release%3A2026.05/components", {
			data: {
				component_key: "component:payments-api",
			},
		}),
	);

	await expectOk(
		await request.post("/api/components/component%3Apayments-api/artifacts", {
			data: {
				artifact_kind: "container-image",
				artifact_identity: "registry.example/payments@sha256:111",
			},
		}),
	);

	await expectOk(
		await request.post(
			"/api/components/component%3Apayments-api/provider-runtime",
			{
				data: {
					provider_key: "fixture-provider",
				},
			},
		),
	);

	await expectOk(
		await request.post("/api/collections/release%3A2026.05/scan-requests", {
			data: {
				freshness: "deterministic",
			},
		}),
	);

	await expectOk(
		await request.post("/api/scan-workers/drain", {
			data: {
				max_commands: 4,
				knowledge_revision: "fixture-rev-1",
				findings: [
					{
						vulnerability_id: "CVE-2026-0001",
						package_name: "openssl",
						package_version: "3.0.0",
						severity: "high",
					},
				],
			},
		}),
	);
}

test("operator console can manage one release collection and execute one scheduled scan", async ({
	page,
}) => {
	await page.goto("/operations");
	const collectionDetailCard = page
		.locator(".result-card")
		.filter({ hasText: "Current collection detail" });

	await page.getByRole("button", { name: "Register", exact: true }).click();
	await expect(page.getByText(/Managed components: 1\./i)).toBeVisible();

	await page.getByRole("button", { name: "Register Context Profile" }).click();
	await expect(page.getByText(/Managed context profiles: 1\./i)).toBeVisible();
	await expect(
		page.getByText(
			/context:internet-prod: Internet Production \(internet, production, critical\)/i,
		),
	).toBeVisible();

	await page.getByRole("button", { name: "Assign Context Profile" }).click();
	await expect(
		page.getByText(/Change: assigned\. Profile: context:internet-prod\./i),
	).toBeVisible();

	await page.getByRole("button", { name: "Create Collection" }).click();
	await expect(page.getByText(/Managed collections: 1\./i)).toBeVisible();

	await page.getByRole("button", { name: "Add Component" }).click();
	await expect(page.getByText(/Members: 1\./i)).toBeVisible();
	await expect(page.getByText(/release:2026.05/i)).toBeVisible();

	await page.getByRole("button", { name: "Bind Artifact" }).click();
	await expect(page.getByText(/Bound artifacts: 1\./i)).toBeVisible();

	await page.getByRole("button", { name: "Configure Provider" }).click();
	await expect(page.getByText(/Provider: fixture-provider\./i)).toBeVisible();

	await page
		.getByRole("button", { name: "Configure Collection Schedule" })
		.click();
	await expect(page.getByText(/Cadence: 60 minutes\./i)).toBeVisible();
	await expect(page.getByText(/Due now: 1\./i)).toBeVisible();
	await expect(page.getByText(/due now - every 60 minutes/i)).toBeVisible();

	await page.getByRole("button", { name: "Run Collection Scheduler" }).click();
	await expect(
		page.getByText(/Processed collections: 1\. Enqueued commands: 1\./i),
	).toBeVisible();

	await page.getByRole("button", { name: "Run Worker" }).click();
	await expect(page.getByText(/Processed: 1\./i)).toBeVisible();
	await expect(page.getByText(/Active findings: 1\./i)).toBeVisible();
	await expect(
		collectionDetailCard.getByText(
			/1 active - 1 open - 0 risk accepted - 0 suppressed - 1 critical risk - 0 high risk/i,
		),
	).toBeVisible();

	await page.getByRole("link", { name: "Release Dashboard" }).click();
	await expect(
		page.getByRole("heading", { level: 2, name: "Release Dashboard" }),
	).toBeVisible();
	await expect(page.getByText(/1 scheduled,\s*0 due now/i)).toBeVisible();
	await expect(page.getByText("May Release")).toBeVisible();
	await expect(
		page.getByText(
			/Health: 1 active - 1 open - 0 risk accepted - 0 suppressed - 1 critical risk - 0 high risk/i,
		),
	).toBeVisible();
});

test("findings console can query one seeded release collection", async ({
	page,
	request,
}) => {
	await seedReleaseCollection(request);

	await page.goto("/findings");
	const collectionPanel = page.locator("section.panel").first();
	await expect(
		collectionPanel.getByRole("textbox", { name: "Collection key" }),
	).toHaveValue("release:2026.05");
	await expect(collectionPanel.getByText("Showing 1-1 of 1")).toBeVisible();
	await expect(
		collectionPanel.getByText(
			/Health: 1 active - 1 open - 0 risk accepted - 0 suppressed - 1 critical risk - 0 high risk/i,
		),
	).toBeVisible();
	await expect(
		collectionPanel.getByRole("button", { name: "Open (1)" }),
	).toBeVisible();
	await expect(
		collectionPanel.getByRole("cell", { name: "component:payments-api" }),
	).toBeVisible();
	await expect(
		collectionPanel.getByRole("cell", { name: "CVE-2026-0001" }),
	).toBeVisible();
	await expect(
		collectionPanel.getByRole("cell", { name: "openssl@3.0.0" }),
	).toBeVisible();
	await expect(
		collectionPanel.getByRole("cell", { name: "critical" }),
	).toBeVisible();
	await expect(
		collectionPanel.getByRole("cell", { name: "Internet Production" }),
	).toBeVisible();
	await collectionPanel
		.getByRole("combobox", { name: "Governance" })
		.selectOption("open");
	await collectionPanel
		.getByRole("button", { name: "Query Collection" })
		.click();
	await collectionPanel
		.getByRole("textbox", { name: "Reason" })
		.fill("Accepted for this release");
	await collectionPanel
		.getByRole("button", { name: "Accept Filtered Open Findings" })
		.click();
	await expect(
		collectionPanel.getByText("Governance: risk-accepted (1/1 accepted)."),
	).toBeVisible();
	await expect(
		collectionPanel.getByText(
			/Health: 1 active - 0 open - 1 risk accepted - 0 suppressed - 1 critical risk - 0 high risk/i,
		),
	).toBeVisible();

	await collectionPanel
		.getByRole("button", { name: "Suppress", exact: true })
		.click();
	await page
		.getByRole("textbox", { name: "Reason" })
		.fill("Known upstream false alarm");
	await page.getByRole("button", { name: "Submit Suppression" }).click();
	await expect(
		collectionPanel.getByText("suppressed: Known upstream false alarm"),
	).toBeVisible();
	await expect(
		collectionPanel.getByText(
			/Health: 1 active - 0 open - 0 risk accepted - 1 suppressed - 1 critical risk - 0 high risk/i,
		),
	).toBeVisible();
	await expect(
		collectionPanel.getByRole("button", { name: "Suppressed (1)" }),
	).toBeVisible();
	await collectionPanel
		.getByRole("combobox", { name: "Governance" })
		.selectOption("suppressed");
	await collectionPanel
		.getByRole("button", { name: "Query Collection" })
		.dispatchEvent("click");
	await expect(collectionPanel.getByText("Showing 1-1 of 1")).toBeVisible();
});
