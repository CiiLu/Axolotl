# Axolotl Telemetry Dashboard

Read-only administration UI for the existing `axolotl-telemetry` D1 database and
`axolotl-telemetry-errors` R2 bucket. It does not share routes or code with the public telemetry
collector.

## Local development

Mock authentication is accepted only when `NODE_ENV=development` and
`TELEMETRY_ADMIN_MOCK_AUTH=true` are both present.

Use the production D1 and R2 resources through Wrangler remote bindings while keeping authentication
local to the development server:

```powershell
$env:TELEMETRY_ADMIN_MOCK_AUTH='true'
$env:TELEMETRY_ADMIN_REMOTE_BINDINGS='true'
pnpm --filter @axolotl/telemetry-dashboard dev
```

Remote mode is read-only at the application layer: the Admin API issues only `SELECT` statements
and a single registered-object R2 `GET`. It never exposes Cloudflare credentials to the browser.

For deterministic fixture scenarios, leave `TELEMETRY_ADMIN_REMOTE_BINDINGS` unset:

```powershell
$env:TELEMETRY_ADMIN_MOCK_AUTH='true'
$env:TELEMETRY_ADMIN_MOCK_SCENARIO='normal'
pnpm --filter @axolotl/telemetry-dashboard dev
```

Supported deterministic scenarios are `normal`, `empty`, `api-error`, `no-sample`,
`budget-reached`, `unconfigured-auth`, and `forbidden`. All mock values are synthetic fixtures.

## Vercel production deployment

Do not deploy the dashboard until every item below is complete:

1. Create a GitHub OAuth App controlled by the `Axolotl-Launcher` organization. Use
   `https://admin.axlmc.org` as its homepage and the callback URL shown by Cloudflare Zero Trust,
   normally `https://<team-name>.cloudflareaccess.com/cdn-cgi/access/callback`.
2. Add GitHub as a Cloudflare Zero Trust identity provider using that Client ID and Client Secret.
   Keep the Client Secret only in GitHub and Cloudflare.
3. Create a self-hosted Access application for `admin.axlmc.org` with an 8-hour session duration.
4. Create one Allow policy with `Include -> GitHub Organization -> Axolotl-Launcher`. Do not add a
   bypass policy or a broader include rule.
5. Link `apps/telemetry-dashboard` to the Vercel project and add the following Production
   environment variables with `vercel env add`. Never prefix Cloudflare credentials with
   `NUXT_PUBLIC_`:
   - `CF_ACCESS_TEAM_DOMAIN`: the team slug from `<team-name>.cloudflareaccess.com`.
   - `CF_ACCESS_AUDIENCE`: the Access application audience (`AUD` tag).
   - `CLOUDFLARE_ACCOUNT_ID`, `CLOUDFLARE_D1_DATABASE_ID`, and `CLOUDFLARE_API_TOKEN`: a token
     limited to read access for the telemetry D1 database.
   - `CLOUDFLARE_R2_ACCESS_KEY_ID`, `CLOUDFLARE_R2_SECRET_ACCESS_KEY`, and
     `CLOUDFLARE_R2_BUCKET_NAME`: read-only S3 credentials limited to the error-context bucket.

6. Deploy from `apps/telemetry-dashboard` with `vercel --prod`. The Vercel build selects Nitro's
   Vercel preset and queries D1 through Cloudflare's server-side REST API. Registered R2 objects are
   read through the server-side S3 API. No credential is included in browser runtime config.
7. Add `admin.axlmc.org` to the Vercel project. In Cloudflare DNS, create a proxied CNAME for
   `admin` pointing to Vercel's assigned CNAME target. Keep the Access application on the hostname;
   do not add a bypass policy.
8. Verify an unauthenticated request is intercepted by Cloudflare Access, then verify
   `/api/admin/session` reports `dataSource: production` after authentication.

The server fails closed when Access or the remote data source configuration is absent. A production
build ignores the mock provider even if a mock environment variable is present. All Admin API D1
operations are restricted to `SELECT`/`WITH` queries, and R2 reads require an object key previously
registered in D1.
