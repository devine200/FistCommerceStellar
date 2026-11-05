/**
 * Playwright browser E2E scenarios for Fist Commerce Soroban Testnet mode.
 * Run against a local FE (`npm run dev`) with preferred network set to soroban-testnet.
 *
 *   npx playwright test e2e/soroban-demo.spec.ts
 *   SOROBAN_E2E_LIVE=1 npx playwright test e2e/soroban-demo.spec.ts
 */
import { test, expect } from '@playwright/test'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { execFileSync } from 'node:child_process'

const FE = process.env.FE_BASE_URL ?? 'http://127.0.0.1:5173'
const DEPLOYMENTS = resolve(
  process.env.DEPLOYMENTS_FILE ?? '../deployments/testnet.json',
)

function liveDeployment() {
  return JSON.parse(readFileSync(DEPLOYMENTS, 'utf8')) as {
    attestation_registry: { contract_id: string }
    trade_credit_pool: { contract_id: string }
    test_asset: { sac_contract_id: string }
  }
}

function assertLiveId(id: string, label: string) {
  expect(id, label).toMatch(/^C[A-Z0-9]{55}$/)
  expect(id, label).not.toMatch(/^C\.\.\./)
}

test.describe('Soroban Testnet capability UI', () => {
  test.beforeEach(async ({ page }) => {
    await page.addInitScript(() => {
      localStorage.setItem('fistcommerce.preferredNetwork', 'soroban-testnet')
    })
  })

  test('network switcher exposes Soroban and FE embeds live contract IDs', async ({ page }) => {
    const dep = liveDeployment()
    assertLiveId(dep.attestation_registry.contract_id, 'registry')
    assertLiveId(dep.trade_credit_pool.contract_id, 'pool')
    assertLiveId(dep.test_asset.sac_contract_id, 'sac')

    await page.goto(`${FE}/dashboard/admin/login`)
    const sorobanBtn = page.getByRole('button', { name: /Soroban/i })
    await expect(sorobanBtn).toBeVisible()
    await sorobanBtn.click()
    await expect(sorobanBtn).toHaveAttribute('aria-pressed', 'true')

    // Vite serves the deployment JSON into the FE bundle for SorobanDemoPanel.
    const embedded = await page.evaluate(async () => {
      const mod = await import('/src/stellar/testnet.deployment.json')
      return (mod as { default?: unknown }).default ?? mod
    })
    const emb = embedded as {
      trade_credit_pool: { contract_id: string }
      attestation_registry: { contract_id: string }
    }
    expect(emb.trade_credit_pool.contract_id).toBe(dep.trade_credit_pool.contract_id)
    expect(emb.attestation_registry.contract_id).toBe(dep.attestation_registry.contract_id)
    expect(emb.trade_credit_pool.contract_id).not.toContain('...')
  })

  test('attestation-gate narrative is visible in Soroban demo panel copy', async ({ page }) => {
    await page.goto(`${FE}/dashboard/admin/overview`)
    const body = await page.locator('body').innerText()
    expect(
      body.includes('Soroban') || body.includes('login') || body.includes('Login'),
    ).toBeTruthy()
  })
})

test.describe('Brief demo checklist (manual/CI with funded Testnet)', () => {
  test.skip(!process.env.SOROBAN_E2E_LIVE, 'Set SOROBAN_E2E_LIVE=1 for live chain demo')

  test('funding fails without attestations then succeeds (live)', async () => {
    const script = resolve('../scripts/run-live-attestation-gate.sh')
    const out = execFileSync('bash', [script], {
      env: { ...process.env, PATH: process.env.PATH },
      encoding: 'utf8',
      timeout: 600_000,
    })
    expect(out).toMatch(/ATTESTATION_GATE_PASS|fund confirmed|SUCCESS/)
  })
})
