# Stargate — a desktop GUI client for SpacetimeDB

**Website: [stargate-client.com](https://stargate-client.com/) · [Download the latest release](https://github.com/fabianboesiger/stargate/releases/latest)**

Stargate is a native desktop **GUI client for [SpacetimeDB](https://spacetimedb.com)**. It connects
to both **Maincloud** (the managed SpacetimeDB cloud) and **self-hosted** instances, and lets you
browse tables, inspect the schema, call reducers, stream logs, run SQL, watch live data over
WebSocket, and manage scheduled tasks — all from one window, without memorising `spacetime` CLI
commands.

It runs on **macOS, Windows, and Linux**, and talks to SpacetimeDB over its HTTP and WebSocket API,
so it works regardless of whether your module is written in **Rust, C#, TypeScript, or C++**, and
alongside any client (Unity, Unreal, web, or custom).

[![Download](https://img.shields.io/github/v/release/fabianboesiger/stargate?label=download&sort=semver)](https://github.com/fabianboesiger/stargate/releases/latest)
![Platforms](https://img.shields.io/badge/platforms-macOS%20%7C%20Windows%20%7C%20Linux-555)
![Built with Rust](https://img.shields.io/badge/built%20with-Rust%20%2B%20Dioxus-orange)

![Stargate showing a SpacetimeDB database](site/images/screenshot-light.png)

## Features

- **Table browser** — page through tables, filter rows, and export to CSV or JSON.
- **Schema inspector** — view tables, reducers, indexes, and constraints, kept in sync with the database.
- **Reducer console** — call any reducer by name with typed arguments (write calls need a read-write license).
- **Live logs** — tail recent log lines or follow them live, with levels highlighted.
- **SQL console** — run SQL against the database, view results in a table, and export them.
- **Live view** — subscribe over WebSocket and watch rows update live; useful for debugging multiplayer state.
- **Scheduled tasks** — see which reducers are scheduled to run, and when.
- **OpenAPI export** — generate an OpenAPI 3.1 document for the connected database.
- **Saved connections** — save servers and databases, then reconnect in a click.
- **Read-only by default** — writes only happen when you turn them on, so you don't change production by accident.
- **Light and dark themes.**

## Use cases

- Debug multiplayer game state by watching tables and the live view update as players act.
- Inspect a SpacetimeDB database without the `spacetime` CLI.
- Test reducers by hand with typed arguments to reproduce a bug or seed data.
- Read and follow module logs while developing.
- Run ad-hoc SQL and export results to CSV or JSON.
- Audit a production database in read-only mode with no risk of changing it.

## Install

Download a build for your platform from the
[**latest release**](https://github.com/fabianboesiger/stargate/releases/latest):

| Platform | Asset |
| --- | --- |
| macOS (Apple Silicon) | `stargate-macos-aarch64.tar.gz` |
| Windows (x86-64) | `stargate-windows-x86_64.zip` |
| Linux (x86-64) | `stargate-linux-x86_64.tar.gz` |

## Connect

Sign in with your SpacetimeDB CLI credentials or a token, choose a server and database, and connect:

- **Maincloud** — sign in with your existing Maincloud login.
- **Self-hosted** — enter the URL of your server (a server started with `spacetime start`, a Docker
  container, or a production host you run yourself).
- **Local** — point Stargate at `http://localhost:3000` while you develop.

Credentials and saved logins stay on your machine.

## Pricing

Reading is free, with every inspection and query feature. A per-device license (priced in CHF)
unlocks read-write mode for mutating reducers and write SQL. See
[stargate-client.com](https://stargate-client.com/#pricing).

## Build from source

Stargate is built with [Rust](https://www.rust-lang.org/) and [Dioxus](https://dioxuslabs.com/).

```sh
# Install the Dioxus CLI, then:
dx build --release --platform desktop --package stargate
```

## Links

- Website: https://stargate-client.com/
- Releases / download: https://github.com/fabianboesiger/stargate/releases/latest
- SpacetimeDB: https://spacetimedb.com

---

Built by [Fabian Bösiger](https://github.com/fabianboesiger). Not affiliated with SpacetimeDB or
Clockwork Labs.
