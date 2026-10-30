Maincloud is the managed, serverless SpacetimeDB platform run by Clockwork Labs. You publish a module with `spacetime publish my-database --server maincloud`, and clients connect to `https://maincloud.spacetimedb.com`. Stargate is an independent desktop client that talks to that same host, so you can look inside a Maincloud database without writing a client or typing CLI commands.

## Connect Stargate to Maincloud

1. Log in with the spacetime CLI if you have not yet: `spacetime login`. It opens a browser where you sign in with GitHub or Google, and links your CLI identity to your Maincloud account.
2. [Download Stargate](/#top) and open it.
3. Under **Server**, choose **maincloud** from the list. Stargate reads the servers configured in your spacetime CLI, and Maincloud is one of them by default.
4. Under **Authentication**, keep **Use CLI credentials** ticked. If you do not use the CLI on this machine, untick it and either click **Login via Browser** (spacetimedb.com with GitHub sign-in) or paste a token.
5. Click **Connect** and pick your database from the list.

Tick **Remember login** to save the connection. Saved logins appear on the start screen, so switching between a Maincloud production database and a [local or self-hosted](/guides/self-hosted-spacetimedb/) development database takes one click.

## What you can do with a Maincloud database

- **Browse tables.** Page through rows, filter them, and export the result to CSV or JSON.
- **Read the schema.** Tables, columns, reducers, indexes, and constraints, as the host reports them.
- **Run SQL.** Ad-hoc queries with results in a table and CSV or JSON export.
- **Follow logs.** Recent lines first, then new lines as they arrive, with levels highlighted.
- **Watch live data.** Subscribe to a table over WebSocket and see inserts and deletes as players act.
- **Check scheduled reducers.** See which scheduled tasks exist and when they run next.
- **Call reducers.** Fill in typed arguments and call a reducer. This needs read-write mode.

## Production safety

Every connection starts read-only. Stargate does not call mutating reducers or run write SQL until you switch the connection to read-write, which needs a [license](/#pricing). That makes it safe to open a live Maincloud database during an incident or a support request, look at real rows, and close it again without a chance of changing anything by accident.

## Stargate and the Maincloud web dashboard

The dashboard on spacetimedb.com is the official place to manage a Maincloud database. It shows status, table and reducer counts, energy usage, a usage breakdown, real-time logs, a SQL console, and SpacetimeAuth settings. Use it for billing, usage, and auth configuration.

Stargate is built for day-to-day development work inside the data:

| Task | Maincloud dashboard | Stargate |
| --- | --- | --- |
| Usage, energy, and auth settings | Yes | No |
| Logs | Yes | Yes |
| SQL console | Yes | Yes, with CSV and JSON export |
| Table browser with filters | | Yes |
| Reducer console with typed arguments | | Yes |
| Live view of inserts and deletes | | Yes |
| Scheduled tasks | | Yes |
| OpenAPI 3.1 export | | Yes |
| Same tool for local and self-hosted servers | | Yes |

The two work well together: the dashboard for account-level settings, and Stargate open next to the game or app while you build.
