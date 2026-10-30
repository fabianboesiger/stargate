The spacetime CLI is the official way to create, build, and publish SpacetimeDB modules, and it also has commands for looking inside a running database. Those inspection commands work well in scripts, but they print plain text, take positional arguments that are easy to mistype, and do not let you scroll, filter, or compare results. Stargate puts the same operations in a native desktop app for macOS, Windows, and Linux.

## Command to view mapping

| spacetime CLI | What it does | In Stargate |
| --- | --- | --- |
| `spacetime describe [DATABASE]` | Print the schema | **Schema**: tables, columns, reducers, indexes, and constraints |
| `spacetime sql [DATABASE] "SELECT * FROM player"` | Run a SQL query | **Tables** to browse and filter without SQL, **SQL** for ad-hoc queries, both with CSV and JSON export |
| `spacetime call [DATABASE] <REDUCER> [ARGS...]` | Call a reducer | **Reducers**: pick a reducer, fill typed fields, confirm, and call |
| `spacetime logs -f [DATABASE]` | Print and follow module logs | **Logs**: recent lines, then a live stream with levels highlighted |
| `spacetime subscribe [DATABASE] <QUERY>` | Print subscription updates | **Live**: inserts and deletes per table, with pause, clear, and record to file |
| `spacetime server list` | List configured servers | **Server** list on the connect screen, read from the same `cli.toml` |

Stargate also has views without a direct CLI equivalent: **Scheduled** lists the rows of schedule tables and when each task runs, **Info** shows the server URL, SpacetimeDB version, identity, and table and reducer counts, and the OpenAPI export generates an OpenAPI 3.1 document for the connected database.

## What still needs the CLI

Stargate is a client for databases that already exist. Keep using the CLI for:

- `spacetime init`, `spacetime build`, and `spacetime publish` to create and deploy modules.
- `spacetime generate` to generate client bindings for Unity, Unreal, TypeScript, Rust, or C#.
- `spacetime login`, `spacetime server add`, and other configuration. Stargate reads the result from `cli.toml`, so a server added in the CLI appears in Stargate.

## Browse tables without writing SQL

In the terminal, seeing what is in a table means writing a query such as `spacetime sql my-game "SELECT * FROM player"` and reading wrapped text output. In Stargate you click the table. Rows are paged, the **Filter rows** box narrows them as you type, and the export action saves the current rows as CSV or JSON through the native save dialog. When you do want SQL (joins, counts, `WHERE` clauses), open the **SQL** view.

## Call reducers with typed arguments

`spacetime call` takes arguments as JSON on the command line, which gets awkward for structs, enums, and identities. Stargate reads each reducer's parameter types from the schema and shows one field per argument, labelled with its type. A confirmation step shows the call before it runs. Calling reducers requires read-write mode; on a read-only connection the **Execute** button stays disabled.

## Read-only by default

The CLI runs whatever you type against whatever server is the default. Stargate starts every connection read-only and only allows mutating reducers and write SQL after you switch to read-write mode. That is the main reason to prefer a GUI when you look at a production database: you can explore freely without risking a write.

## Next steps

- [Self-host SpacetimeDB and connect a GUI](/guides/self-hosted-spacetimedb/)
- [Debug a SpacetimeDB module with logs, reducers, and live data](/guides/debug-spacetimedb/)
