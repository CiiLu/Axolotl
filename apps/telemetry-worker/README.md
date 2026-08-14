# Axolotl telemetry worker

This Worker accepts only opted-in launcher heartbeat and error batches. It stores anonymous metadata in D1 and bounded, sanitized error context in R2.

## Provisioning

```sh
pnpm exec wrangler d1 create axolotl-telemetry
pnpm exec wrangler r2 bucket create axolotl-telemetry-errors
pnpm exec wrangler secret put INSTALLATION_HMAC_SECRET
pnpm exec wrangler d1 migrations apply axolotl-telemetry --remote
pnpm exec wrangler r2 bucket lifecycle add axolotl-telemetry-errors telemetry-errors-30d errors/ --expire-days 30
```

Replace `database_id` in `wrangler.toml` and keep the Worker on the Free plan. The production custom domain is declared in `wrangler.toml`. Configure Cloudflare usage notifications at 50%, 75%, and 90% for Workers, D1, and R2. Do not enable Workers Paid.

`STORE_ERROR_CONTEXT=false` stops all R2 writes while heartbeat and error aggregates continue. The code clamps R2 creation to 2,000 objects per UTC day, three samples per fingerprint/version/day, and 16 KiB uncompressed per object even if environment variables request larger limits.
