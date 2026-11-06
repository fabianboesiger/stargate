This comparison is written by the author of Stargate. Every claim about another tool comes from that tool's own documentation and was checked on the date at the top of this page, against Spacetime Studio v1.0.0 and Stargate v0.1.0. If something here is out of date, [open an issue]({{ site.links.github }}/issues).

## The four options

- **The spacetime CLI** is the official command-line tool. It is the only one of the four that builds and publishes modules, and it works against any host.
- **The Maincloud dashboard** on spacetimedb.com is the official web interface for databases on Maincloud. It covers status, usage, energy, logs, a SQL console, and SpacetimeAuth settings.
- **[Spacetime Studio](https://spacetimestudio.dev)** is an open-source (MIT) desktop admin client built with Tauri and Vue, focused on browsing and editing table data.
- **Stargate** is a source-available desktop client written in Rust with Dioxus, focused on inspecting and debugging live databases, with a free read-only mode and a paid license for writes.

## Feature comparison

| | spacetime CLI | Maincloud dashboard | Spacetime Studio | Stargate |
| --- | --- | --- | --- | --- |
| Runs as | Terminal command | Web page | Desktop app | Desktop app |
| Price | Free | Included with Maincloud | Free, MIT license | Free to read; per-device license for writes |
| Hosts | Any | Maincloud only | Local, Maincloud, self-hosted | Local, Maincloud, self-hosted |
| Reuses the CLI login | Yes | Not applicable | Yes | Yes |
| Token storage | `cli.toml` | Browser session | OS keychain | Local app database |
| Read-only safety | No | Not documented | Per connection, opt in | Default for every connection |
| Browse tables | Through `spacetime sql` | Through the SQL console | Paginated, total row count | Paginated, filter, live updates |
| Edit rows | Through SQL | Not documented | Inline cell editing | Insert, edit, delete (read-write) |
| SQL console | `spacetime sql` | Yes | History, multi-statement | History |
| CSV and JSON export | No | Not documented | Yes | Yes |
| Call reducers | `spacetime call` | Not documented | Yes | Typed fields, confirmation step |
| Logs | `spacetime logs -f` | Real-time | Live, filter by level or text | Live, filter by text |
| Live subscription view | `spacetime subscribe` | Not documented | Not documented | Inserts and deletes, record to file |
| Scheduled tasks | Through SQL | Not documented | Schedule tables labelled | Scheduled view, stop a task |
| OpenAPI export | No | Not documented | Not documented | OpenAPI 3.1 |
| Usage and billing | No | Yes | No | No |
| Publish modules | Yes | No | No | No |

"Not documented" means the feature does not appear in that tool's documentation; it may still exist.

## Which one to use

- **Building, publishing, scripting, and CI:** the spacetime CLI. None of the GUIs replace `spacetime publish` or `spacetime generate`.
- **Maincloud account, usage, and auth settings:** the Maincloud dashboard. It is the only place for energy usage and SpacetimeAuth.
- **Editing table data for free:** Spacetime Studio. Inline cell editing and writes are free, and tokens live in the OS keychain.
- **Debugging live state and inspecting production:** Stargate. The Live view streams every insert and delete and can record them to a file, tables update live while you look at them, scheduled reducers have their own view, and every connection starts read-only, so opening a production database cannot change it by accident.

The tools combine well: the CLI for publishing, plus a desktop client for looking inside the data. For the details of each workflow in Stargate, see [the spacetime CLI mapping](/guides/spacetime-cli-alternative/), [Maincloud](/guides/maincloud-gui/), and [debugging a module](/guides/debug-spacetimedb/).
