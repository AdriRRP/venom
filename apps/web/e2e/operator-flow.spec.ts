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

	await page.getByRole("button", { name: "Register" }).click();
	await expect(page.getByText(/Managed components: 1\./i)).toBeVisible();

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
		collectionPanel.getByRole("cell", { name: "component:payments-api" }),
	).toBeVisible();
	await expect(
		collectionPanel.getByRole("cell", { name: "CVE-2026-0001" }),
	).toBeVisible();
	await expect(
		collectionPanel.getByRole("cell", { name: "openssl@3.0.0" }),
	).toBeVisible();
	await collectionPanel.getByRole("button", { name: "Accept Risk" }).click();
	await page
		.getByRole("textbox", { name: "Reason" })
		.fill("Compensating control in place");
	await page.getByRole("button", { name: "Submit Risk Acceptance" }).click();
	await expect(
		collectionPanel.getByText("risk-accepted: Compensating control in place"),
	).toBeVisible();
});
