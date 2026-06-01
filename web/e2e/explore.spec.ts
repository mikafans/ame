import { expect, test } from "@playwright/test";

test("explore page loads and shows data with empty filters", async ({
  page,
}) => {
  // Assuming the user is already authenticated via setup
  await page.goto("/explore");

  // Wait for loading to finish
  const loading = page.getByText("Loading...");
  await expect(loading).not.toBeVisible();

  // Verify the table loads (either rows or a "No data" message)
  // If the user reports "no data", let's check for the empty table body
  const tableRows = page.locator("table tbody tr");

  // If the user is correct, we should see an empty message or no rows
  const rowCount = await tableRows.count();
  console.log(`Explore page rows: ${rowCount}`);

  // If count is 0, verify if it's because of the filter or no data
  if (rowCount === 0) {
    await expect(page.getByText(/No assessments/i)).toBeVisible();
  }
});
