# Native HTTP/2 Download Optimization Plan

## Scope

- Optimize only the native (`Legacy`) download engine and its shared HTTP/2 path.
- Prioritize large Modrinth modpack throughput and end-to-end install time.
- Keep XMCL behavior unchanged.
- Avoid adding new responsibilities to `packages/app-lib/src/util/fetch.rs`; new reusable transport logic should live under `packages/app-lib/src/util/download/`.

## Current Diagnosis

The first problem is HTTP/2 receive-flow correctness, not the configured concurrency limit.

- Shared H2 connections use a 1 MiB initial receive window per stream and a 64 MiB target connection window.
- The H2 receive loops consume DATA frames without returning capacity through `RecvStream::flow_control().release_capacity(...)`.
- A file larger than the effective stream window can stall, hit the 30-second receive timeout, restart through Range or a reqwest stream, and put the authority into the 10-minute HTTP/1.1 fallback state.
- This prevents H2 from accumulating representative successful throughput samples, biases transport reputation toward HTTP/1.1 Range, and causes duplicate network and disk work.

After flow control is fixed, the next likely native-engine constraints are:

- General H2 files consume one native authority permit per logical stream even though they share one physical connection, limiting one authority to eight streams.
- H2 reports every DATA frame to the install reporter before its internal throttling, causing unnecessary mutex acquisition and state work.
- Successful H2 network transfers return `attempts = 0`, so modpack installation can classify them as recovered files.
- General H2 bytes and active streams are omitted from automatic concurrency sampling.
- Cold 4-16 MiB files require an already-live H2 connection and therefore often choose Range at batch startup.
- HTTP/1.1 Range writes segment files and then rereads and rewrites the complete payload during merge.
- Modpack override extraction and parts of install-state persistence can become bottlenecks once network throughput improves.

## P1: Correctness And Hot-Path Cost

### Changes

1. Return receive capacity after each H2 DATA frame has been consumed.
2. Apply the same fix to unknown-size probe draining and Minecraft asset batch streams.
3. Return capacity after writing and hashing, preserving local disk backpressure.
4. Throttle H2 install progress before entering the shared reporter using the existing native cadence: `max(256 KiB, total_size / 200)`, plus a mandatory final update.
5. Record a real network attempt for successful H2 transfers.
6. Include H2 activity and transferred bytes in native automatic-concurrency observations.
7. Keep the H2 fallback, integrity validation, JAR validation, and atomic finalization semantics unchanged.

### Structure

- Put new reusable H2 receive/progress accounting in a focused module under `packages/app-lib/src/util/download/`.
- Keep `h2_download.rs` responsible for H2 request and file-transfer orchestration.
- Do not add new helpers or state to `fetch.rs` unless an existing API must be called.

### Tests

- Add a regression transfer larger than the 1 MiB initial stream window.
- Cover at least a multi-megabyte successful H2 body with expected hash and size.
- Verify the transfer finishes without fallback and produces the exact destination bytes.
- Verify capacity-return errors are surfaced rather than ignored where practical.
- Preserve existing Range and fallback tests.

### Acceptance Criteria

- A 64 MiB custom-H2 transfer can complete without stalling near 1 MiB or entering HTTP/1.1 fallback.
- Concurrent files reuse the shared physical connection.
- Progress-reporter calls for a large file fall by at least one order of magnitude relative to per-frame reporting.
- H2 network completions are not classified as recovered cache hits.
- Automatic concurrency metrics include H2 bytes and active transfers.
- No unbounded receive buffering appears with a throttled writer.

## P2: Separate H2 Stream And Connection Budgets

Implemented. The initial limits are 128 streams globally and 32 streams per
authority. These are intentionally conservative starting values for later
benchmarking against 8 and 16 stream variants.

1. Separate physical connection permits from logical H2 stream permits. Done.
2. Keep one shared physical H2 connection per authority initially. Done.
3. Apply the stream budget to general H2 files and native asset-batch streams. Done.
4. Retain file-task and disk-write limits so increased multiplexing does not create uncontrolled I/O. Done.
5. Release the physical connection permit when the H2 driver exits. Done.
6. Benchmark per-authority stream limits of 8, 16, and 32. Pending.
7. Respect peer `SETTINGS_MAX_CONCURRENT_STREAMS` through H2 sender readiness and expose queue time in diagnostics. Pending.

Acceptance criteria:

- Higher stream limits improve same-authority modpack throughput without unacceptable CPU, memory, file-handle, or disk-queue growth.
- Slow-disk and low-bandwidth profiles do not regress materially.

## P3: Improve Cold H2 Adoption And Routing

Implemented in this pass:

1. Permit 4-16 MiB files to establish a cold H2 connection.
2. Use the existing per-authority connection slot to serialize the first handshake and let concurrent files reuse it.
3. Emit debug logs for H2 connection establishment and reuse.
4. Keep large-file H2 selection restricted to a warm connection plus existing transport-reputation thresholds.

Remaining work:

1. Measure waiting for H2 establishment against immediately using Range.
2. Record transport selection and throughput by file-size bucket.
3. Avoid custom H2 for known redirect-only routes, or add bounded redirect handling without forwarding sensitive headers across authorities.
4. Evaluate the official Modrinth H2 route when the first mirror route is not H2-suitable.

## P4: Reduce Range Disk Amplification

Implement only if post-P1/P2 profiles show Range merge I/O is material.

1. Preallocate the final `.part` file.
2. Write validated ranges directly at non-overlapping offsets.
3. Hash the completed `.part` once before content validation and finalization.
4. Preserve cancellation, cleanup, retry, resume, and Windows sharing semantics.

Target I/O reduction:

- Current: approximately `N` segment writes + `N` merge reads + `N` final writes.
- Proposed: approximately `N` positional writes + `N` verification reads.

## P5: Modpack Pipeline Optimization

1. Combine managed-override cache hashing with actual extraction to avoid duplicate decompression.
2. Benchmark controlled override extraction concurrency of 2-4 workers.
3. Consolidate per-item and aggregate progress sampling where both update the same reporter state.
4. Reduce full install-job JSON serialization and snapshot IPC on large jobs, favoring incremental summaries and updates.
5. Audit legacy pack formats for the same Minecraft-install overlap already used by `.mrpack` installation.

## Benchmark Matrix

Use a deterministic TLS/H2 server with configurable stream limits, Range support, redirects, stalls, resets, corruption, latency, bandwidth, and loss.

Synthetic modpack corpus:

| Count | File size | Representative workload |
| ---: | ---: | --- |
| 600 | 128 KiB | Small resources/configuration |
| 250 | 1 MiB | Small mods |
| 80 | 8 MiB | Typical mods |
| 12 | 32 MiB | Large mods |
| 2 | 128 MiB | Very large content |

Network profiles:

| Profile | RTT | Bandwidth | Loss |
| --- | ---: | ---: | ---: |
| Loopback | <1 ms | Unlimited | 0% |
| Fast broadband | 10 ms | 500 Mbps | 0% |
| Typical | 40 ms | 100 Mbps | 0% |
| High latency | 150 ms | 100 Mbps | 0% |
| Mild loss | 40 ms | 100 Mbps | 0.1% |
| Mobile-like | 80 ms | 30 Mbps | 0.5% |
| H2 HOL stress | 100 ms | 100 Mbps | 1% |

Metrics:

- End-to-end install and download wall time.
- Time to first completed file and p50/p95/p99 file completion.
- Throughput by file-size bucket and selected transport.
- Physical TCP/TLS connections and peak H2 streams per authority.
- H2 fallback reasons, restarts, and redownloaded bytes.
- Range segment write, merge-read, and final-write bytes.
- Hash, JAR validation, override extraction, and finalization time.
- Reporter mutex wait/hold time, progress calls, JSON serialization, SQLite writes, and Tauri IPC payloads.
- Peak memory, task count, file handles, and disk queue depth.

## Non-Goals

- Do not modify or tune the XMCL engine.
- Do not raise concurrency before fixing and testing H2 flow control.
- Do not add multiple H2 connections per authority without high-BDP or packet-loss evidence.
- Do not weaken hash or archive validation to match PCL-CE throughput.
- Do not replace bounded resource management with PCL-CE-style unbounded cross-loader oversubscription.
