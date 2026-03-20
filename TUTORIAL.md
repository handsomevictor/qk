# qk Complete Tutorial

Every feature in this tutorial includes **copy-paste-ready examples** with expected output.

---

## Table of Contents

1. [Installation](#installation)
2. [Preparing Test Data](#preparing-test-data)
3. [Basic Usage](#basic-usage)
4. [Filtering (where)](#filtering-where)
5. [Field Selection (select)](#field-selection-select)
6. [Counting (count)](#counting-count)
7. [Sorting (sort)](#sorting-sort)
8. [Limiting Results (limit / head)](#limiting-results-limit--head)
9. [Numeric Aggregation (sum / avg / min / max)](#numeric-aggregation-sum--avg--min--max)
10. [Field Discovery (fields)](#field-discovery-fields)
11. [DSL Expression Syntax](#dsl-expression-syntax)
12. [DSL Pipeline Stages](#dsl-pipeline-stages)
13. [qk + jq: Handling JSON-Encoded Strings](#qk--jq-handling-json-encoded-strings)
14. [Output Formats (--fmt)](#output-formats---fmt)
15. [Color Output (--color)](#color-output---color)
16. [Multiple File Formats](#multiple-file-formats)
17. [Pipeline Composition](#pipeline-composition)
18. [Common Questions](#common-questions)
19. [Quick Reference](#quick-reference)

---

## Installation

### Step 1: Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
# After installation, reopen your terminal or run:
source ~/.cargo/env
```

### Step 2: Build and Install qk

```bash
git clone https://github.com/handsomevictor/qk.git
cd qk
cargo install --path .
```

Verify the installation:

```bash
qk --version
```

### Using Without Installing During Development

```bash
cargo run -- where level=error app.log
# Equivalent to the installed version:
qk where level=error app.log
```

---

## Preparing Test Data

All examples below use the following files. Create them first:

```bash
cat > app.log << 'EOF'
{"ts":"2024-01-01T10:00:00Z","level":"info","service":"api","msg":"server started","latency":0,"host":"web-01","context":{"region":"us-east","env":"prod","version":"2.4.1"}}
{"ts":"2024-01-01T10:01:00Z","level":"error","service":"api","msg":"connection timeout","latency":3001,"host":"web-01","context":{"region":"us-east","env":"prod","version":"2.4.1"},"request":{"method":"GET","path":"/api/users","ip":"10.0.0.5","headers":{"user-agent":"Mozilla/5.0","x-trace":"abc123"}},"response":{"status":504,"size":0,"error":"upstream timeout"}}
{"ts":"2024-01-01T10:02:00Z","level":"warn","service":"worker","msg":"queue depth high","latency":150,"host":"worker-01","context":{"region":"us-east","env":"prod","version":"1.9.0"},"metrics":{"queue_depth":1842,"consumers":3,"lag_seconds":45}}
{"ts":"2024-01-01T10:03:00Z","level":"info","service":"api","msg":"request ok","latency":42,"host":"web-01","context":{"region":"us-east","env":"prod","version":"2.4.1"},"request":{"method":"POST","path":"/api/orders","ip":"10.0.0.8","headers":{"user-agent":"axios/1.2","x-trace":"def456"}},"response":{"status":201,"size":512,"error":null}}
{"ts":"2024-01-01T10:04:00Z","level":"error","service":"worker","msg":"panic: nil pointer","latency":0,"host":"worker-01","context":{"region":"us-east","env":"prod","version":"1.9.0"},"request":{"method":"POST","path":"/internal/job","ip":"10.0.0.3","headers":{"user-agent":"internal/1.0","x-trace":"ghi789"}},"response":{"status":500,"size":0,"error":"runtime error: invalid memory address"}}
{"ts":"2024-01-01T10:05:00Z","level":"info","service":"web","msg":"page loaded","latency":88,"host":"web-02","context":{"region":"us-west","env":"prod","version":"3.1.0"}}
{"ts":"2024-01-01T10:06:00Z","level":"debug","service":"api","msg":"cache hit","latency":2,"host":"web-01","context":{"region":"us-east","env":"prod","version":"2.4.1"},"request":{"method":"GET","path":"/api/products","ip":"10.0.0.9","headers":{"user-agent":"Go-http-client/1.1","x-trace":"jkl012"}},"response":{"status":200,"size":8192,"error":null}}
{"ts":"2024-01-01T10:07:00Z","level":"error","service":"db","msg":"query timeout","latency":5001,"host":"db-01","context":{"region":"us-east","env":"prod","version":"5.7.0"},"query":{"sql":"SELECT * FROM orders WHERE user_id=?","params":["usr_999"],"table":"orders"},"response":{"status":503,"size":0,"error":"lock wait timeout exceeded"}}
{"ts":"2024-01-01T10:08:00Z","level":"warn","service":"api","msg":"rate limit approaching","latency":5,"host":"web-02","context":{"region":"us-west","env":"prod","version":"2.4.1"},"metrics":{"requests_per_minute":850,"limit":1000,"remaining":150}}
{"ts":"2024-01-01T10:09:00Z","level":"info","service":"auth","msg":"login success","latency":120,"host":"auth-01","context":{"region":"us-east","env":"prod","version":"4.0.2"},"user":{"id":"usr_123","email":"alice@example.com","roles":["admin","editor"]},"request":{"method":"POST","path":"/auth/login","ip":"203.0.113.5","headers":{"user-agent":"Chrome/120","x-trace":"mno345"}}}
{"ts":"2024-01-01T10:10:00Z","level":"error","service":"auth","msg":"login failed: too many attempts","latency":15,"host":"auth-01","context":{"region":"us-east","env":"prod","version":"4.0.2"},"user":{"id":"usr_999","email":"hacker@evil.com","roles":[]},"request":{"method":"POST","path":"/auth/login","ip":"198.51.100.1","headers":{"user-agent":"python-requests/2.28","x-trace":"pqr678"}}}
{"ts":"2024-01-01T10:11:00Z","level":"info","service":"api","msg":"batch job complete","latency":4500,"host":"web-01","context":{"region":"us-east","env":"prod","version":"2.4.1"},"job":{"id":"job_batch_001","type":"export","records_processed":50000,"errors":0}}
{"ts":"2024-01-01T10:12:00Z","level":"warn","service":"db","msg":"slow query detected","latency":2300,"host":"db-01","context":{"region":"us-east","env":"prod","version":"5.7.0"},"query":{"sql":"SELECT COUNT(*) FROM events GROUP BY date","params":[],"table":"events"},"response":{"status":200,"size":128,"error":null}}
{"ts":"2024-01-01T10:13:00Z","level":"info","service":"cache","msg":"eviction triggered","latency":1,"host":"cache-01","context":{"region":"us-east","env":"prod","version":"7.0.0"},"metrics":{"evicted":1204,"memory_mb":7800,"limit_mb":8192,"usage_pct":95.2}}
{"ts":"2024-01-01T10:14:00Z","level":"error","service":"api","msg":"upstream service unavailable","latency":3000,"host":"web-01","context":{"region":"us-east","env":"prod","version":"2.4.1"},"request":{"method":"GET","path":"/api/recommendations","ip":"10.0.0.5","headers":{"user-agent":"Mozilla/5.0","x-trace":"stu901"}},"response":{"status":503,"size":0,"error":"connection refused: ml-service:8080"}}
{"ts":"2024-01-01T10:15:00Z","level":"info","service":"worker","msg":"job processed","latency":380,"host":"worker-02","context":{"region":"us-east","env":"prod","version":"1.9.0"},"job":{"id":"job_001","type":"email","records_processed":1,"errors":0}}
{"ts":"2024-01-01T10:16:00Z","level":"debug","service":"auth","msg":"token validated","latency":3,"host":"auth-01","context":{"region":"us-east","env":"prod","version":"4.0.2"},"user":{"id":"usr_456","email":"bob@example.com","roles":["viewer"]}}
{"ts":"2024-01-01T10:17:00Z","level":"error","service":"api","msg":"invalid request body","latency":8,"host":"web-02","context":{"region":"us-west","env":"prod","version":"2.4.1"},"request":{"method":"POST","path":"/api/users","ip":"10.0.0.7","headers":{"user-agent":"axios/1.2","x-trace":"vwx234"}},"response":{"status":400,"size":64,"error":"JSON parse error at position 42"}}
{"ts":"2024-01-01T10:18:00Z","level":"info","service":"api","msg":"health check","latency":1,"host":"web-01","context":{"region":"us-east","env":"prod","version":"2.4.1"},"response":{"status":200,"size":15,"error":null}}
{"ts":"2024-01-01T10:19:00Z","level":"warn","service":"api","msg":"deprecated endpoint called","latency":33,"host":"web-01","context":{"region":"us-east","env":"prod","version":"2.4.1"},"request":{"method":"GET","path":"/v1/users","ip":"10.0.0.11","headers":{"user-agent":"legacy-client/0.9","x-trace":"yza567"}},"response":{"status":301,"size":0,"error":null}}
{"ts":"2024-01-01T10:20:00Z","level":"info","service":"db","msg":"backup complete","latency":12000,"host":"db-01","context":{"region":"us-east","env":"prod","version":"5.7.0"},"job":{"id":"job_backup_daily","type":"backup","records_processed":5000000,"errors":0}}
{"ts":"2024-01-01T10:21:00Z","level":"error","service":"cache","msg":"replication lag","latency":0,"host":"cache-02","context":{"region":"us-east","env":"prod","version":"7.0.0"},"metrics":{"lag_ms":8400,"primary":"cache-01","replica":"cache-02"}}
{"ts":"2024-01-01T10:22:00Z","level":"info","service":"auth","msg":"password reset","latency":230,"host":"auth-01","context":{"region":"us-east","env":"prod","version":"4.0.2"},"user":{"id":"usr_789","email":"carol@example.com","roles":["editor"]}}
{"ts":"2024-01-01T10:23:00Z","level":"warn","service":"worker","msg":"retry attempt 2 of 3","latency":0,"host":"worker-01","context":{"region":"us-east","env":"prod","version":"1.9.0"},"job":{"id":"job_002","type":"sms","records_processed":0,"errors":1}}
{"ts":"2024-01-01T10:24:00Z","level":"info","service":"api","msg":"graceful shutdown initiated","latency":0,"host":"web-01","context":{"region":"us-east","env":"prod","version":"2.4.1"}}
EOF
```

```bash
cat > access.log << 'EOF'
{"ts":"2024-01-01T10:00:00Z","method":"GET","path":"/api/users","status":200,"latency":42,"client":{"ip":"10.0.0.5","agent":"curl/7.0","country":"US"},"server":{"host":"web-01","region":"us-east","dc":"nyc1"}}
{"ts":"2024-01-01T10:01:00Z","method":"POST","path":"/api/login","status":401,"latency":15,"client":{"ip":"198.51.100.1","agent":"python-requests/2.28","country":"CN"},"server":{"host":"web-01","region":"us-east","dc":"nyc1"}}
{"ts":"2024-01-01T10:02:00Z","method":"GET","path":"/api/orders","status":500,"latency":3200,"client":{"ip":"10.0.0.8","agent":"axios/1.2","country":"US"},"server":{"host":"web-02","region":"us-west","dc":"sfo1"}}
{"ts":"2024-01-01T10:03:00Z","method":"DELETE","path":"/api/cache","status":200,"latency":8,"client":{"ip":"10.0.0.3","agent":"internal/1.0","country":"US"},"server":{"host":"web-01","region":"us-east","dc":"nyc1"}}
{"ts":"2024-01-01T10:04:00Z","method":"GET","path":"/api/users","status":503,"latency":9800,"client":{"ip":"10.0.0.5","agent":"curl/7.0","country":"US"},"server":{"host":"web-02","region":"us-west","dc":"sfo1"}}
{"ts":"2024-01-01T10:05:00Z","method":"GET","path":"/health","status":200,"latency":1,"client":{"ip":"10.0.0.1","agent":"kube-probe/1.27","country":"US"},"server":{"host":"web-01","region":"us-east","dc":"nyc1"}}
{"ts":"2024-01-01T10:06:00Z","method":"POST","path":"/api/orders","status":201,"latency":180,"client":{"ip":"203.0.113.5","agent":"Mozilla/5.0","country":"DE"},"server":{"host":"web-01","region":"us-east","dc":"nyc1"}}
{"ts":"2024-01-01T10:07:00Z","method":"GET","path":"/api/products","status":200,"latency":25,"client":{"ip":"203.0.113.6","agent":"Chrome/120","country":"FR"},"server":{"host":"web-02","region":"us-west","dc":"sfo1"}}
{"ts":"2024-01-01T10:08:00Z","method":"PUT","path":"/api/users/123","status":200,"latency":95,"client":{"ip":"10.0.0.9","agent":"axios/1.2","country":"US"},"server":{"host":"web-01","region":"us-east","dc":"nyc1"}}
{"ts":"2024-01-01T10:09:00Z","method":"GET","path":"/api/users","status":429,"latency":3,"client":{"ip":"198.51.100.2","agent":"python-requests/2.28","country":"RU"},"server":{"host":"web-01","region":"us-east","dc":"nyc1"}}
{"ts":"2024-01-01T10:10:00Z","method":"POST","path":"/api/webhooks","status":500,"latency":4500,"client":{"ip":"172.16.0.5","agent":"stripe-webhooks/1.0","country":"US"},"server":{"host":"web-02","region":"us-west","dc":"sfo1"}}
{"ts":"2024-01-01T10:11:00Z","method":"GET","path":"/api/search","status":200,"latency":310,"client":{"ip":"203.0.113.7","agent":"Safari/17.0","country":"JP"},"server":{"host":"web-01","region":"us-east","dc":"nyc1"}}
{"ts":"2024-01-01T10:12:00Z","method":"DELETE","path":"/api/orders/999","status":404,"latency":12,"client":{"ip":"10.0.0.7","agent":"internal/2.0","country":"US"},"server":{"host":"web-01","region":"us-east","dc":"nyc1"}}
{"ts":"2024-01-01T10:13:00Z","method":"POST","path":"/api/payments","status":200,"latency":850,"client":{"ip":"203.0.113.8","agent":"Stripe/3.0","country":"US"},"server":{"host":"web-02","region":"us-west","dc":"sfo1"}}
{"ts":"2024-01-01T10:14:00Z","method":"GET","path":"/api/reports","status":200,"latency":1200,"client":{"ip":"10.0.0.10","agent":"axios/1.2","country":"US"},"server":{"host":"web-01","region":"us-east","dc":"nyc1"}}
{"ts":"2024-01-01T10:15:00Z","method":"POST","path":"/api/login","status":200,"latency":220,"client":{"ip":"203.0.113.9","agent":"Firefox/121","country":"BR"},"server":{"host":"web-01","region":"us-east","dc":"nyc1"}}
{"ts":"2024-01-01T10:16:00Z","method":"GET","path":"/api/users","status":200,"latency":55,"client":{"ip":"203.0.113.10","agent":"Chrome/120","country":"GB"},"server":{"host":"web-02","region":"us-west","dc":"sfo1"}}
{"ts":"2024-01-01T10:17:00Z","method":"PATCH","path":"/api/settings","status":400,"latency":18,"client":{"ip":"10.0.0.12","agent":"internal/1.5","country":"US"},"server":{"host":"web-01","region":"us-east","dc":"nyc1"}}
{"ts":"2024-01-01T10:18:00Z","method":"GET","path":"/api/metrics","status":200,"latency":75,"client":{"ip":"10.0.0.1","agent":"prometheus/2.45","country":"US"},"server":{"host":"web-01","region":"us-east","dc":"nyc1"}}
{"ts":"2024-01-01T10:19:00Z","method":"POST","path":"/api/events","status":503,"latency":6000,"client":{"ip":"172.16.0.8","agent":"event-collector/1.0","country":"US"},"server":{"host":"web-02","region":"us-west","dc":"sfo1"}}
EOF
```

```bash
cat > k8s.log << 'EOF'
{"ts":"2024-01-01T10:00:00Z","level":"warn","msg":"pod restart","pod":{"name":"api-v2-7f8b9","namespace":"production","node":"node-03","labels":{"app":"api","version":"2.4.1","team":"platform"}},"container":{"name":"api","image":"company/api:2.4.1","restart_count":3},"reason":"OOMKilled","memory":{"limit_mb":512,"used_mb":520}}
{"ts":"2024-01-01T10:01:00Z","level":"info","msg":"deployment scaled","pod":{"name":"worker-abc12","namespace":"production","node":"node-01","labels":{"app":"worker","version":"1.9.0","team":"data"}},"container":{"name":"worker","image":"company/worker:1.9.0","restart_count":0},"reason":"HPA scale-up","replicas":{"desired":5,"current":3,"available":3}}
{"ts":"2024-01-01T10:02:00Z","level":"error","msg":"liveness probe failed","pod":{"name":"db-proxy-cd3e4","namespace":"production","node":"node-02","labels":{"app":"db-proxy","version":"3.2.0","team":"infra"}},"container":{"name":"db-proxy","image":"company/db-proxy:3.2.0","restart_count":1},"reason":"HTTP probe failed: /healthz returned 503","probe":{"type":"liveness","path":"/healthz","port":8080}}
{"ts":"2024-01-01T10:03:00Z","level":"info","msg":"pod scheduled","pod":{"name":"api-v2-9k2l3","namespace":"production","node":"node-04","labels":{"app":"api","version":"2.4.1","team":"platform"}},"container":{"name":"api","image":"company/api:2.4.1","restart_count":0},"reason":"Scheduled","resources":{"requests":{"cpu":"200m","memory":"256Mi"},"limits":{"cpu":"500m","memory":"512Mi"}}}
{"ts":"2024-01-01T10:04:00Z","level":"warn","msg":"disk pressure","pod":{"name":"log-collector-ef5g6","namespace":"kube-system","node":"node-03","labels":{"app":"log-collector","version":"0.8.0","team":"platform"}},"container":{"name":"fluentd","image":"fluentd:1.16","restart_count":0},"reason":"NodeDiskPressure","disk":{"used_gb":45,"limit_gb":50,"usage_pct":90}}
{"ts":"2024-01-01T10:05:00Z","level":"info","msg":"config map updated","pod":{"name":"api-v2-7f8b9","namespace":"production","node":"node-03","labels":{"app":"api","version":"2.4.1","team":"platform"}},"container":{"name":"api","image":"company/api:2.4.1","restart_count":3},"reason":"ConfigMap/api-config changed"}
{"ts":"2024-01-01T10:06:00Z","level":"error","msg":"image pull failed","pod":{"name":"api-v3-beta-gh7h8","namespace":"staging","node":"node-05","labels":{"app":"api","version":"3.0.0-beta","team":"platform"}},"container":{"name":"api","image":"company/api:3.0.0-beta","restart_count":0},"reason":"ErrImagePull: repository not found","registry":{"host":"registry.company.com","repo":"company/api","tag":"3.0.0-beta"}}
{"ts":"2024-01-01T10:07:00Z","level":"warn","msg":"cpu throttling","pod":{"name":"worker-abc12","namespace":"production","node":"node-01","labels":{"app":"worker","version":"1.9.0","team":"data"}},"container":{"name":"worker","image":"company/worker:1.9.0","restart_count":0},"reason":"CPUThrottling","cpu":{"throttled_pct":78,"requests":"200m","limits":"500m"}}
{"ts":"2024-01-01T10:08:00Z","level":"info","msg":"rolling update started","pod":{"name":"web-ij9k0","namespace":"production","node":"node-02","labels":{"app":"web","version":"3.1.0","team":"frontend"}},"container":{"name":"web","image":"company/web:3.1.0","restart_count":0},"reason":"Deployment update: 3.0.9 -> 3.1.0","replicas":{"desired":3,"current":3,"available":2}}
{"ts":"2024-01-01T10:09:00Z","level":"error","msg":"crash loop backoff","pod":{"name":"auth-lm1n2","namespace":"production","node":"node-03","labels":{"app":"auth","version":"4.0.2","team":"security"}},"container":{"name":"auth","image":"company/auth:4.0.2","restart_count":8},"reason":"CrashLoopBackOff","last_exit":{"code":1,"reason":"Error","message":"failed to connect to postgres: connection refused"}}
{"ts":"2024-01-01T10:10:00Z","level":"info","msg":"pod terminated gracefully","pod":{"name":"api-v2-old-op3q4","namespace":"production","node":"node-04","labels":{"app":"api","version":"2.4.0","team":"platform"}},"container":{"name":"api","image":"company/api:2.4.0","restart_count":0},"reason":"Evicted for rolling update"}
{"ts":"2024-01-01T10:11:00Z","level":"warn","msg":"persistent volume claim pending","pod":{"name":"db-01-rs5t6","namespace":"production","node":"","labels":{"app":"db","version":"5.7.0","team":"infra"}},"container":{"name":"mysql","image":"mysql:5.7","restart_count":0},"reason":"PVC db-data-01 in Pending state","storage":{"class":"fast-ssd","size":"100Gi","provisioner":"ebs.csi.aws.com"}}
{"ts":"2024-01-01T10:12:00Z","level":"info","msg":"horizontal pod autoscaler triggered","pod":{"name":"api-v2-uv7w8","namespace":"production","node":"node-01","labels":{"app":"api","version":"2.4.1","team":"platform"}},"container":{"name":"api","image":"company/api:2.4.1","restart_count":0},"reason":"CPU utilization 82% > target 70%","replicas":{"desired":6,"current":4,"available":4}}
{"ts":"2024-01-01T10:13:00Z","level":"error","msg":"secret not found","pod":{"name":"payments-xy9z0","namespace":"production","node":"node-02","labels":{"app":"payments","version":"2.1.0","team":"billing"}},"container":{"name":"payments","image":"company/payments:2.1.0","restart_count":2},"reason":"secret payments-db-creds not found in namespace production"}
{"ts":"2024-01-01T10:14:00Z","level":"info","msg":"readiness probe passed","pod":{"name":"api-v2-9k2l3","namespace":"production","node":"node-04","labels":{"app":"api","version":"2.4.1","team":"platform"}},"container":{"name":"api","image":"company/api:2.4.1","restart_count":0},"reason":"HTTP 200 from /ready","probe":{"type":"readiness","path":"/ready","port":8080}}
{"ts":"2024-01-01T10:15:00Z","level":"warn","msg":"network policy violation","pod":{"name":"worker-abc12","namespace":"production","node":"node-01","labels":{"app":"worker","version":"1.9.0","team":"data"}},"container":{"name":"worker","image":"company/worker:1.9.0","restart_count":0},"reason":"Egress blocked to 0.0.0.0/0 port 443"}
{"ts":"2024-01-01T10:16:00Z","level":"info","msg":"cron job completed","pod":{"name":"cleanup-job-1234","namespace":"production","node":"node-05","labels":{"app":"cleanup","version":"1.0.0","team":"platform"}},"container":{"name":"cleanup","image":"company/cleanup:1.0.0","restart_count":0},"reason":"Completed","job":{"name":"daily-cleanup","duration_s":45,"items_deleted":1204}}
{"ts":"2024-01-01T10:17:00Z","level":"error","msg":"resource quota exceeded","pod":{"name":"batch-ab3c4","namespace":"staging","node":"","labels":{"app":"batch","version":"1.2.0","team":"data"}},"container":{"name":"batch","image":"company/batch:1.2.0","restart_count":0},"reason":"exceeded quota: requests.memory 4Gi > limit 3Gi","quota":{"namespace":"staging","resource":"requests.memory","used":"4Gi","limit":"3Gi"}}
{"ts":"2024-01-01T10:18:00Z","level":"info","msg":"node joined cluster","pod":{"name":"","namespace":"kube-system","node":"node-06","labels":{"app":"kubelet","version":"1.27.0","team":"infra"}},"container":{"name":"","image":"","restart_count":0},"reason":"Node node-06 registered successfully","node_info":{"os":"linux","arch":"amd64","kernel":"5.15.0","capacity":{"cpu":"8","memory":"32Gi"}}}
{"ts":"2024-01-01T10:19:00Z","level":"warn","msg":"certificate expiring soon","pod":{"name":"ingress-ef5g6","namespace":"kube-system","node":"node-01","labels":{"app":"ingress-nginx","version":"1.9.0","team":"infra"}},"container":{"name":"controller","image":"ingress-nginx:1.9.0","restart_count":0},"reason":"TLS certificate for api.company.com expires in 14 days","cert":{"domain":"api.company.com","issuer":"Let's Encrypt","expires":"2024-02-14"}}
EOF
```

---

## Basic Usage

### Display All Records

```bash
qk app.log
# → {"ts":"2024-01-01T10:00:00Z","level":"info","service":"api","msg":"server started","latency":0,...}
# → {"ts":"2024-01-01T10:01:00Z","level":"error","service":"api","msg":"connection timeout","latency":3001,...}
# → (25 records total)
```

In a terminal, output is colorized: error=red, warn=yellow, info=green.

### Read From stdin

```bash
echo '{"level":"error","msg":"oops"}' | qk
# → {"level":"error","msg":"oops"}
```

### Inspect Parsing (--explain)

```bash
qk --explain where level=error app.log
# → mode:    Keyword
# → format:  Ndjson (detected)
# → query:   FastQuery { filters: [level = error], ... }
# → files:   ["app.log"]
```

The `--explain` flag prints the detected format and parsed query, then exits.

---

## Filtering (where)

### Equals (=)

```bash
qk where level=error app.log
# → {"ts":"2024-01-01T10:01:00Z","level":"error","service":"api","msg":"connection timeout","latency":3001,...}
# → {"ts":"2024-01-01T10:04:00Z","level":"error","service":"worker","msg":"panic: nil pointer","latency":0,...}
# → (all error records)
```

### Not Equals (!=)

```bash
qk where level!=info app.log
# → {"ts":"2024-01-01T10:01:00Z","level":"error",...}
# → {"ts":"2024-01-01T10:02:00Z","level":"warn",...}
# → {"ts":"2024-01-01T10:04:00Z","level":"error",...}
# → (all non-info entries)
```

### Numeric Greater Than (>)

```bash
# Quoted (embedded operators work when quoted)
qk where 'latency>100' app.log
# Word operators — no quoting needed, shell-safe
qk where latency gt 100 app.log
# → {"ts":"2024-01-01T10:01:00Z","level":"error","service":"api","msg":"connection timeout","latency":3001,...}
# → {"ts":"2024-01-01T10:02:00Z","level":"warn","service":"worker","msg":"queue depth high","latency":150,...}
# → (all records with latency > 100)
```

### Numeric Less Than (<)

```bash
# Quoted style
qk where 'latency<50' app.log
# Word operator style — shell-safe
qk where latency lt 50 app.log
# → {"ts":"2024-01-01T10:00:00Z","level":"info","service":"api","msg":"server started","latency":0,...}
# → {"ts":"2024-01-01T10:03:00Z","level":"info","service":"api","msg":"request ok","latency":42,...}
# → (all records with latency < 50)
```

### Greater Than or Equal (>=)

```bash
# Quoted style
qk where 'status>=500' access.log
# Word operator style — shell-safe
qk where status gte 500 access.log
# → {"ts":"2024-01-01T10:02:00Z","method":"GET","path":"/api/orders","status":500,"latency":3200,...}
# → {"ts":"2024-01-01T10:04:00Z","method":"GET","path":"/api/users","status":503,"latency":9800,...}
# → (all 5xx responses)
```

### Less Than or Equal (<=)

```bash
# Quoted style
qk where 'latency<=42' app.log
# Word operator style — shell-safe
qk where latency lte 42 app.log
# → {"ts":"2024-01-01T10:00:00Z",...,"latency":0}
# → {"ts":"2024-01-01T10:03:00Z",...,"latency":42}
# → (all records with latency <= 42)
```

### Regex Match (~=)

```bash
qk where msg~=.*timeout.* app.log
# → {"ts":"2024-01-01T10:01:00Z","level":"error","service":"api","msg":"connection timeout","latency":3001,...}
# → {"ts":"2024-01-01T10:07:00Z","level":"error","service":"db","msg":"query timeout","latency":5001,...}
```

### Contains Substring (contains)

```bash
qk where msg contains queue app.log
# → {"ts":"2024-01-01T10:02:00Z","level":"warn","service":"worker","msg":"queue depth high","latency":150,...}
```

### Field Exists (exists)

```bash
# Find all records that have a field named "error" (note: this is the field name, not level=error)
echo '{"level":"info","msg":"ok"}
{"level":"error","msg":"bad","error":"connection refused"}' | qk where error exists
# → {"level":"error","msg":"bad","error":"connection refused"}
```

### AND — Multiple Conditions

```bash
qk where level=error and service=api app.log
# → {"ts":"2024-01-01T10:01:00Z","level":"error","service":"api","msg":"connection timeout","latency":3001,...}
# → (all error records from the api service)
```

### OR — Multiple Conditions

```bash
qk where level=error or level=warn app.log
# → {"ts":"2024-01-01T10:01:00Z","level":"error",...}
# → {"ts":"2024-01-01T10:02:00Z","level":"warn",...}
# → {"ts":"2024-01-01T10:04:00Z","level":"error",...}
# → (all error and warn records)
```

### Comma Separator (Readable AND)

Comma is an alias for `and` — write conditions as a comma-separated list for clarity:

```bash
qk where level=error, service=api app.log
# → {"level":"error","service":"api","msg":"connection timeout","latency":3001,...}

# Comma can also stand alone as a token
qk where level=error , service=api app.log

# Mix comma with and/or (comma binds as and)
qk where level=error, latency gt 100 app.log
# → {"level":"error","latency":3001,...}
# → {"level":"error","latency":5001,...}
```

Before commas, the only option was:
`qk where level=error and service=api and latency gt 100 app.log`

With commas:
`qk where level=error, service=api, latency gt 100 app.log`

### Nested Field Access (dot path)

```bash
# Simple two-level nested field filter
qk where response.status=503 app.log
# → {"level":"error","service":"api","msg":"upstream service unavailable","response":{"status":503,...},...}

# Word operators on nested numeric fields
qk where response.status gte 500 app.log
qk where 'response.status>=500' app.log

# Access context (2-level nesting)
qk where context.region=us-east app.log

# Three-level nesting: pod.labels.app in Kubernetes logs
qk where pod.labels.app=api k8s.log
qk where pod.labels.team=platform, level=error k8s.log
```

---

## Field Selection (select)

### Keep Only Specified Fields

```bash
qk where level=error select ts service msg app.log
# → {"ts":"2024-01-01T10:01:00Z","service":"api","msg":"connection timeout"}
# → {"ts":"2024-01-01T10:04:00Z","service":"worker","msg":"panic: nil pointer"}
# → (all error records with only ts, service, msg)
```

### Select Fields Without Filtering

```bash
qk select level msg service app.log
# → {"level":"info","msg":"server started","service":"api"}
# → {"level":"error","msg":"connection timeout","service":"api"}
# → {"level":"warn","msg":"queue depth high","service":"worker"}
# → (all 25 records, only level, msg, service retained)
```

### Select Nested Fields

```bash
qk where response.status=503 select service response.status response.error app.log
# → {"service":"api","response.status":503,"response.error":"connection refused: ml-service:8080"}
```

---

## Counting (count)

### Count Total Records

```bash
qk count app.log
# → {"count":25}
```

### Count After Filtering

```bash
qk where level=error count app.log
# → {"count":7}
```

### Count Grouped By Field

```bash
qk count by level app.log
# → {"level":"info","count":10}
# → {"level":"error","count":7}
# → {"level":"warn","count":5}
# → {"level":"debug","count":2}
# → (sorted by count descending)
```

```bash
qk count by level k8s.log
# → {"level":"info","count":9}
# → {"level":"warn","count":6}
# → {"level":"error","count":5}
```

Results are sorted by count descending.

### Group By Another Field

```bash
qk count by service app.log
# → {"service":"api","count":9}
# → {"service":"worker","count":4}
# → (all services by count)
```

```bash
# Three-level nested group-by
qk count by pod.labels.team k8s.log
# → {"pod.labels.team":"platform","count":8}
# → {"pod.labels.team":"infra","count":4}
# → {"pod.labels.team":"data","count":4}
# → (all teams by count)
```

### Filter Then Group

```bash
qk where latency gt 0 count by service app.log
# → records filtered to latency > 0, then grouped by service
```

---

## Sorting (sort)

### Numeric Descending (largest first)

```bash
qk sort latency desc app.log
# → {"ts":"...","level":"info","service":"db","msg":"backup complete","latency":12000,...}
# → {"ts":"...","level":"error","service":"db","msg":"query timeout","latency":5001,...}
# → {"ts":"...","level":"info","service":"api","msg":"batch job complete","latency":4500,...}
# → (all records sorted by latency high to low)
```

### Numeric Ascending (smallest first)

```bash
qk sort latency asc app.log
# → {"ts":"...","latency":0}   (multiple records with latency=0)
# → {"ts":"...","latency":1}
# → {"ts":"...","latency":2}
# → ...
```

### Sort By String Field

```bash
qk sort service app.log
# → {"service":"api",...}
# → {"service":"api",...}
# → (sorted alphabetically by service)
```

Sorted alphabetically by service.

### Combined: Filter Then Sort

```bash
qk where level=error sort latency desc app.log
# → {"ts":"...","level":"error","service":"db","msg":"query timeout","latency":5001,...}
# → {"ts":"...","level":"error","service":"api","msg":"connection timeout","latency":3001,...}
# → (errors sorted by latency descending)
```

---

## Limiting Results (limit / head)

### Take First N Records

```bash
qk limit 3 app.log
# → {"ts":"2024-01-01T10:00:00Z","level":"info",...}
# → {"ts":"2024-01-01T10:01:00Z","level":"error",...}
# → {"ts":"2024-01-01T10:02:00Z","level":"warn",...}
```

### head Is an Alias for limit

```bash
qk head 2 app.log
# → {"ts":"2024-01-01T10:00:00Z","level":"info",...}
# → {"ts":"2024-01-01T10:01:00Z","level":"error",...}
```

Identical behavior to `limit 2`.

### Combined: Sort Then Take Top N

```bash
qk sort latency desc limit 3 app.log
# → {"latency":12000,...}
# → {"latency":5001,...}
# → {"latency":4500,...}
```

---

## Numeric Aggregation (sum / avg / min / max)

### Sum

```bash
qk sum latency app.log
# → {"sum":<total of all 25 latency values>}
```

### Sum After Filtering

```bash
qk where level=error sum latency app.log
# → {"sum":<sum of latency for error records>}
```

### Average

```bash
qk avg latency app.log
# → {"avg":<average latency across all 25 records>}
```

### Average After Filtering

```bash
qk where latency gt 0 avg latency app.log
# → {"avg":<average of non-zero latency records>}
```

### Minimum

```bash
qk min latency app.log
# → {"min":0}
```

### Minimum (Excluding Zero)

```bash
qk where latency gt 0 min latency app.log
# → {"min":1}
```

The smallest non-zero latency.

### Maximum

```bash
qk max latency app.log
# → {"max":12000}
```

### Worst HTTP Response Time

```bash
qk where status gte 500 max latency access.log
# → {"max":9800}
```

The slowest 5xx response.

---

## Field Discovery (fields)

### Discover All Field Names

```bash
qk fields app.log
# → {"field":"context"}
# → {"field":"host"}
# → {"field":"latency"}
# → {"field":"level"}
# → {"field":"msg"}
# → {"field":"service"}
# → {"field":"ts"}
# → (sorted alphabetically; nested objects shown as top-level keys)
```

### Discover Fields After Filtering

```bash
qk where level=error fields app.log
# → (field names present in error records)
```

### Field Discovery on a Different File

```bash
qk fields access.log
# → {"field":"client"}
# → {"field":"latency"}
# → {"field":"method"}
# → {"field":"path"}
# → {"field":"server"}
# → {"field":"status"}
# → {"field":"ts"}
```

### Count How Many Fields Exist

```bash
qk fields app.log | qk count
# → {"count":<number of top-level fields>}
```

---

## DSL Expression Syntax

DSL mode is activated automatically when the first argument starts with `.`, `not `, or `|`.

### Equals

```bash
qk '.level == "error"' app.log
# → {"ts":"2024-01-01T10:01:00Z","level":"error","service":"api","msg":"connection timeout","latency":3001,...}
# → (all error records)
```

### Not Equals

```bash
qk '.level != "info"' app.log
# → {"level":"error",...}
# → {"level":"warn",...}
# → {"level":"debug",...}
# → (all non-info records)
```

### Numeric Comparison

```bash
qk '.latency > 100' app.log
# → {"latency":3001,...}
# → {"latency":150,...}
# → (all records with latency > 100)
```

```bash
qk '.latency >= 88' app.log
# → records with latency 88, 120, 150, 230, 380, ... (all >= 88)
```

### Boolean Values

```bash
echo '{"service":"api","enabled":true}
{"service":"worker","enabled":false}' | qk '.enabled == true'
# → {"service":"api","enabled":true}
```

### null Comparison

```bash
echo '{"service":"api","error":null}
{"service":"web"}
{"service":"worker","error":"timeout"}' | qk '.error != null'
# → {"service":"worker","error":"timeout"}
```

Records where `error` is null or the field is absent are excluded; only records with an actual value are kept.

### Field Exists (exists)

```bash
qk '.latency exists' app.log
# → (all 25 records — every record has a latency field)
```

### Contains Substring (contains)

```bash
qk '.msg contains "timeout"' app.log
# → {"ts":"2024-01-01T10:01:00Z","level":"error","service":"api","msg":"connection timeout","latency":3001,...}
# → {"ts":"2024-01-01T10:07:00Z","level":"error","service":"db","msg":"query timeout","latency":5001,...}
```

### Regex Match (matches)

```bash
qk '.msg matches "pan.*pointer"' app.log
# → {"ts":"2024-01-01T10:04:00Z","level":"error","service":"worker","msg":"panic: nil pointer","latency":0,...}
```

### AND

```bash
qk '.level == "error" and .service == "api"' app.log
# → (all error records from service=api)
```

### OR

```bash
qk '.level == "error" or .level == "warn"' app.log
# → {"level":"error",...}
# → {"level":"warn",...}
# → (all error and warn records)
```

### NOT

```bash
qk 'not .level == "info"' app.log
# → {"level":"error",...}
# → {"level":"warn",...}
# → {"level":"debug",...}
# → (all non-info records — equivalent to != info)
```

### Compound Logic

```bash
qk '.latency > 100 and (.level == "error" or .level == "warn")' app.log
# → records where latency > 100 AND (error or warn)
```

### Nested Fields — 2 Levels Deep

```bash
# Match on a nested field
qk where response.status=503 app.log
# → {"level":"error","service":"api","msg":"upstream service unavailable","response":{"status":503,...},...}

# Word operators on nested numeric fields
qk where response.status gte 500 app.log
qk where 'response.status>=500' app.log

# Select nested fields
qk where response.status=503 select service response.status response.error app.log

# Count by nested field
qk count by response.status app.log
qk count by context.region app.log
```

### Nested Fields — 3 Levels Deep

```bash
# context.region is 2 levels; request.headers.x-trace is 3 levels
qk where context.region=us-east app.log
qk where context.env=prod, level=error app.log

# DSL — three-level access
qk '.request.headers.x-trace exists' app.log
qk '.request.headers.user-agent contains "Mozilla"' app.log

# Kubernetes logs: pod.labels.app is 3 levels deep
qk where pod.labels.app=api k8s.log
qk where pod.labels.team=platform, level=error k8s.log

# Even deeper: container info
qk where 'container.restart_count gt 2' k8s.log
qk where container.restart_count gt 2, level=warn k8s.log
```

### Nested Fields — DSL Mode

```bash
# Filter on deeply nested field, then pick only the fields you want
qk '.response.status >= 500 | pick(.ts, .service, .response.status, .response.error)' app.log

# Group by nested field
qk '| group_by(.context.region)' app.log
qk '| group_by(.response.status)' app.log

# Aggregate on nested numeric
qk '.response.status >= 200 | avg(.latency)' app.log
qk '.response.status >= 500 | max(.latency)' app.log

# Three-level access in DSL
qk '.pod.labels.app == "api" | group_by(.level)' k8s.log
qk '.pod.labels.team == "platform" and .level == "error"' k8s.log
qk '.container.restart_count > 5 | pick(.ts, .pod.name, .container.restart_count, .reason)' k8s.log
```

### No Filter (Pass All Records Through)

```bash
qk '| count()' app.log
# → {"count":25}
```

Starting with `|` skips filtering and goes directly to the pipeline stage.

---

## DSL Pipeline Stages

### pick (Keep Only Specified Fields)

```bash
qk '.level == "error" | pick(.ts, .service, .msg)' app.log
# → {"ts":"2024-01-01T10:01:00Z","service":"api","msg":"connection timeout"}
# → {"ts":"2024-01-01T10:04:00Z","service":"worker","msg":"panic: nil pointer"}
# → (all error records with only ts, service, msg)
```

`latency` is dropped.

### omit (Remove Specified Fields)

```bash
qk '.level == "error" | omit(.ts, .latency)' app.log
# → {"level":"error","service":"api","msg":"connection timeout",...}
# → {"level":"error","service":"worker","msg":"panic: nil pointer",...}
```

`ts` and `latency` are removed.

### count (Count Records)

```bash
qk '.level == "error" | count()' app.log
# → {"count":7}
```

### sort\_by (Sort Records)

```bash
qk '.latency > 0 | sort_by(.latency desc)' app.log
# → {"latency":12000,...}
# → {"latency":5001,...}
# → {"latency":4500,...}
# → (non-zero latency records, highest first)
```

```bash
qk '.latency > 0 | sort_by(.latency asc)' app.log
# → {"latency":1,...}
# → {"latency":2,...}
# → {"latency":3,...}
# → (non-zero latency records, lowest first)
```

### group\_by (Group and Count)

```bash
qk '| group_by(.level)' app.log
# → {"level":"info","count":10}
# → {"level":"error","count":7}
# → {"level":"warn","count":5}
# → {"level":"debug","count":2}
```

Sorted by count descending.

```bash
qk '.level == "error" | group_by(.service)' app.log
# → (error records grouped by service)
```

### limit (Take First N Records)

```bash
qk '.latency >= 0 | sort_by(.latency desc) | limit(3)' app.log
# → {"latency":12000,...}
# → {"latency":5001,...}
# → {"latency":4500,...}
```

Top 3 by highest latency.

### skip (Skip First N Records — Pagination)

```bash
qk '.latency >= 0 | sort_by(.latency desc) | skip(2)' app.log
# → starts from the 3rd record (skips top 2)
```

### skip + limit for Pagination

```bash
# Page 1 (records 1–3)
qk '.latency >= 0 | sort_by(.latency desc) | limit(3)' app.log
# Page 2 (records 4–6)
qk '.latency >= 0 | sort_by(.latency desc) | skip(3) | limit(3)' app.log
# Page 3 (records 7–9)
qk '.latency >= 0 | sort_by(.latency desc) | skip(6) | limit(3)' app.log
```

### dedup (Deduplicate)

```bash
qk '| dedup(.service)' app.log
# → {"service":"api",...}    (first occurrence of api)
# → {"service":"worker",...} (first occurrence of worker)
# → (one record per unique service)
```

Only the first record for each unique service value is kept.

```bash
# Count distinct service values
qk '| dedup(.service) | count()' app.log
# → {"count":<number of unique services>}
```

### sum (Sum a Field)

```bash
qk '.latency >= 0 | sum(.latency)' app.log
# → {"sum":<sum of all latency values>}
```

### avg (Average a Field)

```bash
qk '.latency > 0 | avg(.latency)' app.log
# → {"avg":<average of non-zero latency records>}
```

### min (Minimum of a Field)

```bash
qk '.latency > 0 | min(.latency)' app.log
# → {"min":1}
```

Smallest non-zero latency.

### max (Maximum of a Field)

```bash
qk '.latency > 0 | max(.latency)' app.log
# → {"max":12000}
```

### Chained Pipelines (Multi-Stage)

```bash
# Filter errors → sort by latency descending → keep key fields only
qk '.level == "error" | sort_by(.latency desc) | pick(.ts, .service, .msg, .latency)' app.log
# → {"ts":"2024-01-01T10:07:00Z","service":"db","msg":"query timeout","latency":5001}
# → {"ts":"2024-01-01T10:01:00Z","service":"api","msg":"connection timeout","latency":3001}
# → (errors sorted by latency, key fields only)
```

```bash
# All records → group by service → take top 3 groups
qk '| group_by(.service) | limit(3)' app.log
# → {"service":"api","count":9}
# → (top 3 services by record count)
```

```bash
# Filter slow requests → deduplicate (one entry per service) → keep key fields
qk '.latency > 50 | dedup(.service) | pick(.service, .latency, .msg)' app.log
# → (first slow record per service)
```

---

## qk + jq: Handling JSON-Encoded Strings

Sometimes a field's **value** is itself a JSON string (double-encoded):

```json
{"service":"api","metadata":"{\"region\":\"us-east\",\"env\":\"prod\"}","payload":"{\"level\":\"error\",\"code\":500}"}
```

qk cannot drill into a string — it sees `metadata` as a plain string. The solution is to combine qk and jq. These tools compose naturally because qk outputs NDJSON.

### Decode the nested string, then query with qk

```bash
cat > encoded.log << 'EOF'
{"service":"api","metadata":"{\"region\":\"us-east\",\"env\":\"prod\"}","payload":"{\"level\":\"error\",\"code\":500}","ts":"2024-01-01T10:01:00Z"}
{"service":"worker","metadata":"{\"region\":\"us-west\",\"env\":\"staging\"}","payload":"{\"level\":\"info\",\"code\":200}","ts":"2024-01-01T10:02:00Z"}
{"service":"web","metadata":"{\"region\":\"us-east\",\"env\":\"prod\"}","payload":"{\"level\":\"warn\",\"code\":429}","ts":"2024-01-01T10:03:00Z"}
{"service":"db","metadata":"{\"region\":\"us-east\",\"env\":\"prod\"}","payload":"{\"level\":\"error\",\"code\":503}","ts":"2024-01-01T10:04:00Z"}
EOF

# Step 1: use jq to decode the string field into a real object
# Step 2: pipe to qk to filter on the decoded field
cat encoded.log | jq -c '.payload = (.payload | fromjson)' | qk where payload.level=error
# → {"service":"api","metadata":"...","payload":{"level":"error","code":500},"ts":"..."}
# → {"service":"db","metadata":"...","payload":{"level":"error","code":503},"ts":"..."}
```

### Decode multiple string fields at once

```bash
cat encoded.log | jq -c '{service, ts, payload: (.payload | fromjson), meta: (.metadata | fromjson)}' \
  | qk where meta.env=prod, payload.level=error
# → {"service":"api","ts":"...","payload":{"level":"error","code":500},"meta":{"region":"us-east","env":"prod"}}
# → {"service":"db","ts":"...","payload":{"level":"error","code":503},"meta":{"region":"us-east","env":"prod"}}
```

### qk first, jq drills deeper

```bash
# qk does the fast filter on top-level fields, jq extracts the encoded sub-field
cat encoded.log | qk where service=api | jq -r '.payload | fromjson | .code'
# → 500
```

### Full pipeline: qk filters → jq decodes → qk aggregates

```bash
# Three-stage pipeline: qk pre-filters by service → jq decodes payload → qk counts by decoded level
cat encoded.log \
  | qk where metadata contains prod \
  | jq -c '.payload = (.payload | fromjson)' \
  | qk count by payload.level
# → {"payload.level":"error","count":2}
# → {"payload.level":"warn","count":1}
```

### When to use qk vs jq vs both

| Situation | Tool |
|-----------|------|
| Fields are real JSON objects (nested) | qk alone handles it |
| A field's **value** is a JSON-encoded string | Use `jq ... \| fromjson` to decode first, then qk |
| Fast filtering on millions of records, then decode | qk first (fast), then jq (precise) |
| Complex reshaping / math / conditionals | jq |
| Counting, aggregating, tabular output | qk |

---

## Output Formats (--fmt)

> **`--fmt` must be placed before the query expression!**
> Correct: `qk --fmt table where level=error app.log`
> Wrong: `qk where level=error --fmt table app.log`

### ndjson (Default)

```bash
qk --fmt ndjson where level=error app.log
# → {"ts":"2024-01-01T10:01:00Z","level":"error","service":"api","msg":"connection timeout","latency":3001,...}
# → (all error records, one JSON object per line)
```

One JSON object per line — same as the default output.

### pretty (Indented JSON — replaces `jq .`)

```bash
qk --fmt pretty where level=error limit 1 app.log
# → {
# →   "ts": "2024-01-01T10:01:00Z",
# →   "level": "error",
# →   "service": "api",
# →   "msg": "connection timeout",
# →   "latency": 3001,
# →   ...
# → }
```

Indented format with blank lines between blocks.

### pretty + color (Pretty-Print With Semantic Color)

```bash
qk --fmt pretty --color where level=error app.log
```

In a terminal: key names are bold cyan, strings are green, numbers are yellow, null is dim.

### table (Aligned Table)

```bash
qk --fmt table where level=error select ts service msg latency app.log
# →  ts                       service  msg                              latency
# →  2024-01-01T10:01:00Z     api      connection timeout               3001
# →  2024-01-01T10:04:00Z     worker   panic: nil pointer               0
# →  (all error records in table format)
```

Auto-aligned columns with bold headers.

### table + Field Selection

```bash
qk --fmt table where level=error select ts service msg app.log
# →  ts                       service  msg
# →  2024-01-01T10:01:00Z     api      connection timeout
# →  2024-01-01T10:04:00Z     worker   panic: nil pointer
```

Only 3 columns.

### csv (Openable in Excel)

```bash
qk --fmt csv where level=error select ts service msg latency app.log
# → latency,msg,service,ts
# → 3001,connection timeout,api,2024-01-01T10:01:00Z
# → 0,panic: nil pointer,worker,2024-01-01T10:04:00Z
```

First row is the header.

### Export csv to a File

```bash
qk --fmt csv where level=error app.log > errors.csv
cat errors.csv
```

### raw (Original Lines, No Re-serialization)

```bash
qk --fmt raw where level=error app.log
# → (original text lines from the file, field order exactly as in the source)
```

The original text line from the file, with field order exactly as in the source.

### DSL + pretty

```bash
qk --fmt pretty '.level == "error" | pick(.service, .msg, .latency)' app.log
# → {
# →   "service": "api",
# →   "msg": "connection timeout",
# →   "latency": 3001
# → }
# →
# → (one pretty block per error record)
```

---

## Color Output (--color)

### Default Behavior

- **Terminal**: colors are enabled automatically
- **Pipe** (`qk ... | other`): colors are disabled automatically

### Force Colors On (Piping to less)

```bash
qk --color where level=error app.log | less -R
```

`less -R` renders ANSI color codes; `--color` forces qk to emit them even in a pipe.

### Force Colors Off

```bash
qk --no-color where level=error app.log
```

Plain text output with no color codes — suitable for writing to files or tools that don't support color.

### Disable Via Environment Variable (NO\_COLOR Standard)

```bash
NO_COLOR=1 qk where level=error app.log
```

### Priority Verification

```bash
# --no-color takes precedence over --color; output has no color
qk --no-color --color where level=error app.log
```

### Color Scheme (NDJSON Output)

| Field / Value                    | Color           |
| -------------------------------- | --------------- |
| Field names (all keys)           | Bold cyan       |
| `level: "error"` / `"fatal"`    | **Bold red**    |
| `level: "warn"`                  | **Bold yellow** |
| `level: "info"`                  | **Bold green**  |
| `level: "debug"`                 | Blue            |
| `level: "trace"`                 | Dim             |
| `msg` / `message` values         | Bright white    |
| `ts` / `timestamp` values        | Dim             |
| `error` / `exception` field values | Red           |
| HTTP `status` 200–299            | Green           |
| HTTP `status` 300–399            | Cyan            |
| HTTP `status` 400–499            | Yellow          |
| HTTP `status` 500–599            | **Bold red**    |
| Numbers (other fields)           | Yellow          |
| Booleans                         | Magenta         |
| null                             | Dim             |

---

## Multiple File Formats

`qk` detects the format automatically — no flags needed.

### logfmt Format

```bash
cat > app.logfmt << 'EOF'
level=info service=api msg="server started" latency=0
level=error service=api msg="connection timeout" latency=3001
level=warn service=worker msg="queue depth high" latency=150
EOF

qk where level=error app.logfmt
# → {"level":"error","service":"api","msg":"connection timeout","latency":"3001"}
```

### CSV Format

```bash
cat > data.csv << 'EOF'
name,age,city
alice,30,NYC
bob,25,SF
carol,35,NYC
EOF

qk where city=NYC data.csv
# → {"name":"alice","age":"30","city":"NYC"}
# → {"name":"carol","age":"35","city":"NYC"}
```

### YAML Format (Multi-Document)

```bash
cat > services.yaml << 'EOF'
---
name: api
port: 8080
enabled: true
---
name: worker
port: 9090
enabled: false
---
name: web
port: 3000
enabled: true
EOF

qk where enabled=true services.yaml
# → {"name":"api","port":8080,"enabled":true}
# → {"name":"web","port":3000,"enabled":true}
```

2 records with enabled=true.

### TOML Format

```bash
cat > config.toml << 'EOF'
port = 8080
host = "localhost"
debug = false
max_connections = 100
EOF

qk config.toml
# → {"port":8080,"host":"localhost","debug":false,"max_connections":100}
```

The entire TOML file is treated as a single record.

```bash
qk '.port > 8000' config.toml
# → {"port":8080,"host":"localhost","debug":false,"max_connections":100}
```

### Gzip Compressed Files (Transparent Decompression)

```bash
# Compress the log first
gzip -k app.log      # creates app.log.gz, keeps the original

# Query directly — no manual decompression needed
qk where level=error app.log.gz
# → (same error records as querying app.log directly)
```

Identical output to querying `app.log`.

### Plain Text (Each Line Becomes a `line` Field)

```bash
cat > notes.txt << 'EOF'
error: connection refused at 10:01
info: server started
error: timeout after 30s
EOF

qk where line contains error notes.txt
# → {"line":"error: connection refused at 10:01"}
# → {"line":"error: timeout after 30s"}
```

### Query Multiple Files and Formats Simultaneously

```bash
qk where level=error app.log app.logfmt
```

Both files are processed in parallel and output is merged.

### Glob Patterns

```bash
qk where level=error *.log
```

The shell expands the glob; qk processes all matching files in parallel.

---

## Pipeline Composition

### Two qk Commands Chained

```bash
# Filter errors, then count by service
qk where level=error app.log | qk count by service
# → (error records grouped by service)
```

### Three-Stage Pipeline

```bash
# Filter → sort → limit
qk where level=error app.log | qk sort latency desc | qk limit 1
# → (the single error record with the highest latency)
```

The slowest error record.

### Combined With jq

```bash
# qk filters, jq does further processing
qk where level=error app.log | jq '.latency'
# → 3001
# → 0
# → (latency values for all error records)
```

### Combined With grep

```bash
# qk filters by format, grep does exact text matching
qk where service=api app.log | grep timeout
```

### Live Log Tailing (tail -f)

```bash
# Monitor errors in a live log stream (requires a real log file)
tail -f /var/log/app.log | qk where level=error
```

---

## Common Questions

### Q: `--fmt` has no effect and output is still NDJSON?

Flags must come before the query:

```bash
# Correct
qk --fmt table where level=error app.log

# Wrong (--fmt is treated as a file name)
qk where level=error --fmt table app.log
```

### Q: Why do string comparisons in DSL require quotes?

In keyword mode the `=` operator takes a bare value; in DSL mode `==` requires JSON-style double quotes:

```bash
# Keyword mode: no quotes needed
qk where level=error app.log

# DSL mode: strings must be double-quoted
qk '.level == "error"' app.log
```

### Q: How do I filter records where a field is null?

```bash
# Field exists but its value is null
echo '{"service":"api","error":null}
{"service":"web","error":"timeout"}' | qk '.error == null'
# → {"service":"api","error":null}
```

### Q: Colors don't render in less?

```bash
qk --color where level=error app.log | less -R
```

You need both `--color` (to force qk to emit ANSI codes) and `less -R` (to render them).

### Q: How do I suppress colors when writing to a file?

```bash
qk --no-color where level=error app.log > filtered.log
```

### Q: How do I count records that match a condition?

```bash
# Method 1: keyword syntax
qk where level=error count app.log

# Method 2: DSL syntax
qk '.level == "error" | count()' app.log
```

Both produce the same output:

```bash
qk where level=error count app.log
# → {"count":7}
```

### Q: How do I use numeric operators without shell quoting issues?

Use word operators instead of symbol operators — they require no quoting:

```bash
# Symbol operators require quoting in most shells
qk where 'latency>=100' app.log
qk where 'status>=500' access.log

# Word operators are always shell-safe
qk where latency gte 100 app.log
qk where status gte 500 access.log
qk where latency gt 100 app.log      # >
qk where latency lt 100 app.log      # <
qk where latency lte 100 app.log     # <=
```

---

## Quick Reference

### Global Flags (Must Come Before the Query)

```bash
qk --fmt ndjson   # NDJSON (default)
qk --fmt pretty   # indented JSON
qk --fmt table    # aligned table
qk --fmt csv      # CSV
qk --fmt raw      # original lines
qk --color        # force colors on
qk --no-color     # force colors off
qk --explain      # print parsed query then exit
```

### Keyword Mode

```bash
# Filtering
qk where FIELD=VALUE                    # equals
qk where FIELD!=VALUE                   # not equals
qk where FIELD>N                        # numeric greater than (>=  <  <= also work)
qk where FIELD gt N                     # word operator: greater than (shell-safe)
qk where FIELD gte N                    # word operator: >= (shell-safe)
qk where FIELD lt N                     # word operator: < (shell-safe)
qk where FIELD lte N                    # word operator: <= (shell-safe)
qk where FIELD~=PATTERN                 # regex match
qk where FIELD contains TEXT            # substring match
qk where FIELD exists                   # field presence check
qk where A=1 and B=2                    # AND
qk where A=1 or B=2                     # OR
qk where A=1, B=2                       # comma = AND (readable shorthand)
qk where A.B.C=VALUE                    # nested field (dot path)

# Field selection
qk select F1 F2 F3

# Counting
qk count                                # total count
qk count by FIELD                       # grouped count

# Aggregation
qk fields                               # discover all field names
qk sum FIELD                            # sum
qk avg FIELD                            # average
qk min FIELD                            # minimum
qk max FIELD                            # maximum

# Sorting / pagination
qk sort FIELD [asc|desc]
qk limit N
qk head N                               # alias for limit
```

### DSL Mode (First Argument Starts With `.` / `not ` / `|`)

```bash
# Filter expressions
qk '.f == "v"'                          # equals
qk '.f != "v"'                          # not equals
qk '.f > N'  '.f < N'  '.f >= N'  '.f <= N'
qk '.f exists'
qk '.f contains "text"'
qk '.f matches "regex"'
qk 'EXPR and EXPR'
qk 'EXPR or EXPR'
qk 'not EXPR'
qk '.a.b.c == 1'                        # nested field access (2+ levels)

# Pipeline stages
qk 'FILTER | pick(.f1, .f2)'           # keep only specified fields
qk 'FILTER | omit(.f1, .f2)'           # remove specified fields
qk 'FILTER | count()'                  # count records
qk 'FILTER | sort_by(.f desc)'         # sort
qk 'FILTER | group_by(.f)'             # group and count
qk 'FILTER | limit(N)'                 # first N records
qk 'FILTER | skip(N)'                  # skip N records
qk 'FILTER | dedup(.f)'                # deduplicate
qk 'FILTER | sum(.f)'                  # sum
qk 'FILTER | avg(.f)'                  # average
qk 'FILTER | min(.f)'                  # minimum
qk 'FILTER | max(.f)'                  # maximum

# Pass all records directly to pipeline (no filter)
qk '| count()'
qk '| group_by(.level)'
qk '| sort_by(.latency desc) | limit(10)'
```

### Input Formats (Auto-Detected, No Flags Required)

| Format     | Detection Criteria                                  |
| ---------- | --------------------------------------------------- |
| NDJSON     | Content starts with `{`, multiple lines             |
| JSON array | Content starts with `[`                             |
| YAML       | Starts with `---` / `.yaml` or `.yml` extension    |
| TOML       | `key = value` pattern / `.toml` extension           |
| CSV        | Comma-separated / `.csv` extension                  |
| TSV        | `.tsv` extension                                    |
| logfmt     | `key=value key=value` pattern                       |
| Gzip       | Magic bytes `0x1f 0x8b` / `.gz` (transparent decomp)|
| Plain text | Everything else                                     |
