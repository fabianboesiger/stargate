---
title: "Self-host SpacetimeDB and connect a GUI (Docker, spacetime start)"
description: >-
  Run SpacetimeDB with spacetime start or Docker, then browse tables, run SQL, and follow logs from a desktop GUI. Covers localhost, nginx, and SSH tunnels.
h1: Self-host SpacetimeDB and connect a GUI
lede: >-
  To inspect a self-hosted SpacetimeDB server, start it with spacetime start or the
  clockworklabs/spacetime Docker image, then point Stargate at its URL (usually
  http://localhost:3000) and sign in with your CLI credentials or a token. For a remote
  server behind the nginx config from the official guide, connect through an SSH tunnel.
card_title: Self-hosted SpacetimeDB
summary: Run SpacetimeDB locally, in Docker, or on your own server, and connect Stargate to it.
icon: server
order: 1
faq_group: self-hosted
permalink: /guides/self-hosted-spacetimedb/
last_modified_at: 2026-10-30
---

{% include guides/self-hosted-spacetimedb.md %}
