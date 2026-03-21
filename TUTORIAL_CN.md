# qk 完整教程

本教程每个功能都提供**可直接复制粘贴运行**的例子，并标注预期输出。

---

## 目录

1. [安装](#安装)
2. [准备测试数据](#准备测试数据)
3. [基础用法](#基础用法)
4. [过滤（where）](#过滤where)
5. [选择字段（select）](#选择字段select)
6. [统计（count）](#统计count)
7. [排序（sort）](#排序sort)
8. [限制数量（limit / head）](#限制数量limit--head)
9. [数值聚合（sum / avg / min / max）](#数值聚合sum--avg--min--max)
10. [字段发现（fields）](#字段发现fields)
11. [DSL 表达式语法](#dsl-表达式语法)
12. [DSL 管道阶段](#dsl-管道阶段)
13. [qk + jq：处理 JSON 编码字符串](#qk--jq处理-json-编码字符串)
14. [输出格式（--fmt）](#输出格式---fmt)
15. [颜色输出（--color）](#颜色输出---color)
16. [多种文件格式](#多种文件格式)
17. [管道组合](#管道组合)
18. [常见问题](#常见问题)
19. [完整速查表](#完整速查表)

> **最新版本新增功能**：`startswith`、`endswith`、`glob` 算子；CSV 支持 `--no-header`；`--cast FIELD=TYPE` 类型强转；数值聚合遇到非数字值时自动输出警告。

---

## 安装

### 第一步：安装 Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
# 安装完成后重新打开终端，或运行：
source ~/.cargo/env
```

### 第二步：编译并安装 qk

```bash
git clone https://github.com/handsomevictor/qk.git
cd qk
cargo install --path .
```

验证安装：

```bash
qk --version
```

### 开发时不安装也能用

```bash
cargo run -- where level=error app.log
# 等价于安装后的：
qk where level=error app.log
```

---

## 准备测试数据

仓库中的 `tutorial/` 目录包含了所有 11 种支持格式的现成测试文件，无需手动创建。进入该目录后即可运行所有示例：

```bash
cd qk/tutorial    # 以下所有命令均在该目录下执行

# 验证文件是否正常 — 每条命令应输出记录总数：
qk count app.log           # 25 — NDJSON，2~3 层嵌套 JSON
qk count access.log        # 20 — NDJSON（包含 client/server 嵌套对象）
qk count k8s.log           # 20 — NDJSON（3 层嵌套：pod.labels.app/team）
qk count encoded.log       # 7  — NDJSON（字段值本身是 JSON 字符串）
qk count data.json         # 8  — JSON 数组
qk count services.yaml     # 6  — YAML 多文档
qk count config.toml       # 1  — TOML（整个文件作为一条记录）
qk count users.csv         # 15 — CSV
qk count events.tsv        # 20 — TSV
qk count services.logfmt   # 16 — logfmt（key=value，Go 服务常见格式）
qk count notes.txt         # 20 — 纯文本（每行 → {"line":"..."}）
qk count app.log.gz        # 25 — 透明 gzip 解压
```

**文件参考表：**

| 文件 | 格式 | 记录数 | 主要字段 |
|------|--------|---------|------------|
| `app.log` | NDJSON | 25 | `level service msg latency host context.region request.path response.status` |
| `access.log` | NDJSON | 20 | `method path status latency client.ip client.country server.host` |
| `k8s.log` | NDJSON | 20 | `level msg pod.name pod.namespace pod.labels.app pod.labels.team container.restart_count` |
| `encoded.log` | NDJSON | 7 | `service metadata payload`（值为 JSON 字符串） |
| `data.json` | JSON 数组 | 8 | `id name age city role active score address.country` |
| `services.yaml` | YAML | 6 | `name status replicas enabled port env resources.cpu` |
| `config.toml` | TOML | 1 | `server.port server.workers database.pool_max logging.level feature_flags.*` |
| `users.csv` | CSV | 15 | `name age city role active score department salary` |
| `events.tsv` | TSV | 20 | `ts event service severity region duration_ms user_id` |
| `services.logfmt` | logfmt | 16 | `ts level service msg host latency version` |
| `notes.txt` | 纯文本 | 20 | `line`（每行的完整文本） |
| `app.log.gz` | gzip | 25 | 同 `app.log` |
| `mixed.log` | NDJSON | 12 | 故意使用混合类型字段：`latency`（Number/String/null）、`score`（Number/String/null）、`active`（Bool/String）、`status`（Number） |

---

## 基础用法

### 显示所有记录

```bash
qk app.log
# → {"ts":"2024-01-01T10:00:00Z","level":"info","service":"api","msg":"server started","latency":0,...}
# → {"ts":"2024-01-01T10:01:00Z","level":"error","service":"api","msg":"connection timeout","latency":3001,...}
# → （共 25 条记录）
```

在终端中输出会有颜色：error=红，warn=黄，info=绿。

### 从 stdin 读取

```bash
echo '{"level":"error","msg":"oops"}' | qk
# → {"level":"error","msg":"oops"}
```

### 查看解析方式（--explain）

```bash
qk --explain where level=error app.log
# → mode:    Keyword
# → format:  Ndjson (detected)
# → query:   FastQuery { filters: [level = error], ... }
# → files:   ["app.log"]
```

`--explain` 标志会打印检测到的格式和解析后的查询，然后退出。

---

## 过滤（where）

### 等于（=）

```bash
qk where level=error app.log
# → {"ts":"2024-01-01T10:01:00Z","level":"error","service":"api","msg":"connection timeout","latency":3001,...}
# → {"ts":"2024-01-01T10:04:00Z","level":"error","service":"worker","msg":"panic: nil pointer","latency":0,...}
# → （所有 error 记录）
```

### 不等于（!=）

```bash
qk where level!=info app.log
# → {"ts":"2024-01-01T10:01:00Z","level":"error",...}
# → {"ts":"2024-01-01T10:02:00Z","level":"warn",...}
# → {"ts":"2024-01-01T10:04:00Z","level":"error",...}
# → （所有非 info 记录）
```

### 数值大于（>）

```bash
# 带引号写法（内嵌运算符需加引号）
qk where 'latency>100' app.log
# 单词运算符写法 — 无需引号，shell 安全
qk where latency gt 100 app.log
# → {"ts":"2024-01-01T10:01:00Z","level":"error","service":"api","msg":"connection timeout","latency":3001,...}
# → {"ts":"2024-01-01T10:02:00Z","level":"warn","service":"worker","msg":"queue depth high","latency":150,...}
# → （所有 latency > 100 的记录）
```

### 数值小于（<）

```bash
# 带引号写法
qk where 'latency<50' app.log
# 单词运算符写法 — shell 安全
qk where latency lt 50 app.log
# → {"ts":"2024-01-01T10:00:00Z","level":"info","service":"api","msg":"server started","latency":0,...}
# → {"ts":"2024-01-01T10:03:00Z","level":"info","service":"api","msg":"request ok","latency":42,...}
# → （所有 latency < 50 的记录）
```

### 大于等于（>=）

```bash
# 带引号写法
qk where 'status>=500' access.log
# 单词运算符写法 — shell 安全
qk where status gte 500 access.log
# → {"ts":"2024-01-01T10:02:00Z","method":"GET","path":"/api/orders","status":500,"latency":3200,...}
# → {"ts":"2024-01-01T10:04:00Z","method":"GET","path":"/api/users","status":503,"latency":9800,...}
# → （所有 5xx 响应）
```

### 小于等于（<=）

```bash
# 带引号写法
qk where 'latency<=42' app.log
# 单词运算符写法 — shell 安全
qk where latency lte 42 app.log
# → {"ts":"2024-01-01T10:00:00Z",...,"latency":0}
# → {"ts":"2024-01-01T10:03:00Z",...,"latency":42}
# → （所有 latency <= 42 的记录）
```

### 正则匹配（\~=）

> **zsh/bash 注意**：`*` 是 shell 中的通配符。请务必用引号包裹正则表达式，防止 shell 展开。

```bash
# 用引号包裹，防止 shell 展开 *
qk where 'msg~=.*timeout.*' app.log
# → {"ts":"2024-01-01T10:01:00Z","level":"error","service":"api","msg":"connection timeout","latency":3001,...}
# → {"ts":"2024-01-01T10:07:00Z","level":"error","service":"db","msg":"query timeout","latency":5001,...}

qk where 'msg~=pan.*pointer' app.log
# → （msg 匹配该正则的记录）
```

### 包含子字符串（contains）

```bash
qk where msg contains queue app.log
# → {"ts":"2024-01-01T10:02:00Z","level":"warn","service":"worker","msg":"queue depth high","latency":150,...}
```

### 前缀匹配（startswith）

```bash
qk where msg startswith connection app.log
# → {"level":"error","msg":"connection timeout",...}
# → （所有 msg 以 "connection" 开头的记录）

qk where path startswith /api/ access.log
# → （所有路径以 /api/ 开头的记录）

qk where name startswith Al users.csv
# → （Alice、Alex、Alfred 等）

qk where line startswith ERROR notes.txt
# → （以 "ERROR" 单词开头的行）
```

### 后缀匹配（endswith）

```bash
qk where path endswith users access.log
# → （所有路径以 "users" 结尾的记录，例如 /api/users）

qk where msg endswith timeout app.log
# → （msg 以 "timeout" 结尾的记录）

qk where name endswith son users.csv
# → （Jackson、Wilson 等）
```

### Shell 风格通配符（glob）

> **Shell 注意**：`*` 和 `?` 是 shell 元字符，请务必用单引号包裹 glob 表达式。

`glob` **不区分大小写** — `'*error*'` 同时匹配 `ERROR`、`Error` 和 `error`。

```bash
qk where msg glob '*timeout*' app.log
# → （msg 中任意位置包含 "timeout" 的所有记录 — 不区分大小写）

qk where name glob 'Al*' users.csv
# → Alice、Alex、Alfred 等（以 "Al" 开头，后接任意内容）

qk where name glob '*son' users.csv
# → Jackson、Wilson 等（以 "son" 结尾）

qk where path glob '/api/*' access.log
# → （所有 API 路径）

qk where line glob '*ERROR*' notes.txt
# → （包含 ERROR 的行 — 匹配 error、Error、ERROR）

# ? 匹配任意单个字符
qk where msg glob 'timeout?' app.log
# → （例如 "timeouts"、"timeout."）
```

**文本搜索算子对比：**

| 算子 | 示例 | 区分大小写？ | 说明 |
|----------|---------|----------------|-------|
| `contains` | `where msg contains timeout` | 是 | 简单子字符串匹配 |
| `startswith` | `where path startswith /api/` | 是 | 前缀检查 |
| `endswith` | `where path endswith users` | 是 | 后缀检查 |
| `glob` | `where msg glob '*timeout*'` | **否** | `*` = 任意字符，`?` = 单个字符 |
| `~=` | `where 'msg~=.*timeout.*'` | 取决于正则 | 完整正则，用 `(?i)` 实现不区分大小写 |

### 字段存在（exists）

```bash
# 找所有包含 error 字段的记录（注意：这是字段名，不是 level=error）
echo '{"level":"info","msg":"ok"}
{"level":"error","msg":"bad","error":"connection refused"}' | qk where error exists
# → {"level":"error","msg":"bad","error":"connection refused"}
```

### AND 多条件

```bash
qk where level=error and service=api app.log
# → {"ts":"2024-01-01T10:01:00Z","level":"error","service":"api","msg":"connection timeout","latency":3001,...}
# → （所有来自 api 服务的 error 记录）
```

### OR 多条件

```bash
qk where level=error or level=warn app.log
# → {"ts":"2024-01-01T10:01:00Z","level":"error",...}
# → {"ts":"2024-01-01T10:02:00Z","level":"warn",...}
# → {"ts":"2024-01-01T10:04:00Z","level":"error",...}
# → （所有 error 和 warn 记录）
```

### 逗号分隔符（可读的 AND）

逗号是 `and` 的别名，用逗号分隔条件可提高可读性：

```bash
qk where level=error, service=api app.log
# → {"level":"error","service":"api","msg":"connection timeout","latency":3001,...}

# 逗号也可作为独立标记
qk where level=error , service=api app.log

# 逗号与 and/or 混用（逗号等同于 and）
qk where level=error, latency gt 100 app.log
# → {"level":"error","latency":3001,...}
# → {"level":"error","latency":5001,...}
```

使用逗号前只能这样写：
`qk where level=error and service=api and latency gt 100 app.log`

使用逗号后：
`qk where level=error, service=api, latency gt 100 app.log`

### 嵌套字段访问（点号路径）

```bash
# 两层嵌套字段过滤
qk where response.status=503 app.log
# → {"level":"error","service":"api","msg":"upstream service unavailable","response":{"status":503,...},...}

# 嵌套数值字段使用单词运算符
qk where response.status gte 500 app.log
qk where 'response.status>=500' app.log

# 访问 context（两层嵌套）
qk where context.region=us-east app.log

# 三层嵌套：Kubernetes 日志中的 pod.labels.app
qk where pod.labels.app=api k8s.log
qk where pod.labels.team=platform, level=error k8s.log
```

---

## 选择字段（select）

### 只保留指定字段

```bash
qk where level=error select ts service msg app.log
```

预期输出（只有 3 个字段，其余去掉）：

```
{"ts":"2024-01-01T10:01:00Z","service":"api","msg":"connection timeout"}
{"ts":"2024-01-01T10:04:00Z","service":"worker","msg":"panic: nil pointer"}
```

### 不过滤只选字段

```bash
qk select level msg app.log
```

预期输出（所有 6 条，但只保留 level 和 msg）：

```
{"level":"info","msg":"server started"}
{"level":"error","msg":"connection timeout"}
{"level":"warn","msg":"queue depth high"}
{"level":"info","msg":"request ok"}
{"level":"error","msg":"panic: nil pointer"}
{"level":"info","msg":"page loaded"}
```

---

## 统计（count）

### 统计总数

```bash
qk count app.log
```

预期输出：

```
{"count":6}
```

### 过滤后统计

```bash
qk where level=error count app.log
```

预期输出：

```
{"count":2}
```

### 按字段分组统计

```bash
qk count by level app.log
```

预期输出（按数量降序排列）：

```
{"level":"info","count":3}
{"level":"error","count":2}
{"level":"warn","count":1}
```

### 另一个字段分组

```bash
qk count by service app.log
```

预期输出：

```
{"service":"api","count":3}
{"service":"worker","count":2}
{"service":"web","count":1}
```

### 先过滤再分组

```bash
qk where latency>0 count by service app.log
```

预期输出（latency>0 只有 3 条，排除了 latency=0 的记录）：

```
{"service":"api","count":1}
{"service":"worker","count":1}
{"service":"web","count":1}
```

### 按时间分桶统计

将事件分组到固定时间窗口。使用时间后缀：`s`（秒）、`m`（分钟）、`h`（小时）、`d`（天）。
时间戳字段默认为 `ts`。

```bash
# 按 5 分钟分桶（读取 ts 字段）
qk count by 5m app.log
# → {"bucket":"2024-01-15T10:00:00Z","count":3}
# → {"bucket":"2024-01-15T10:05:00Z","count":5}
# → {"bucket":"2024-01-15T10:10:00Z","count":2}

# 按 1 小时分桶
qk count by 1h app.log
# → {"bucket":"2024-01-15T10:00:00Z","count":42}

# 指定字段名（字段不叫 ts 时）
qk count by 1h timestamp app.log

# 先过滤再分桶
qk where level=error, count by 5m app.log
```

时间戳可以是：
- RFC 3339 字符串：`"2024-01-15T10:05:30Z"` 或带时区偏移 `"2024-01-15T18:05:30+08:00"`
- Unix epoch 秒（整数 ≥ 1 000 000 000）
- Unix epoch 毫秒（整数 ≥ 1 000 000 000 000）

缺少时间戳字段或无法解析的记录会被静默跳过，不影响其他记录。

#### DSL 等价写法

```bash
qk '| group_by_time(.ts, "5m")' app.log
# → 与 'count by 5m app.log' 输出相同

qk '| group_by_time(.timestamp, "1h")' events.ndjson
```

---

## 排序（sort）

### 按数值降序（最大在前）

```bash
qk sort latency desc app.log
```

预期输出（latency 从高到低）：

```
{"ts":"...","level":"error","service":"api","msg":"connection timeout","latency":3001}
{"ts":"...","level":"warn","service":"worker","msg":"queue depth high","latency":150}
{"ts":"...","level":"info","service":"web","msg":"page loaded","latency":88}
{"ts":"...","level":"info","service":"api","msg":"request ok","latency":42}
{"ts":"...","level":"info","service":"api","msg":"server started","latency":0}
{"ts":"...","level":"error","service":"worker","msg":"panic: nil pointer","latency":0}
```

### 按数值升序（最小在前）

```bash
qk sort latency asc app.log
```

预期输出（latency 从低到高）：

```
{"ts":"...","latency":0}   ← 两条 latency=0
{"ts":"...","latency":0}
{"ts":"...","latency":42}
...
```

### 按字符串排序

```bash
qk sort service app.log
```

预期输出（service 字母序）：

```
{"service":"api",...}
{"service":"api",...}
{"service":"api",...}
{"service":"web",...}
{"service":"worker",...}
{"service":"worker",...}
```

### 组合：先过滤再排序

```bash
qk where level=error sort latency desc app.log
```

预期输出（2 条 error，按 latency 降序）：

```
{"ts":"...","level":"error","service":"api","msg":"connection timeout","latency":3001}
{"ts":"...","level":"error","service":"worker","msg":"panic: nil pointer","latency":0}
```

---

## 限制数量（limit / head）

### 取前 N 条

```bash
qk limit 3 app.log
```

预期输出（前 3 条）：

```
{"ts":"2024-01-01T10:00:00Z","level":"info",...}
{"ts":"2024-01-01T10:01:00Z","level":"error",...}
{"ts":"2024-01-01T10:02:00Z","level":"warn",...}
```

### head 是 limit 的别名

```bash
qk head 2 app.log
```

预期输出（前 2 条，和 limit 2 完全相同）：

```
{"ts":"2024-01-01T10:00:00Z","level":"info",...}
{"ts":"2024-01-01T10:01:00Z","level":"error",...}
```

### 组合：排序后取 Top N

```bash
qk sort latency desc limit 3 app.log
```

预期输出（延迟最高的 3 条）：

```
{"latency":3001,...}
{"latency":150,...}
{"latency":88,...}
```

---

## 数值聚合（sum / avg / min / max）

### 求和

```bash
qk sum latency app.log
```

预期输出（0+3001+150+42+0+88 = 3281）：

```
{"sum":3281}
```

### 过滤后求和

```bash
qk where level=error sum latency app.log
```

预期输出（3001+0 = 3001）：

```
{"sum":3001}
```

### 平均值

```bash
qk avg latency app.log
```

预期输出（3281 / 6 ≈ 546.83）：

```
{"avg":546.833333}
```

### 先过滤再平均

```bash
qk where latency>0 avg latency app.log
```

预期输出（排除 latency=0 后的平均，3 条：3001+150+42+88 = 3281，但 latency>0 只有 3 条：3001,150,42,88 = 4 条）：

实际有 4 条 latency>0（3001、150、42、88），平均 = 3281/4 = 820.25：

```
{"avg":820.25}
```

### 最小值

```bash
qk min latency app.log
```

预期输出：

```
{"min":0}
```

### 最小值（排除零）

```bash
qk where latency>0 min latency app.log
```

预期输出（最小的非零延迟）：

```
{"min":42}
```

### 最大值

```bash
qk max latency app.log
```

预期输出：

```
{"max":3001}
```

### HTTP 最差响应时间

```bash
qk where status>=500 max latency access.log
```

预期输出（5xx 中最慢的）：

```
{"max":9800}
```

---

## 字段发现（fields）

### 发现所有字段名

```bash
qk fields app.log
```

预期输出（按字母排序）：

```
{"field":"latency"}
{"field":"level"}
{"field":"msg"}
{"field":"service"}
{"field":"ts"}
```

### 先过滤再发现（error 记录有哪些字段）

```bash
qk where level=error fields app.log
```

预期输出（和全量一样，说明 error 记录字段完整）：

```
{"field":"latency"}
{"field":"level"}
{"field":"msg"}
{"field":"service"}
{"field":"ts"}
```

### 不同格式文件的字段发现

```bash
qk fields access.log
```

预期输出：

```
{"field":"latency"}
{"field":"method"}
{"field":"path"}
{"field":"status"}
{"field":"ts"}
```

### 结合 count 看有多少个字段

```bash
qk fields app.log | qk count
```

预期输出：

```
{"count":5}
```

---

## DSL 表达式语法

当第一个参数以 `.`、`not `  或 `|` 开头时，自动进入 DSL 模式。

### 等于

```bash
qk '.level == "error"' app.log
```

预期输出（2 条 error）：

```
{"ts":"2024-01-01T10:01:00Z","level":"error","service":"api","msg":"connection timeout","latency":3001}
{"ts":"2024-01-01T10:04:00Z","level":"error","service":"worker","msg":"panic: nil pointer","latency":0}
```

### 不等于

```bash
qk '.level != "info"' app.log
```

预期输出（3 条，排除 info）：

```
{"level":"error",...}
{"level":"warn",...}
{"level":"error",...}
```

### 数值比较

```bash
qk '.latency > 100' app.log
```

预期输出：

```
{"latency":3001,...}
{"latency":150,...}
```

```bash
qk '.latency >= 88' app.log
```

预期输出（88、150、3001 这 3 条）：

```
{"latency":88,...}
{"latency":150,...}
{"latency":3001,...}
```

### 布尔值

```bash
echo '{"service":"api","enabled":true}
{"service":"worker","enabled":false}' | qk '.enabled == true'
```

预期输出：

```
{"service":"api","enabled":true}
```

### null 比较

```bash
echo '{"service":"api","error":null}
{"service":"web"}
{"service":"worker","error":"timeout"}' | qk '.error != null'
```

预期输出（null 和字段不存在都被排除，只保留有实际值的）：

```
{"service":"worker","error":"timeout"}
```

### 字段存在（exists）

```bash
qk '.latency exists' app.log
```

预期输出（所有记录都有 latency 字段，全部输出）：

```
（全部 6 条）
```

### 包含子字符串（contains）

```bash
qk '.msg contains "timeout"' app.log
```

预期输出：

```
{"ts":"2024-01-01T10:01:00Z","level":"error","service":"api","msg":"connection timeout","latency":3001}
```

### 正则匹配（matches）

```bash
qk '.msg matches "pan.*pointer"' app.log
```

预期输出：

```
{"ts":"2024-01-01T10:04:00Z","level":"error","service":"worker","msg":"panic: nil pointer","latency":0}
```

### AND

```bash
qk '.level == "error" and .service == "api"' app.log
```

预期输出（1 条）：

```
{"ts":"2024-01-01T10:01:00Z","level":"error","service":"api","msg":"connection timeout","latency":3001}
```

### OR

```bash
qk '.level == "error" or .level == "warn"' app.log
```

预期输出（3 条）：

```
{"level":"error",...}
{"level":"warn",...}
{"level":"error",...}
```

### NOT

```bash
qk 'not .level == "info"' app.log
```

预期输出（3 条，等同于 != info）：

```
{"level":"error",...}
{"level":"warn",...}
{"level":"error",...}
```

### 复合逻辑

```bash
qk '.latency > 100 and (.level == "error" or .level == "warn")' app.log
```

预期输出（latency>100 且是 error 或 warn，2 条）：

```
{"level":"error","latency":3001,...}
{"level":"warn","latency":150,...}
```

### 嵌套字段 — 两层深度

```bash
# 在嵌套字段上过滤
qk where response.status=503 app.log
# → {"level":"error","service":"api","msg":"upstream service unavailable","response":{"status":503,...},...}

# 嵌套数值字段使用单词运算符
qk where response.status gte 500 app.log
qk where 'response.status>=500' app.log

# 选择嵌套字段
qk where response.status=503 select service response.status response.error app.log

# 按嵌套字段统计
qk count by response.status app.log
qk count by context.region app.log
```

### 嵌套字段 — 三层深度

```bash
# context.region 是两层；request.headers.x-trace 是三层
qk where context.region=us-east app.log
qk where context.env=prod, level=error app.log

# DSL — 三层访问
qk '.request.headers.x-trace exists' app.log
qk '.request.headers.user-agent contains "Mozilla"' app.log

# Kubernetes 日志：pod.labels.app 是三层深度
qk where pod.labels.app=api k8s.log
qk where pod.labels.team=platform, level=error k8s.log

# 更深层：container 信息
qk where 'container.restart_count gt 2' k8s.log
qk where container.restart_count gt 2, level=warn k8s.log
```

### 嵌套字段 — DSL 模式

```bash
# 过滤深层嵌套字段，只保留所需字段
qk '.response.status >= 500 | pick(.ts, .service, .response.status, .response.error)' app.log

# 按嵌套字段分组
qk '| group_by(.context.region)' app.log
qk '| group_by(.response.status)' app.log

# 对嵌套数值进行聚合
qk '.response.status >= 200 | avg(.latency)' app.log
qk '.response.status >= 500 | max(.latency)' app.log

# DSL 中的三层访问
qk '.pod.labels.app == "api" | group_by(.level)' k8s.log
qk '.pod.labels.team == "platform" and .level == "error"' k8s.log
qk '.container.restart_count > 5 | pick(.ts, .pod.name, .container.restart_count, .reason)' k8s.log
```

### 不加过滤（全部通过）

```bash
qk '| count()' app.log
```

预期输出（`|` 开头 = 不过滤，直接进管道阶段）：

```
{"count":6}
```

---

## DSL 管道阶段

### pick（只保留字段）

```bash
qk '.level == "error" | pick(.ts, .service, .msg)' app.log
```

预期输出（3 个字段，latency 被去掉）：

```
{"ts":"2024-01-01T10:01:00Z","service":"api","msg":"connection timeout"}
{"ts":"2024-01-01T10:04:00Z","service":"worker","msg":"panic: nil pointer"}
```

### omit（去掉字段）

```bash
qk '.level == "error" | omit(.ts, .latency)' app.log
```

预期输出（ts 和 latency 被去掉）：

```
{"level":"error","service":"api","msg":"connection timeout"}
{"level":"error","service":"worker","msg":"panic: nil pointer"}
```

### count（统计数量）

```bash
qk '.level == "error" | count()' app.log
```

预期输出：

```
{"count":2}
```

### sort\_by（排序）

```bash
qk '.latency > 0 | sort_by(.latency desc)' app.log
```

预期输出（latency>0 的记录，从高到低排序）：

```
{"latency":3001,...}
{"latency":150,...}
{"latency":88,...}
{"latency":42,...}
```

```bash
qk '.latency > 0 | sort_by(.latency asc)' app.log
```

预期输出（从低到高）：

```
{"latency":42,...}
{"latency":88,...}
{"latency":150,...}
{"latency":3001,...}
```

### group\_by（分组统计）

```bash
qk '| group_by(.level)' app.log
```

预期输出（按数量降序）：

```
{"level":"info","count":3}
{"level":"error","count":2}
{"level":"warn","count":1}
```

```bash
qk '.level == "error" | group_by(.service)' app.log
```

预期输出：

```
{"service":"api","count":1}
{"service":"worker","count":1}
```

### limit（取前 N 条）

```bash
qk '.latency >= 0 | sort_by(.latency desc) | limit(3)' app.log
```

预期输出（latency 最高的 3 条）：

```
{"latency":3001,...}
{"latency":150,...}
{"latency":88,...}
```

### skip（跳过前 N 条，分页）

```bash
qk '.latency >= 0 | sort_by(.latency desc) | skip(2)' app.log
```

预期输出（跳过最高的 2 条，从第 3 条开始）：

```
{"latency":88,...}
{"latency":42,...}
{"latency":0,...}
{"latency":0,...}
```

### skip + limit 组合分页

```bash
# 第 1 页（第 1-2 条）
qk '.latency >= 0 | sort_by(.latency desc) | limit(2)' app.log
# 第 2 页（第 3-4 条）
qk '.latency >= 0 | sort_by(.latency desc) | skip(2) | limit(2)' app.log
# 第 3 页（第 5-6 条）
qk '.latency >= 0 | sort_by(.latency desc) | skip(4) | limit(2)' app.log
```

第 2 页预期输出：

```
{"latency":88,...}
{"latency":42,...}
```

### dedup（去重）

```bash
qk '| dedup(.service)' app.log
```

预期输出（每个 service 只保留第一次出现的那条）：

```
{"service":"api",...}   ← api 的第一条
{"service":"worker",...} ← worker 的第一条
{"service":"web",...}   ← web 的第一条
```

```bash
# 去重后统计有多少个不同的 service
qk '| dedup(.service) | count()' app.log
```

预期输出：

```
{"count":3}
```

### sum（求和）

```bash
qk '.latency >= 0 | sum(.latency)' app.log
```

预期输出（所有 latency 总和：0+3001+150+42+0+88 = 3281）：

```
{"sum":3281}
```

### avg（平均值）

```bash
qk '.latency > 0 | avg(.latency)' app.log
```

预期输出（4 条非零 latency 的平均：(3001+150+42+88)/4 = 820.25）：

```
{"avg":820.25}
```

### min（最小值）

```bash
qk '.latency > 0 | min(.latency)' app.log
```

预期输出（非零 latency 中最小的）：

```
{"min":42}
```

### max（最大值）

```bash
qk '.latency > 0 | max(.latency)' app.log
```

预期输出：

```
{"max":3001}
```

### 链式管道（多阶段组合）

```bash
# 过滤 error → 按 latency 降序 → 只保留关键字段
qk '.level == "error" | sort_by(.latency desc) | pick(.ts, .service, .msg, .latency)' app.log
```

预期输出：

```
{"ts":"2024-01-01T10:01:00Z","service":"api","msg":"connection timeout","latency":3001}
{"ts":"2024-01-01T10:04:00Z","service":"worker","msg":"panic: nil pointer","latency":0}
```

```bash
# 全部记录 → 分组 → 取 top 2 组
qk '| group_by(.service) | limit(2)' app.log
```

预期输出（出现最多的 2 个 service）：

```
{"service":"api","count":3}
{"service":"worker","count":2}
```

```bash
# 过滤慢请求 → 去重（每个服务只看一次）→ 只保留关键字段
qk '.latency > 50 | dedup(.service) | pick(.service, .latency, .msg)' app.log
```

预期输出：

```
{"service":"api","latency":3001,"msg":"connection timeout"}
{"service":"worker","latency":150,"msg":"queue depth high"}
{"service":"web","latency":88,"msg":"page loaded"}
```

---

## qk + jq：处理 JSON 编码字符串

有时字段的**值**本身就是一个 JSON 字符串（双重编码）：

```json
{"service":"api","metadata":"{\"region\":\"us-east\",\"env\":\"prod\"}","payload":"{\"level\":\"error\",\"code\":500}"}
```

qk 无法深入解析字符串内部——它将 `metadata` 视为普通字符串。解决方案是将 qk 与 jq 组合使用。由于 qk 输出 NDJSON，这两个工具可以自然地组合。

### 先解码嵌套字符串，再用 qk 查询

```bash
cat > encoded.log << 'EOF'
{"service":"api","metadata":"{\"region\":\"us-east\",\"env\":\"prod\"}","payload":"{\"level\":\"error\",\"code\":500}","ts":"2024-01-01T10:01:00Z"}
{"service":"worker","metadata":"{\"region\":\"us-west\",\"env\":\"staging\"}","payload":"{\"level\":\"info\",\"code\":200}","ts":"2024-01-01T10:02:00Z"}
{"service":"web","metadata":"{\"region\":\"us-east\",\"env\":\"prod\"}","payload":"{\"level\":\"warn\",\"code\":429}","ts":"2024-01-01T10:03:00Z"}
{"service":"db","metadata":"{\"region\":\"us-east\",\"env\":\"prod\"}","payload":"{\"level\":\"error\",\"code\":503}","ts":"2024-01-01T10:04:00Z"}
EOF

# 第一步：用 jq 将字符串字段解码为真正的对象
# 第二步：管道到 qk，在解码后的字段上过滤
cat encoded.log | jq -c '.payload = (.payload | fromjson)' | qk where payload.level=error
# → {"service":"api","metadata":"...","payload":{"level":"error","code":500},"ts":"..."}
# → {"service":"db","metadata":"...","payload":{"level":"error","code":503},"ts":"..."}
```

### 同时解码多个字符串字段

```bash
cat encoded.log | jq -c '{service, ts, payload: (.payload | fromjson), meta: (.metadata | fromjson)}' \
  | qk where meta.env=prod, payload.level=error
# → {"service":"api","ts":"...","payload":{"level":"error","code":500},"meta":{"region":"us-east","env":"prod"}}
# → {"service":"db","ts":"...","payload":{"level":"error","code":503},"meta":{"region":"us-east","env":"prod"}}
```

### qk 先过滤，jq 再深入

```bash
# qk 在顶层字段上做快速过滤，jq 提取编码的子字段
cat encoded.log | qk where service=api | jq -r '.payload | fromjson | .code'
# → 500
```

### 完整管道：qk 过滤 → jq 解码 → qk 聚合

```bash
# 三阶段管道：qk 按 service 预过滤 → jq 解码 payload → qk 按解码后的 level 统计
cat encoded.log \
  | qk where metadata contains prod \
  | jq -c '.payload = (.payload | fromjson)' \
  | qk count by payload.level
# → {"payload.level":"error","count":2}
# → {"payload.level":"warn","count":1}
```

### 什么时候用 qk、jq 还是两者结合

| 场景 | 工具 |
|-----------|------|
| 字段是真正的 JSON 对象（嵌套） | 单独使用 qk 即可 |
| 字段的**值**是 JSON 编码字符串 | 先用 `jq ... \| fromjson` 解码，再用 qk |
| 对百万条记录快速过滤，然后解码 | 先 qk（快速），再 jq（精确） |
| 复杂重整 / 数学运算 / 条件逻辑 | jq |
| 统计、聚合、表格输出 | qk |

---

## 输出格式（--fmt）

> **必须将** **`--fmt`** **放在查询之前！**
> ✅ `qk --fmt table where level=error app.log`
> ❌ `qk where level=error --fmt table app.log`

### ndjson（默认）

```bash
qk --fmt ndjson where level=error app.log
```

预期输出（每行一个 JSON，和默认一样）：

```
{"ts":"2024-01-01T10:01:00Z","level":"error","service":"api","msg":"connection timeout","latency":3001}
{"ts":"2024-01-01T10:04:00Z","level":"error","service":"worker","msg":"panic: nil pointer","latency":0}
```

### pretty（缩进 JSON，替代 jq .）

```bash
qk --fmt pretty where level=error app.log
```

预期输出（缩进格式，块间空行）：

```
{
  "ts": "2024-01-01T10:01:00Z",
  "level": "error",
  "service": "api",
  "msg": "connection timeout",
  "latency": 3001
}

{
  "ts": "2024-01-01T10:04:00Z",
  "level": "error",
  "service": "worker",
  "msg": "panic: nil pointer",
  "latency": 0
}
```

### pretty + color（带语义颜色的漂亮打印）

```bash
qk --fmt pretty --color where level=error app.log
```

（在终端中：键名加粗青色，字符串绿色，数字黄色，null 暗淡）

### table（对齐表格）

```bash
qk --fmt table where level=error app.log
```

预期输出（自动对齐，列名加粗）：

```
 ts                       level   service  msg                   latency
 2024-01-01T10:01:00Z     error   api      connection timeout    3001
 2024-01-01T10:04:00Z     error   worker   panic: nil pointer    0
```

### table + 选字段

```bash
qk --fmt table where level=error select ts service msg app.log
```

预期输出（只有 3 列）：

```
 ts                       service  msg
 2024-01-01T10:01:00Z     api      connection timeout
 2024-01-01T10:04:00Z     worker   panic: nil pointer
```

### csv（可用 Excel 打开）

```bash
qk --fmt csv where level=error app.log
```

预期输出（第一行是列名）：

```
latency,level,msg,service,ts
3001,error,connection timeout,api,2024-01-01T10:01:00Z
0,error,panic: nil pointer,worker,2024-01-01T10:04:00Z
```

### csv 导出到文件

```bash
qk --fmt csv where level=error app.log > errors.csv
cat errors.csv
```

### raw（原始行，不重新序列化）

```bash
qk --fmt raw where level=error app.log
```

预期输出（原始的那一行文本，字段顺序和原文件完全相同）：

```
{"ts":"2024-01-01T10:01:00Z","level":"error","service":"api","msg":"connection timeout","latency":3001}
{"ts":"2024-01-01T10:04:00Z","level":"error","service":"worker","msg":"panic: nil pointer","latency":0}
```

### DSL + pretty

```bash
qk --fmt pretty '.level == "error" | pick(.service, .msg, .latency)' app.log
```

预期输出：

```
{
  "service": "api",
  "msg": "connection timeout",
  "latency": 3001
}

{
  "service": "worker",
  "msg": "panic: nil pointer",
  "latency": 0
}
```

---

## 颜色输出（--color）

### 默认行为

- **终端**：自动开启颜色
- **管道**（`qk ... | other`）：自动关闭颜色

### 强制开启颜色（管道给 less）

```bash
qk --color where level=error app.log | less -R
```

（`less -R` 渲染 ANSI 颜色，`--color` 强制 qk 输出颜色码）

### 强制关闭颜色

```bash
qk --no-color where level=error app.log
```

输出纯文本，无任何颜色码，适合写入文件或给不支持颜色的工具处理。

### 通过环境变量禁用（NO\_COLOR 标准）

```bash
NO_COLOR=1 qk where level=error app.log
```

### 优先级验证

```bash
# --no-color 优先于 --color，输出无颜色
qk --no-color --color where level=error app.log
```

### 颜色方案（NDJSON 输出）

| 字段 / 值                       | 颜色       |
| ---------------------------- | -------- |
| 字段名（所有键）                     | 粗体青色     |
| `level: "error"` / `"fatal"` | **粗体红色** |
| `level: "warn"`              | **粗体黄色** |
| `level: "info"`              | **粗体绿色** |
| `level: "debug"`             | 蓝色       |
| `level: "trace"`             | 暗淡       |
| `msg` / `message` 的值         | 亮白色      |
| `ts` / `timestamp` 的值        | 暗淡       |
| `error` / `exception` 字段的值   | 红色       |
| HTTP `status` 200–299        | 绿色       |
| HTTP `status` 300–399        | 青色       |
| HTTP `status` 400–499        | 黄色       |
| HTTP `status` 500–599        | **粗体红色** |
| 数字（其他字段）                     | 黄色       |
| 布尔值                          | 洋红色      |
| null                         | 暗淡       |

---

## 多种文件格式

`qk` 自动检测格式，无需指定参数。以下所有示例均使用 `tutorial/` 目录中的文件。

### JSON 数组（data.json）

```bash
# JSON 数组中的每个元素变为一条记录
qk data.json
# → {"id":1,"name":"Alice","age":30,"city":"New York","role":"admin",...}
# → （共 8 条记录）

qk where role=admin data.json
# → （role 为 admin 的记录）

qk where address.country=US data.json
# → （嵌套字段访问 — 两层深度）

qk count by role data.json
# → {"role":"viewer","count":4}
# → {"role":"admin","count":3}
# → {"role":"editor","count":2}（按数量降序排列）

qk sort score desc limit 3 data.json
# → score 最高的前 3 条
```

### YAML 格式（多文档，services.yaml）

```bash
# 每个 --- 文档变为一条记录；共 6 个服务
qk services.yaml
qk where status=running services.yaml
# → （status=running 的服务）

qk where enabled=true services.yaml
# → （仅启用的服务）

qk count by status services.yaml
# → {"status":"running","count":4}
# → {"status":"stopped","count":1}
# → {"status":"degraded","count":1}

qk select name status replicas services.yaml
# → {"name":"api-gateway","status":"running","replicas":3}
# → （6 条记录，只保留 name/status/replicas）
```

### TOML 格式（config.toml）

```bash
# 整个文件 = 一条记录；嵌套节可通过点号访问
qk config.toml
# → （包含所有配置值的一条记录）

# 访问嵌套节字段
qk select server.port server.workers database.pool_max config.toml
# → {"server.port":8080,"server.workers":4,"database.pool_max":50}

qk '.server.port > 8000' config.toml
# → （该记录，因为 server.port 是 8080）

qk '.logging.level == "info"' config.toml
# → （该记录）
```

### CSV 格式（users.csv）

```bash
# 表头行成为字段名；共 15 个用户
# 数值列自动强转：age=30 存储为 Number(30) 而非 String("30")
# 类 null 的单元格（"None"、"null"、"NA"、"N/A"、""）存储为 null — 在 avg/sum 中跳过
qk users.csv

qk where role=admin users.csv
qk where city=New\ York users.csv     # 转义空格
qk where department=Engineering users.csv
qk where score gt 90 users.csv        # 有效：score 是 Number 而非 String
qk where age lt 30 users.csv
qk where name startswith Al users.csv
qk where name endswith son users.csv
qk where name glob 'Al*' users.csv    # 不区分大小写：Alice、Alex、Alfred...

qk count by role users.csv
# → {"role":"viewer","count":5}
# → {"role":"editor","count":5}
# → {"role":"admin","count":3} ...

qk count by department users.csv
qk sort score desc users.csv
qk sort salary desc limit 5 users.csv
qk where role=admin select name city score salary users.csv

# 统计
qk avg score users.csv
qk max salary users.csv
qk sum salary users.csv
qk where department=Engineering avg salary users.csv
```

#### 无表头 CSV（--no-header）

当 CSV 文件没有表头行时，使用 `--no-header`。列会自动命名为 `col1`、`col2`、`col3` 等。

> `--no-header` 必须放在查询表达式**之前**（`clap trailing_var_arg` 语义）。

```bash
# 示例：没有表头的 CSV 文件
# （从 users.csv 中去掉表头行来创建测试文件）
tail -n +2 users.csv > users_no_header.csv

# --no-header 自动生成 col1、col2、col3... 作为字段名
qk --no-header users_no_header.csv
# → {"col1":"1","col2":"Alice","col3":30,"col4":"New York","col5":"admin",...}

# 查看前 5 行以了解列的布局
qk --no-header head 5 users_no_header.csv

# 了解各列含义后，按列索引过滤
qk --no-header where col5=admin users_no_header.csv      # col5 = role
qk --no-header where col4=Engineering users_no_header.csv  # col4 = department

# 数值比较有效（类型强转仍然生效）
qk --no-header where col3 lt 30 users_no_header.csv      # col3 = age

# 按列聚合
qk --no-header count by col5 users_no_header.csv          # 按 role 统计
qk --no-header sort col8 desc limit 5 users_no_header.csv # 按 salary 排序

# 无表头模式下的类型强转
# "None"、"null"、"NA"、""、"NaN" → 存储为 null（数值操作中跳过）
# 看起来像整数的单元格 → 存储为 Number（支持 gt/lt/avg/sum）
# "true"/"false" → 存储为 Bool
```

**null 类值的处理方式：**

| CSV 单元格值 | 存储为 | 行为 |
|----------------|-----------|---------|
| `30`、`1000` | `Number` | 支持 `gt`/`lt`/`avg`/`sum` |
| `true`、`false` | `Bool` | 支持 `=true`/`=false` |
| `""`、`None`、`null`、`NA`、`N/A`、`NaN` | `null` | 数值操作中跳过；`exists` 返回 false |
| `"New York"`、`"api"` | `String` | 支持 `=`/`contains`/`glob` |

### TSV 格式（events.tsv）

```bash
# 制表符分隔；通过 .tsv 扩展名自动检测；共 20 条事件
qk events.tsv

qk where severity=error events.tsv
qk where event=login events.tsv
qk where region=us-east events.tsv
qk where duration_ms gt 1000 events.tsv

qk count by event events.tsv
qk count by severity events.tsv
qk count by region events.tsv
qk where severity=error count by event events.tsv

qk sort duration_ms desc limit 5 events.tsv
qk avg duration_ms events.tsv
qk where severity=error avg duration_ms events.tsv
qk max duration_ms events.tsv
```

### logfmt 格式（services.logfmt）

```bash
# Go 服务（Logrus、Zap、zerolog）常见的 key=value 格式；共 16 条记录
qk services.logfmt

qk where level=error services.logfmt
qk where service=api services.logfmt
qk where latency gt 1000 services.logfmt
qk where level=error, service=db services.logfmt
qk where msg contains timeout services.logfmt

qk count by level services.logfmt
# → {"level":"info","count":7}
# → {"level":"error","count":4}
# → {"level":"warn","count":4}
# → {"level":"debug","count":1}

qk where level=error select ts service msg services.logfmt
qk avg latency services.logfmt
qk sort latency desc limit 5 services.logfmt
```

### Gzip 压缩文件（app.log.gz）

```bash
# 无需 gunzip — qk 通过魔数字节检测自动透明解压
qk where level=error app.log.gz
# → （与直接查询 app.log 相同的 error 记录）

qk count app.log.gz
# → {"count":25}

# 交叉验证：压缩版和未压缩版输出应完全一致
qk count by level app.log
qk count by level app.log.gz
# → （两者输出完全相同）
```

### 纯文本（notes.txt）

文件中的每一行变为一条记录，包含单个字段：`{"line": "..."}`。所有查询中使用 `line` 作为字段名。

```bash
# 查看所有行
qk notes.txt
# → {"line":"2024-01-01 10:00 [INFO] api server started on port 8080"}
# → （共 20 条记录）

# 查看前 N 行（类似 head -N）
qk head 5 notes.txt
qk limit 3 notes.txt

# 统计总行数
qk count notes.txt
# → {"count":20}
```

#### 子字符串匹配（区分大小写）

```bash
qk where line contains error notes.txt
qk where line contains timeout notes.txt
qk where line contains WARN notes.txt      # 仅匹配大写 WARN
```

#### 前缀匹配 / 后缀匹配

```bash
qk where line startswith 2024 notes.txt
qk where line startswith ERROR notes.txt
qk where line startswith '[WARN]' notes.txt

qk where line endswith ok notes.txt
qk where line endswith done notes.txt
```

#### Shell 风格通配符（glob — 不区分大小写，记得加引号）

```bash
# glob 不区分大小写：*ERROR* 也匹配 error、Error、ERROR
qk where line glob '*ERROR*' notes.txt
qk where line glob '*warn*' notes.txt       # 匹配 WARN、Warn、warn
qk where line glob '*timeout*' notes.txt
qk where line glob '2024*ERROR*' notes.txt  # 以 2024 开头且包含 ERROR
qk where line glob '*[WARN]*' notes.txt     # 字面括号（由 glob_to_regex 转义）
qk where line glob 'ERROR?*' notes.txt      # ERROR 后接任意单字符，再接任意内容
```

#### 正则（完整模式支持 — 记得加引号）

```bash
# 用引号防止 shell 展开 * 和 ?
qk where 'line~=.*error.*' notes.txt
qk where 'line~=.*\[ERROR\].*' notes.txt     # 字面括号
qk where 'line~=(WARN|ERROR)' notes.txt      # 或逻辑
qk where 'line~=^\d{4}-\d{2}-\d{2}' notes.txt  # 以日期开头的行
qk where 'line~=(?i)error' notes.txt         # 不区分大小写的正则
```

#### 组合多个条件

```bash
qk where line contains error, line startswith 2024 notes.txt
# → 包含 "error" 且以 "2024" 开头的行
```

#### 全文搜索能力总结

| 功能 | 命令 | 说明 |
|---------|---------|-------|
| 关键字搜索 | `where line contains TEXT` | 区分大小写 |
| 前缀匹配 | `where line startswith PREFIX` | 区分大小写 |
| 后缀匹配 | `where line endswith SUFFIX` | 区分大小写 |
| 通配符搜索 | `where line glob '*PATTERN*'` | 不区分大小写；`*` 需加引号 |
| 正则搜索 | `where 'line~=PATTERN'` | 始终加引号；用 `(?i)` 实现不区分大小写 |
| 统计匹配行数 | `where line contains X count notes.txt` | |
| 查看前 N 行 | `head N notes.txt` | 等同于 `head -N` |
| **不支持** | 模糊搜索 | 可用 `~=` 配合 `(?i)` 作为替代方案 |

### 混合类型字段与类型强转

现实中的日志文件，同一个字段在不同记录中的值类型往往不同——例如 `latency` 字段通常是数字，但某些记录中是 `"None"` 或 `null`；或者 `status` 字段在一个数据源是数字，在另一个数据源是字符串。

`tutorial/mixed.log` 专为演示这种情况而设计，包含 12 条记录，字段类型故意设置为混合：
- `latency`：大多数为 `Number`，但也有 `"None"`、`"NA"`、`"unknown"` 和 `null`
- `score`：大多数为 `Number`，但也有 `null`、`"N/A"` 和 `"pending"`
- `active`：大多数为 `Bool`，但也有字符串 `"yes"` 和 `"no"`
- `status`：始终为 `Number`

#### 默认行为（不使用 --cast）

```bash
qk count mixed.log
# → {"count":12}

# 数值聚合自动处理混合值：
# - Number 值 → 参与计算
# - null / "None" / "NA" / "N/A" / "NaN" / "" → 静默跳过（视为 null）
# - 无法解析的字符串如 "unknown" / "pending" → 跳过并向 stderr 输出警告
qk avg latency mixed.log
# stdout: {"avg":1199.625}
# stderr: [qk warning] field 'latency': value "unknown" is not numeric (line 5, mixed.log) — skipped

# 过滤：非数值的 latency 行不匹配数值比较，静默排除
qk where latency gt 100 mixed.log     # "None"、null、"unknown" 行静默排除
qk where latency gt 100, count mixed.log

# 警告输出到 stderr — 管道给其他工具不受影响
qk avg latency mixed.log 2>/dev/null  # 屏蔽警告，只保留 JSON 输出
qk avg latency mixed.log | jq '.avg'  # 警告在 stderr，jq 处理 stdout
```

**警告规则总结：**

| 字段值 | 数值操作中（avg/sum/gt/lt） | 输出警告？ |
|-------------|-------------------------------|---------|
| `3001`（Number） | 正常使用 | 否 |
| `"3001"`（可解析为数字的 String） | 正常使用 | 否 |
| `null` | 静默跳过 | 否 |
| `"None"` / `"NA"` / `"N/A"` / `"NaN"` / `""` | 静默跳过 | 否 |
| `"unknown"` / `"pending"` / `"abc"` | 跳过 | **是 — 警告到 stderr** |

#### --cast：查询前强制类型转换

`--cast FIELD=TYPE` 在查询运行前将字段转换为指定类型。必须放在查询表达式**之前**。

**支持的类型：**

| 类型 | 别名 | 作用 |
|------|---------|-------------|
| `number` | `num`、`float`、`int`、`integer` | 字符串解析为 Number；类 null 字符串 → Null；其他字符串 → 警告并移除字段 |
| `string` | `str`、`text` | 转为 String：`200` → `"200"`，`true` → `"true"`，`null` → `"null"` |
| `bool` | `boolean` | `"true"/"1"/"yes"/"on"` → true；`"false"/"0"/"no"/"off"` → false；其他 → 警告并移除 |
| `null` | `none` | 强制将字段置为 null（实际上使其从数值操作中消失） |
| `auto` | | CSV 风格推断：数字、布尔值、类 null 值、字符串 |

```bash
# --cast latency=number：显式强转；"None"/"NA" → Null，"unknown" → 警告并跳过
qk --cast latency=number avg latency mixed.log
# → {"avg":1199.625}
# stderr: [qk warning] --cast latency=number: value "unknown" is not numeric (line 5) — field skipped

# --cast status=string：将 Number 200 转为 String "200"
# 现在可以对 status 使用文本算子（contains、startswith、glob）
qk --cast status=string where status contains 20 mixed.log    # 匹配 200、201
qk --cast status=string where status startswith 5 mixed.log   # 匹配 500、503、504
qk --cast status=string where status glob '5??' mixed.log     # 5xx 状态码

# --cast active=bool：将 "yes"/"no" 字符串强转为 Bool；支持 =true/=false 过滤
qk --cast active=bool where active=true mixed.log
qk --cast active=bool count by active mixed.log

# 多个 --cast 标志（每个接受一个 FIELD=TYPE）
qk --cast latency=number --cast score=number avg latency mixed.log
qk --cast latency=number --cast score=number where latency gt 100, score gt 7.0 mixed.log

# --cast score=auto：CSV 风格推断
# "N/A" → Null，"9.5" → Number(9.5)，"pending" → String("pending")
qk --cast score=auto avg score mixed.log
```

#### 实际应用场景

```bash
# Python 日志中 None 被输出为字符串 "None"
# 不使用 --cast：avg 会对 "None" 值发出警告
# 使用 --cast："None" → Null 静默处理，无警告
qk --cast latency=number avg latency app.log

# 混合数字和字符串状态码的日志管道
qk --cast status=string count by status access.log

# CSV 中某列应该是数字但含有一些文本哨兵值
# 使用 --cast 获得正确的数字，并对错误行输出警告
qk --cast age=number avg age users.csv

# 强制字段为 null 以将其从聚合中排除
qk --cast score=null avg latency mixed.log  # score 完全忽略
```

### 同时查询多个文件和多种格式

```bash
# 文件并行处理 — 输出合并；每个文件自动检测格式
qk where level=error app.log k8s.log services.logfmt
qk count by level app.log k8s.log
qk where level=error count by service app.log k8s.log
```

### 通配符

```bash
# Shell 展开通配符；qk 并行处理所有匹配文件
qk where level=error *.log
qk count *.log
```

---

## 管道组合

### 两个 qk 串联

```bash
# 先过滤 error，再按 service 统计
qk where level=error app.log | qk count by service
# → （按 service 分组的 error 记录）
```

### 三级管道

```bash
# 过滤 → 排序 → 限制
qk where level=error app.log | qk sort latency desc | qk limit 1
# → （latency 最高的那条 error 记录）
```

### 配合 jq

```bash
# qk 过滤，jq 做后续处理
qk where level=error app.log | jq '.latency'
# → 3001
# → 0
# → （所有 error 记录的 latency 值）
```

### 配合 grep

```bash
# qk 按格式过滤，grep 做精确文本匹配
qk where service=api app.log | grep timeout
```

### 处理最近的日志条目

```bash
# 处理日志文件最后 1000 行
tail -n 1000 /path/to/app.log | qk where level=error

# 处理最后 500 行并按 service 统计
tail -n 500 /path/to/app.log | qk count by service
```

> **已知限制：** `tail -f file | qk ...` 会**无限阻塞**。
> qk 需要读到 stdin 的 EOF 才开始处理，因此暂不支持实时流式处理（`tail -f`）。
> 临时替代方案：使用 `tail -n <行数>` 处理有限输入。

---

## 常见问题

### Q: `--fmt` 没有生效，输出还是 NDJSON？

标志必须在查询之前：

```bash
# ✅ 正确
qk --fmt table where level=error app.log

# ❌ 错误（--fmt 会被当成文件名）
qk where level=error --fmt table app.log
```

### Q: DSL 里字符串比较为什么要加引号？

关键字模式的 `=` 右边直接写值，DSL 的 `==` 右边需要 JSON 引号：

```bash
# 关键字模式：不加引号
qk where level=error app.log

# DSL 模式：字符串要加双引号
qk '.level == "error"' app.log
```

### Q: 过滤出 null 值的记录？

```bash
# 字段存在但值为 null
echo '{"service":"api","error":null}
{"service":"web","error":"timeout"}' | qk '.error == null'
```

预期输出：

```
{"service":"api","error":null}
```

### Q: 颜色在 less 里显示不出来？

```bash
qk --color where level=error app.log | less -R
```

必须同时用 `--color`（强制输出颜色码）和 `less -R`（渲染颜色码）。

### Q: 输出到文件时不想要颜色

```bash
qk --no-color where level=error app.log > filtered.log
```

### Q: 如何查看有多少条记录满足条件？

```bash
# 方法一：关键字语法
qk where level=error count app.log

# 方法二：DSL 语法
qk '.level == "error" | count()' app.log
```

两者预期输出相同：

```
{"count":7}
```

### Q: 如何在不加引号的情况下使用数值运算符？

使用单词运算符代替符号运算符，无需任何引号：

```bash
# 符号运算符在大多数 shell 中需要加引号
qk where 'latency>=100' app.log
qk where 'status>=500' access.log

# 单词运算符始终 shell 安全，无需引号
qk where latency gte 100 app.log
qk where status gte 500 access.log
qk where latency gt 100 app.log      # >
qk where latency lt 100 app.log      # <
qk where latency lte 100 app.log     # <=
```

---

## 完整速查表

### 全局标志（必须放在查询之前）

```bash
qk --fmt ndjson   # NDJSON（默认）
qk --fmt pretty   # 缩进 JSON
qk --fmt table    # 对齐表格
qk --fmt csv      # CSV
qk --fmt raw      # 原始行
qk --color        # 强制开启颜色
qk --no-color     # 强制关闭颜色
qk --no-header    # 将 CSV/TSV 第一行视为数据；列命名为 col1、col2...
qk --cast FIELD=TYPE  # 查询前强制类型转换（可多次使用）
qk --explain      # 打印解析结果后退出
```

### 关键字模式

```bash
# 过滤
qk where FIELD=VALUE                    # 等于
qk where FIELD!=VALUE                   # 不等于
qk where FIELD>N                        # 数值大于（>=  <  <=  同理）
qk where FIELD gt N                     # 单词运算符：大于（shell 安全）
qk where FIELD gte N                    # 单词运算符：>=（shell 安全）
qk where FIELD lt N                     # 单词运算符：<（shell 安全）
qk where FIELD lte N                    # 单词运算符：<=（shell 安全）
qk where FIELD contains TEXT            # 子字符串匹配（区分大小写）
qk where FIELD startswith PREFIX        # 前缀匹配（区分大小写）
qk where FIELD endswith SUFFIX          # 后缀匹配（区分大小写）
qk where 'FIELD glob PATTERN'           # shell 通配符：* 任意字符，? 单个字符（不区分大小写）
qk where 'FIELD~=PATTERN'              # 正则匹配（始终加引号！）
qk where FIELD exists                   # 字段存在检查
qk where A=1 and B=2                    # AND
qk where A=1 or B=2                     # OR
qk where A=1, B=2                       # 逗号 = AND（可读简写）
qk where A.B.C=VALUE                    # 嵌套字段（点号路径）

# 选字段
qk select F1 F2 F3

# 统计
qk count                                # 总数
qk count by FIELD                       # 分组统计

# 聚合
qk fields                               # 发现所有字段名
qk sum FIELD                            # 求和
qk avg FIELD                            # 平均
qk min FIELD                            # 最小
qk max FIELD                            # 最大

# 排序 / 分页
qk sort FIELD [asc|desc]
qk limit N
qk head N                               # 同 limit
```

### DSL 模式（第一个参数以 `.` / `not `  / `|` 开头）

```bash
# 过滤表达式
qk '.f == "v"'                          # 等于
qk '.f != "v"'                          # 不等于
qk '.f > N'  '.f < N'  '.f >= N'  '.f <= N'
qk '.f exists'
qk '.f contains "text"'
qk '.f matches "regex"'
qk 'EXPR and EXPR'
qk 'EXPR or EXPR'
qk 'not EXPR'
qk '.a.b.c == 1'                        # 嵌套字段

# 管道阶段
qk 'FILTER | pick(.f1, .f2)'           # 只保留字段
qk 'FILTER | omit(.f1, .f2)'           # 去掉字段
qk 'FILTER | count()'                  # 统计
qk 'FILTER | sort_by(.f desc)'         # 排序
qk 'FILTER | group_by(.f)'             # 分组统计
qk 'FILTER | limit(N)'                 # 前 N 条
qk 'FILTER | skip(N)'                  # 跳过 N 条
qk 'FILTER | dedup(.f)'                # 去重
qk 'FILTER | sum(.f)'                  # 求和
qk 'FILTER | avg(.f)'                  # 平均
qk 'FILTER | min(.f)'                  # 最小
qk 'FILTER | max(.f)'                  # 最大

# 不过滤直接进管道（| 开头）
qk '| count()'
qk '| group_by(.level)'
qk '| sort_by(.latency desc) | limit(10)'
```

### 输入格式（自动检测，无需指定）

| 格式      | 检测依据                         |
| ------- | ---------------------------- |
| NDJSON  | 内容以 `{` 开头，多行                |
| JSON 数组 | 内容以 `[` 开头                   |
| YAML    | `---` 开头 / `.yaml` `.yml`    |
| TOML    | `key = value` / `.toml`      |
| CSV     | 逗号分隔 / `.csv`                |
| TSV     | `.tsv`                       |
| logfmt  | `key=value key=value`        |
| Gzip    | 魔数 `0x1f 0x8b` / `.gz`（透明解压） |
| 纯文本     | 其他所有格式                       |

