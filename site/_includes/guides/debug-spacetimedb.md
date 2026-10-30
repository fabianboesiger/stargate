In SpacetimeDB, game and app logic runs inside reducers on the server, and clients only see the rows those reducers write. When something goes wrong (a player teleports, an item duplicates, a scheduled task never fires), the question is always the same: which reducer wrote what, and when? This workflow answers it without adding print statements to a client or writing a throwaway test harness.

## 1. Stream the module logs

Open **Logs**. Stargate loads the most recent lines and then keeps streaming new ones; the badge in the header reads **Live** while the stream is connected. Levels are color-coded (ERROR, WARN, INFO, DEBUG, TRACE), so panics and failed reducer calls stand out.

Log from your reducers with the logging macros or functions of your module language (for example `log::info!` in Rust or `Log.Info` in C#). Each line includes the reducer's output, so you can see the arguments a reducer received and the branch it took.

## 2. Watch rows change in the Live view

Open **Live** and subscribe to the tables involved. Every change arrives as an event with a timestamp, the table, the row count, and a preview of the row:

- **INIT** for the initial rows of the subscription,
- **INSERT** for new or updated rows,
- **DELETE** for removed or replaced rows.

Play the game or use the app as usual and watch the stream. **Pause** freezes the list while you read, **Clear** empties it, and **Record** writes events to a file so you can compare two runs or attach them to an issue.

Because SpacetimeDB represents an update as a delete followed by an insert, a single reducer call that touches one row shows up as a DELETE and an INSERT with the same timestamp.

## 3. Reproduce the bug by calling the reducer

Once you suspect a reducer, open **Reducers**, filter the list by name, and select it. Stargate shows one input per parameter with its SpacetimeDB type (for example `u64` or `u32`). Fill in the exact values from the log line, confirm, and call. Watch the Live view and Logs to see what it does.

This is faster than rebuilding a client to trigger one code path, and it lets you test edge cases (zero, maximum values, an identity that does not exist) that are hard to reach through gameplay. Calling reducers requires read-write mode, so do this against a local or staging database, not production.

## 4. Check the data and export it

Open **Tables** and the affected table. Use **Filter rows** to find the rows in question, or open **SQL** for a precise query such as:

```sql
SELECT * FROM inventory WHERE owner = 42
```

Export the rows to CSV or JSON from either view. A small export of the broken state plus the recorded Live view events makes a bug report anyone on the team can follow.

## 5. Scheduled reducers

If the bug involves timers (respawns, cooldowns, decay), open **Scheduled**. It lists the rows of every schedule table and when each task runs. Check that the task you expect exists and that its time matches what your code computed, then look in Logs for the scheduled reducer's output when that time comes. In read-write mode you can also stop a scheduled task.

## Debugging production safely

For a live game, connect to the production database in read-only mode (the default). Logs, Live view, Tables, SQL reads, and Scheduled all work, and nothing can be written. Reproduce the bug on a [local or self-hosted copy](/guides/self-hosted-spacetimedb/) with read-write mode on.
