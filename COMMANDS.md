# COMMANDS — Quick Copy-Paste Reference

All runnable commands. **Create the test data first**, then copy any block below.

---

## Test Data Setup

```bash
# app.log — 25 service logs with 2–3 level nested JSON
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
# access.log — 20 HTTP access logs with nested client/server objects
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
# k8s.log — 20 Kubernetes pod events with 3-level nested JSON
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
{"ts":"2024-01-01T10:12:00Z","level":"info","msg":"hpa triggered","pod":{"name":"api-v2-uv7w8","namespace":"production","node":"node-01","labels":{"app":"api","version":"2.4.1","team":"platform"}},"container":{"name":"api","image":"company/api:2.4.1","restart_count":0},"reason":"CPU utilization 82% > target 70%","replicas":{"desired":6,"current":4,"available":4}}
{"ts":"2024-01-01T10:13:00Z","level":"error","msg":"secret not found","pod":{"name":"payments-xy9z0","namespace":"production","node":"node-02","labels":{"app":"payments","version":"2.1.0","team":"billing"}},"container":{"name":"payments","image":"company/payments:2.1.0","restart_count":2},"reason":"secret payments-db-creds not found in namespace production"}
{"ts":"2024-01-01T10:14:00Z","level":"info","msg":"readiness probe passed","pod":{"name":"api-v2-9k2l3","namespace":"production","node":"node-04","labels":{"app":"api","version":"2.4.1","team":"platform"}},"container":{"name":"api","image":"company/api:2.4.1","restart_count":0},"reason":"HTTP 200 from /ready","probe":{"type":"readiness","path":"/ready","port":8080}}
{"ts":"2024-01-01T10:15:00Z","level":"warn","msg":"network policy violation","pod":{"name":"worker-abc12","namespace":"production","node":"node-01","labels":{"app":"worker","version":"1.9.0","team":"data"}},"container":{"name":"worker","image":"company/worker:1.9.0","restart_count":0},"reason":"Egress blocked to 0.0.0.0/0 port 443"}
{"ts":"2024-01-01T10:16:00Z","level":"info","msg":"cron job completed","pod":{"name":"cleanup-job-1234","namespace":"production","node":"node-05","labels":{"app":"cleanup","version":"1.0.0","team":"platform"}},"container":{"name":"cleanup","image":"company/cleanup:1.0.0","restart_count":0},"reason":"Completed","job":{"name":"daily-cleanup","duration_s":45,"items_deleted":1204}}
{"ts":"2024-01-01T10:17:00Z","level":"error","msg":"resource quota exceeded","pod":{"name":"batch-ab3c4","namespace":"staging","node":"","labels":{"app":"batch","version":"1.2.0","team":"data"}},"container":{"name":"batch","image":"company/batch:1.2.0","restart_count":0},"reason":"exceeded quota: requests.memory 4Gi > limit 3Gi","quota":{"namespace":"staging","resource":"requests.memory","used":"4Gi","limit":"3Gi"}}
{"ts":"2024-01-01T10:18:00Z","level":"info","msg":"node joined cluster","pod":{"name":"","namespace":"kube-system","node":"node-06","labels":{"app":"kubelet","version":"1.27.0","team":"infra"}},"container":{"name":"","image":"","restart_count":0},"reason":"Node node-06 registered successfully"}
{"ts":"2024-01-01T10:19:00Z","level":"warn","msg":"certificate expiring soon","pod":{"name":"ingress-ef5g6","namespace":"kube-system","node":"node-01","labels":{"app":"ingress-nginx","version":"1.9.0","team":"infra"}},"container":{"name":"controller","image":"ingress-nginx:1.9.0","restart_count":0},"reason":"TLS certificate for api.company.com expires in 14 days","cert":{"domain":"api.company.com","issuer":"Lets Encrypt","expires":"2024-02-14"}}
EOF
```

```bash
# encoded.log — JSON string values (for qk + jq examples)
cat > encoded.log << 'EOF'
{"service":"api","ts":"2024-01-01T10:01:00Z","metadata":"{\"region\":\"us-east\",\"env\":\"prod\"}","payload":"{\"level\":\"error\",\"code\":500,\"msg\":\"upstream timeout\"}"}
{"service":"worker","ts":"2024-01-01T10:02:00Z","metadata":"{\"region\":\"us-west\",\"env\":\"staging\"}","payload":"{\"level\":\"info\",\"code\":200,\"msg\":\"job complete\"}"}
{"service":"web","ts":"2024-01-01T10:03:00Z","metadata":"{\"region\":\"us-east\",\"env\":\"prod\"}","payload":"{\"level\":\"warn\",\"code\":429,\"msg\":\"rate limit hit\"}"}
{"service":"db","ts":"2024-01-01T10:04:00Z","metadata":"{\"region\":\"us-east\",\"env\":\"prod\"}","payload":"{\"level\":\"error\",\"code\":503,\"msg\":\"connection pool exhausted\"}"}
{"service":"auth","ts":"2024-01-01T10:05:00Z","metadata":"{\"region\":\"eu-west\",\"env\":\"prod\"}","payload":"{\"level\":\"info\",\"code\":200,\"msg\":\"token refreshed\"}"}
EOF
```

---

## Basic

```bash
qk app.log
echo '{"level":"error","msg":"oops"}' | qk
qk --explain where level=error app.log
qk fields app.log
```

---

## Filtering (where)

```bash
# Equals / not equals
qk where level=error app.log
qk where level!=info app.log

# Numeric comparisons — word operators (shell-safe, no quoting)
qk where latency gt 100 app.log
qk where latency lt 50 app.log
qk where latency gte 88 app.log
qk where latency lte 42 app.log

# Or quote the embedded operators
qk where 'latency>100' app.log
qk where 'latency<50' app.log
qk where 'status>=500' access.log
qk where 'latency<=42' app.log

# Regex / substring
qk where msg~=.*timeout.* app.log
qk where msg contains timeout app.log
qk where msg contains queue app.log
qk where reason contains failed k8s.log

# Field existence
qk where request exists app.log
qk where response.error exists app.log
qk where metrics exists app.log

# AND — three equivalent styles
qk where level=error and service=api app.log
qk where level=error, service=api app.log
qk where level=error , service=api app.log

# OR
qk where level=error or level=warn app.log

# Comma-separated conditions (readable multi-condition style)
qk where service=api, level=error app.log
qk where level=error, latency gt 100 app.log
qk where service=api, latency gt 40, level=info app.log
qk where msg contains queue, level=warn app.log
qk where context.region=us-east, level=error app.log
qk where pod.labels.team=platform, level=error k8s.log
qk where container.restart_count gt 2, level=warn k8s.log
```

---

## Nested Field Access (2 Levels)

```bash
# Filter on nested field
qk where response.status=503 app.log
qk where response.status=200 app.log
qk where context.region=us-east app.log
qk where context.env=prod app.log
qk where client.country=US access.log
qk where server.region=us-west access.log

# Word operators on nested numeric fields
qk where response.status gte 500 app.log
qk where 'response.status>=500' app.log
qk where latency gt 1000, response.status gte 500 app.log

# Select nested fields
qk where response.status gte 500 select service response.status response.error app.log
qk where client.country=US select method path status client.ip access.log

# Count by nested field
qk count by response.status app.log
qk count by context.region app.log
qk count by server.region access.log
qk count by client.country access.log
qk count by pod.labels.team k8s.log
```

---

## Nested Field Access (3 Levels)

```bash
# 3-level dot path: pod.labels.app, request.headers.x-trace
qk where pod.labels.app=api k8s.log
qk where pod.labels.team=platform k8s.log
qk where pod.labels.team=infra, level=error k8s.log
qk where pod.namespace=production k8s.log
qk where pod.namespace=staging k8s.log

# Header-level access in app.log
qk where request.headers.x-trace exists app.log
qk where request.headers.user-agent contains Mozilla app.log

# Filter + select on 3-level fields
qk where pod.labels.team=platform select ts level msg pod.name container.restart_count k8s.log

# Count by 3-level field
qk count by pod.labels.app k8s.log
qk count by pod.labels.team k8s.log
qk count by pod.namespace k8s.log
```

---

## DSL — Nested Fields

```bash
# 2-level filter
qk '.response.status >= 500' app.log
qk '.context.region == "us-east"' app.log
qk '.client.country != "US"' access.log

# 3-level filter
qk '.pod.labels.app == "api"' k8s.log
qk '.pod.labels.team == "platform" and .level == "error"' k8s.log
qk '.request.headers.x-trace exists' app.log
qk '.request.headers.user-agent contains "Mozilla"' app.log

# Filter + pick nested fields
qk '.response.status >= 500 | pick(.ts, .service, .response.status, .response.error)' app.log
qk '.level == "error" | pick(.ts, .pod.name, .pod.labels.app, .container.restart_count, .reason)' k8s.log

# Group by nested field
qk '| group_by(.context.region)' app.log
qk '| group_by(.response.status)' app.log
qk '| group_by(.pod.labels.team)' k8s.log
qk '| group_by(.pod.namespace)' k8s.log

# Aggregate on nested numeric
qk '.response.status >= 200 | avg(.latency)' app.log
qk '.response.status >= 500 | max(.latency)' app.log
qk '.container.restart_count > 0 | sum(.container.restart_count)' k8s.log

# Deep 3-level chain
qk '.pod.labels.team == "platform" | group_by(.level)' k8s.log
qk '.container.restart_count > 2 | pick(.ts, .pod.name, .container.restart_count, .reason)' k8s.log
qk '.pod.namespace == "production" and .level == "error" | sort_by(.ts desc)' k8s.log
```

---

## Field Selection (select)

```bash
qk where level=error select ts service msg app.log
qk select level msg app.log
qk select ts service msg latency app.log
qk where response.status gte 500 select ts service response.status response.error app.log
qk where pod.labels.team=platform select ts level msg pod.name reason k8s.log
```

---

## Counting (count)

```bash
qk count app.log
qk where level=error count app.log
qk count by level app.log
qk count by service app.log
qk where latency gt 0 count by service app.log
qk count by context.region app.log
qk count by response.status app.log
qk count by pod.labels.team k8s.log
qk count by pod.namespace k8s.log
qk count by client.country access.log
```

---

## Sorting (sort)

```bash
qk sort latency desc app.log
qk sort latency asc app.log
qk sort service app.log
qk where level=error sort latency desc app.log
qk sort latency desc limit 5 app.log
qk where 'response.status>=400' sort latency desc access.log
```

---

## Limit / Head

```bash
qk limit 3 app.log
qk head 5 app.log
qk sort latency desc limit 3 app.log
qk where level=error sort latency desc head 1 app.log
```

---

## Numeric Aggregation (sum / avg / min / max)

```bash
qk sum latency app.log
qk avg latency app.log
qk min latency app.log
qk max latency app.log

qk where level=error sum latency app.log
qk where latency gt 0 avg latency app.log
qk where latency gt 0 min latency app.log
qk where latency gt 0 max latency app.log

qk where 'response.status>=500' max latency app.log
qk where 'response.status>=500' avg latency access.log
qk where pod.labels.team=platform avg latency k8s.log
```

---

## Field Discovery (fields)

```bash
qk fields app.log
qk fields access.log
qk fields k8s.log
qk where level=error fields app.log
qk fields app.log | qk count
```

---

## DSL Expression Syntax

```bash
# Basic comparisons
qk '.level == "error"' app.log
qk '.level != "info"' app.log
qk '.latency > 100' app.log
qk '.latency >= 88' app.log
qk '.latency < 50' app.log
qk '.latency <= 42' app.log

# Substring / regex
qk '.msg contains "timeout"' app.log
qk '.msg matches "pan.*pointer"' app.log
qk '.reason contains "failed"' k8s.log

# Exists
qk '.request exists' app.log
qk '.response.error exists' app.log

# Logic
qk '.level == "error" and .service == "api"' app.log
qk '.level == "error" or .level == "warn"' app.log
qk 'not .level == "info"' app.log
qk '.latency > 100 and (.level == "error" or .level == "warn")' app.log

# No filter — straight to pipeline
qk '| count()' app.log
qk '| group_by(.level)' app.log
qk '| group_by(.service)' app.log
```

---

## DSL Pipeline Stages

```bash
# pick / omit
qk '.level == "error" | pick(.ts, .service, .msg)' app.log
qk '.level == "error" | omit(.ts, .latency)' app.log

# count
qk '.level == "error" | count()' app.log

# sort_by
qk '.latency > 0 | sort_by(.latency desc)' app.log
qk '.latency > 0 | sort_by(.latency asc)' app.log

# group_by
qk '| group_by(.level)' app.log
qk '| group_by(.service)' app.log
qk '| group_by(.context.region)' app.log
qk '| group_by(.pod.labels.team)' k8s.log

# limit / skip
qk '.latency >= 0 | sort_by(.latency desc) | limit(3)' app.log
qk '.latency >= 0 | sort_by(.latency desc) | skip(2)' app.log
qk '.latency >= 0 | sort_by(.latency desc) | skip(0) | limit(3)' app.log
qk '.latency >= 0 | sort_by(.latency desc) | skip(3) | limit(3)' app.log

# dedup
qk '| dedup(.service)' app.log
qk '| dedup(.service) | count()' app.log
qk '| dedup(.pod.labels.app)' k8s.log

# sum / avg / min / max
qk '.latency >= 0 | sum(.latency)' app.log
qk '.latency > 0 | avg(.latency)' app.log
qk '.latency > 0 | min(.latency)' app.log
qk '.latency > 0 | max(.latency)' app.log

# Multi-stage chains
qk '.level == "error" | sort_by(.latency desc) | pick(.ts, .service, .msg, .latency)' app.log
qk '| group_by(.service) | limit(3)' app.log
qk '.latency > 50 | dedup(.service) | pick(.service, .latency, .msg)' app.log
qk '.pod.labels.team == "platform" | group_by(.level)' k8s.log
qk '.level == "error" | sort_by(.ts desc) | pick(.ts, .pod.name, .reason) | limit(5)' k8s.log
```

---

## Output Formats (--fmt must come FIRST)

```bash
qk --fmt ndjson where level=error app.log
qk --fmt pretty where level=error app.log
qk --fmt pretty --color where level=error app.log
qk --fmt table where level=error app.log
qk --fmt table where level=error select ts service msg app.log
qk --fmt table count by level app.log
qk --fmt csv where level=error app.log
qk --fmt csv where level=error app.log > errors.csv
qk --fmt raw where level=error app.log

qk --fmt pretty '.level == "error" | pick(.service, .msg, .latency)' app.log
qk --fmt table '| group_by(.level)' app.log
qk --fmt table '| group_by(.pod.labels.team)' k8s.log
```

---

## Color

```bash
qk --color where level=error app.log | less -R
qk --no-color where level=error app.log
NO_COLOR=1 qk where level=error app.log
```

---

## Multiple Formats (auto-detected)

```bash
# logfmt
cat > app.logfmt << 'EOF'
level=info service=api msg="server started" latency=0
level=error service=api msg="connection timeout" latency=3001
level=warn service=worker msg="queue depth high" latency=150
EOF
qk where level=error app.logfmt
qk count by level app.logfmt

# CSV
cat > data.csv << 'EOF'
name,age,city,dept
alice,30,NYC,engineering
bob,25,SF,design
carol,35,NYC,engineering
dave,28,NYC,product
EOF
qk where city=NYC data.csv
qk count by dept data.csv

# YAML
cat > services.yaml << 'EOF'
---
name: api
port: 8080
enabled: true
team: platform
---
name: worker
port: 9090
enabled: false
team: data
---
name: web
port: 3000
enabled: true
team: frontend
EOF
qk where enabled=true services.yaml
qk count by team services.yaml

# TOML
cat > config.toml << 'EOF'
port = 8080
host = "localhost"
debug = false
max_connections = 100
EOF
qk config.toml
qk '.port > 8000' config.toml

# Gzip
gzip -k app.log
qk where level=error app.log.gz

# Plain text
cat > notes.txt << 'EOF'
error: connection refused at 10:01
info: server started at 10:00
error: timeout after 30s at 10:07
warning: disk usage at 90%
EOF
qk where line contains error notes.txt
qk count notes.txt

# Multiple files at once
qk where level=error app.log app.logfmt
qk where level=error *.log
```

---

## qk + jq: Handling JSON-Encoded String Fields

When a field's **value** is itself a JSON string, use `jq | fromjson` to decode it, then pipe to `qk`:

```bash
# Decode one field, then filter with qk
cat encoded.log | jq -c '.payload = (.payload | fromjson)' | qk where payload.level=error

# Decode both fields, filter on decoded content
cat encoded.log | jq -c '{service, ts, payload: (.payload | fromjson), meta: (.metadata | fromjson)}' \
  | qk where meta.env=prod, payload.level=error

# qk first (fast pre-filter) → jq decodes → qk aggregates
cat encoded.log | qk where metadata contains prod \
  | jq -c '.payload = (.payload | fromjson)' \
  | qk count by payload.level

# Extract a single value from encoded field
cat encoded.log | qk where service=api | jq -r '.payload | fromjson | .code'

# Filter by service with qk, then pretty-print decoded payload with jq
cat encoded.log | qk where service=api | jq '.payload | fromjson'
```

---

## Pipeline Composition

```bash
# Two qk commands chained
qk where level=error app.log | qk count by service

# Three stages
qk where level=error app.log | qk sort latency desc | qk limit 1

# With jq
qk where level=error app.log | jq '.latency'
qk where level=error app.log | jq '{service: .service, ms: .latency}'

# With grep
qk where service=api app.log | grep timeout

# Live log tailing
tail -f /var/log/app.log | qk where level=error
tail -f /var/log/app.log | qk where level=error | qk count by service
```
