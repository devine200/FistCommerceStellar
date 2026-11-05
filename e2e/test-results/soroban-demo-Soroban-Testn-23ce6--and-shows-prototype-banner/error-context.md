# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: soroban-demo.spec.ts >> Soroban Testnet capability UI >> network switcher exposes Soroban and shows prototype banner
- Location: soroban-demo.spec.ts:18:7

# Error details

```
Error: page.goto: net::ERR_CONNECTION_REFUSED at http://127.0.0.1:5173/dashboard/admin/login
Call log:
  - navigating to "http://127.0.0.1:5173/dashboard/admin/login", waiting until "load"

```

# Test source

```ts
  1  | /**
  2  |  * Playwright browser E2E scenarios for Fist Commerce Soroban Testnet mode.
  3  |  * Run against a local FE (`npm run dev`) with preferred network set to soroban-testnet.
  4  |  *
  5  |  *   npx playwright test e2e/soroban-demo.spec.ts
  6  |  */
  7  | import { test, expect } from '@playwright/test'
  8  | 
  9  | const FE = process.env.FE_BASE_URL ?? 'http://127.0.0.1:5173'
  10 | 
  11 | test.describe('Soroban Testnet capability UI', () => {
  12 |   test.beforeEach(async ({ page }) => {
  13 |     await page.addInitScript(() => {
  14 |       localStorage.setItem('fistcommerce.preferredNetwork', 'soroban-testnet')
  15 |     })
  16 |   })
  17 | 
  18 |   test('network switcher exposes Soroban and shows prototype banner', async ({ page }) => {
> 19 |     await page.goto(`${FE}/dashboard/admin/login`)
     |                ^ Error: page.goto: net::ERR_CONNECTION_REFUSED at http://127.0.0.1:5173/dashboard/admin/login
  20 |     // Banner may appear on admin layouts after login; switcher on login
  21 |     const sorobanBtn = page.getByRole('button', { name: /Soroban/i })
  22 |     if (await sorobanBtn.count()) {
  23 |       await sorobanBtn.click()
  24 |       await expect(sorobanBtn).toHaveAttribute('aria-pressed', 'true')
  25 |     }
  26 |   })
  27 | 
  28 |   test('attestation-gate narrative is visible in Soroban demo panel copy', async ({ page }) => {
  29 |     await page.goto(`${FE}/dashboard/admin/overview`)
  30 |     // If redirected to login, still assert switcher preference stuck
  31 |     const body = await page.locator('body').innerText()
  32 |     expect(
  33 |       body.includes('Soroban') || body.includes('login') || body.includes('Login'),
  34 |     ).toBeTruthy()
  35 |   })
  36 | })
  37 | 
  38 | test.describe('Brief demo checklist (manual/CI with funded Testnet)', () => {
  39 |   test.skip(!process.env.SOROBAN_E2E_LIVE, 'Set SOROBAN_E2E_LIVE=1 for live chain demo')
  40 | 
  41 |   test('funding fails without attestations then succeeds (live)', async () => {
  42 |     // Driven by FistCommerceStellar/scripts/run-demo-flow.sh + client
  43 |     expect(true).toBeTruthy()
  44 |   })
  45 | })
  46 | 
```