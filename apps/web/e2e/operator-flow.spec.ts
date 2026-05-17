import { expect, test } from "@playwright/test";

test("operator flow registers, scans, and queries one active finding", async ({
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

	await page.getByRole("button", { name: "Request Scan" }).click();
	await expect(page.getByText(/Command: scan-command-/i)).toBeVisible();

	await page.getByRole("button", { name: "Run Worker" }).click();
	await expect(page.getByText(/Processed: 1\./i)).toBeVisible();

	await page.goto("/findings");
	await page.getByRole("textbox", { name: "Package name" }).fill("openssl");
	await page
		.locator("form.filters")
		.evaluate((form) => (form as HTMLFormElement).requestSubmit());

	await expect(page.getByText("Showing 1-1 of 1")).toBeVisible();
	await expect(page.getByRole("cell", { name: "CVE-2026-0001" })).toBeVisible();
	await expect(page.getByRole("cell", { name: "openssl@3.0.0" })).toBeVisible();
});
