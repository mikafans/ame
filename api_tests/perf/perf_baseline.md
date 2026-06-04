# AME Performance Baseline

This document tracks the performance baseline for both micro-benchmarks (Criterion) and black-box concurrent load tests (`oha`). Use these values as a point of reference to detect regressions or verify performance improvements.

Last updated: 2026-06-04
Environment: Local Linux (Tokio / Postgres 18 / Valkey)

---

## 1. Micro-Benchmarks (Criterion)

These measure pure-function execution time on CPU, bypassing the network and database layers. Run with:
```bash
make bench
```

| Benchmark Path | Metric / Operation | Baseline (Mean) | Description |
| :--- | :--- | :--- | :--- |
| `token_verify/match` | Token verification (match) | **~171.5 ns** | Verified API token signature decryption & match |
| `token_verify/mismatch` | Token verification (mismatch) | **~175.0 ns** | Time-constant mismatch detection |
| `grade_response_mc` | Multiple Choice grading | **~228.3 ns** | Grader validation and correct option matching |
| `update_elo` | ELO rating update | **~47.5 ns** | Mathematical recalculation of user tag ratings |

---

## 2. Load Tests (oha)

These measure concurrency, database contention, and network overhead. Run with:
```bash
# Start API server in dev mode
make dev
# Run oha load test (in another terminal)
make bench-load
```

### A. Authenticated Reads (`GET /v1/me`)
* **Concurrency**: 20 concurrent connections (VUs)
* **Total Requests**: 1,000
* **Success Rate**: 100.00%
* **Throughput**: **~10,521 requests/sec**
* **Latencies**:
  * Average: **1.15 ms** (or **~8.50 ms** under full saturation)
  * Median (p50): **3.19 ms**
  * p95: **3.61 ms**

### B. Session Creation Writes (`POST /v1/sessions`)
* **Concurrency**: 10 concurrent connections (VUs)
* **Total Requests**: 500
* **Success Rate**: 100.00%
* **Throughput**: **~310.7 requests/sec**
* **Latencies**:
  * Average: **31.89 ms**
  * Fastest: **8.08 ms**
  * Slowest: **37.50 ms**

### C. Submitting Answers (`POST /v1/sessions/{id}/answer`)
* **Concurrency**: 10 concurrent connections (VUs)
* **Total Requests**: 500
* **Success Rate**: **99.60%** (2 conflict errors expected under parallel write contention on the same session slot)
* **Throughput**: **~310.0 requests/sec**
* **Latencies**:
  * Average: **32.82 ms**
  * Median (p50): **32.83 ms**
  * p95: **36.11 ms**

---

## 3. Detecting Regressions

When submitting pull requests:
1. Run `make bench` and verify that the change in mean execution time is within the noise threshold (not flagged as a regression by Criterion).
2. Run `make bench-load` and verify that throughput does not significantly deteriorate (e.g. less than 10% drop in throughput) and error rates remain at 0.00% (or very low for write conflicts).
