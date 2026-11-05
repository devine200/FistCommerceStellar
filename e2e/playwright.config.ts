import { defineConfig } from '@playwright/test'

export default defineConfig({
  testDir: '.',
  timeout: 120_000,
  retries: 0,
  use: {
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
  },
  reporter: [['list'], ['html', { open: 'never', outputFolder: 'results/html' }]],
  outputDir: 'results/test-output',
})
