SpacetimeDB can run anywhere you can run a single binary or container. Self-hosting is common for local development, for CI, and for teams that want their game or app backend on their own infrastructure instead of on [Maincloud](/guides/maincloud-gui/). This guide shows how to start a self-hosted instance and how to inspect it with Stargate, a desktop GUI client for SpacetimeDB.

## 1. Start a SpacetimeDB server

You have two common options. Both serve the HTTP and WebSocket API on port 3000 by default.

**With the spacetime CLI.** Install the CLI and start a local server:

```sh
# macOS and Linux
curl -sSf https://install.spacetimedb.com | sh

# start a local server
spacetime start
```

On Windows, install the CLI from PowerShell with `iwr https://windows.spacetimedb.com -useb | iex`, then run `spacetime start`.

**With Docker.** The official image runs the same server in a container:

```sh
docker run --rm --pull always -p 3000:3000 clockworklabs/spacetime start
```

Add a volume if you want data to survive container restarts. Then publish a module to it as usual, for example `spacetime publish my-database --server local`.

## 2. Connect Stargate to the server

1. [Download Stargate](/#top) for macOS, Windows, or Linux and open it.
2. Under **Server**, pick the server from the list. Stargate reads the servers you configured in the spacetime CLI (`~/.config/spacetime/cli.toml` on macOS and Linux, `%LocalAppData%\SpacetimeDB\config\cli.toml` on Windows). For a server that is not in that file, tick **Custom server** and enter its URL, for example `http://localhost:3000`.
3. Under **Authentication**, keep **Use CLI credentials** ticked to reuse the token from `spacetime login`, or paste a SpacetimeDB token.
4. Click **Connect** and choose a database from the list of databases owned by your identity.

Stargate opens read-only. You can browse tables, read the schema, run read SQL, follow logs, and open the live view straight away. Calling mutating reducers and running write SQL needs [read-write mode](/#pricing), which you turn on deliberately.

## 3. Connect to a remote self-hosted server

The [official self-hosting guide](https://spacetimedb.com/docs/how-to/deploy/self-hosting) runs SpacetimeDB on `127.0.0.1:3000` behind nginx with HTTPS. Register the server in your CLI with `spacetime server add self-hosted --url https://your-domain.example`, and it shows up in Stargate's server list.

That nginx config is deliberately strict: remote clients may only reach `/v1/identity` and the subscribe endpoint, and every other route is limited to localhost. Stargate also uses the schema, SQL, logs, and reducer call routes, so through that proxy those views stay empty. You have two safe options:

- **SSH tunnel (recommended).** Forward the server's local port to your machine and connect Stargate to `http://localhost:3000`:

  ```sh
  ssh -L 3000:localhost:3000 you@your-server
  ```

  Nothing new is exposed to the internet, and Stargate behaves as if the server ran locally.

- **Allow trusted IPs.** Add `allow <your-ip>;` to the restricted `location` blocks in nginx for the routes you need. Keep the default deny for everyone else, since those routes include SQL and reducer calls.

## Which routes Stargate uses

Stargate is a pure client of the SpacetimeDB HTTP and WebSocket API. There is no agent or extra service to install on the server. It calls:

| Feature | Route |
| --- | --- |
| Health check | `GET /v1/health` |
| Database list | `/v1/identity/{identity}/databases` |
| Schema inspector | `/v1/database/{name}/schema` |
| Table browser and SQL console | `/v1/database/{name}/sql` |
| Logs | `/v1/database/{name}/logs` |
| Reducer console | `/v1/database/{name}/call/{reducer}` |
| Live view | `/v1/database/{name}/subscribe` (WebSocket) |

Use this list when you write proxy or firewall rules for a self-hosted instance.

## Troubleshooting

- **Connection failed.** Check that the URL includes the scheme (`http://` or `https://`) and the port, and that `curl http://localhost:3000/v1/health` answers from the same machine.
- **No databases found for this identity.** The token belongs to a different identity than the one that published the database. Sign in with the same CLI login you used for `spacetime publish`, or paste that identity's token.
- **Tables load but logs or SQL do not.** A reverse proxy is blocking those routes. Use the SSH tunnel above.
