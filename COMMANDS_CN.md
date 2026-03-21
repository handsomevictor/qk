# 命令速查手册 — 可直接复制粘贴的命令

所有可运行的命令。**无需额外配置** — 测试文件位于 `tutorial/` 目录。只需先执行 `cd tutorial`。

```bash
git clone https://github.com/handsomevictor/qk.git
cd qk && cargo install --path .
cd tutorial      # 以下所有命令均在此目录下执行
```

***

## 混合类型字段（类型不匹配处理）

当某个数值字段在不同记录中包含非数值内容时，qk 的处理方式如下：

| 记录中的值                                        | 在 `avg`/`sum`/`gt`/`lt` 中的行为 | 是否输出警告？ |
| -------------------------------------------- | ---------------------------- | ------- |
| `3001`（Number）                               | 正常使用                         | 否       |
| `"3001"`（String）                             | 自动解析为 Number                 | 否       |
| `null`                                       | 静默跳过                         | 否       |
| `"None"` / `"NA"` / `"N/A"` / `"NaN"` / `""` | 视为 null，静默跳过                 | 否       |
| `"unknown"` / `"error"` / `"abc"`            | 跳过 — **警告输出到 stderr**        | **是**   |

```bash
# mixed.log 中 latency 字段混合了 "None"、"unknown"、null 和真实数字
qk avg latency mixed.log
# → {"avg":1199.625}
# stderr: [qk warning] field 'latency': value "unknown" is not numeric (line 5, mixed.log) — skipped

# null 和 "None" 静默跳过 — 不输出警告
qk count mixed.log    # → 12 条记录
qk where latency gt 100 mixed.log   # latency 为 "None"/null 的行直接被排除
```

### 强制类型转换（--cast FIELD=TYPE）

`--cast` 在查询执行前将字段转换为指定类型。必须放在查询表达式**之前**。

**支持的类型：**

| 类型       | 别名                               | 行为                                                                                      |
| -------- | -------------------------------- | --------------------------------------------------------------------------------------- |
| `number` | `num`, `float`, `int`, `integer` | 将字符串解析为 Number；类 null 值 → Null；无法解析 → **警告 + 删除字段**                                     |
| `string` | `str`, `text`                    | 转换为 String：`200` → `"200"`，`true` → `"true"`，`null` → `"null"`                          |
| `bool`   | `boolean`                        | 将 `"true"/"1"/"yes"/"on"` 解析为 true；`"false"/"0"/"no"/"off"` 解析为 false；其他值 → **警告 + 删除** |
| `null`   | `none`                           | 不论原始值为何，强制将字段设为 null                                                                    |
| `auto`   | <br />                           | CSV 风格自动推断：数字、布尔值、类 null 值、字符串                                                          |

```bash
# --cast latency=number：将字符串类型的 latency 转换为 Number；无法解析时输出警告
qk --cast latency=number avg latency mixed.log
# → {"avg":1199.625}
# stderr: [qk warning] --cast latency=number: value "unknown" is not numeric (line 5) — field skipped

# --cast status=string：将数值类型的 status 转换为 String — 从而支持文本操作符
qk --cast status=string where status contains 20 mixed.log    # 匹配 200, 201
qk --cast status=string where status startswith 5 mixed.log   # 匹配 500, 503, 504

# --cast active=bool：将 "yes"/"no" 字符串转换为 Bool
qk --cast active=bool count by active mixed.log

# 多个 --cast 标志（每个标志指定一个 FIELD=TYPE）
qk --cast latency=number --cast score=number avg latency mixed.log

# --cast score=auto：自动推断类型（与 CSV 的 coerce_value 行为相同）
# "N/A" → Null, "9.5" → 9.5, "pending" → String("pending")
qk --cast score=auto avg score mixed.log
```

***

## 验证所有格式均可正常解析

```bash
qk count app.log          # 25 条记录 — NDJSON，嵌套 JSON
qk count access.log       # 20 条记录 — NDJSON，嵌套 client/server
qk count k8s.log          # 20 条记录 — NDJSON，三层嵌套
qk count encoded.log      # 7  条记录 — NDJSON，字段值为 JSON 字符串
qk count data.json        # 8  条记录 — JSON 数组
qk count services.yaml    # 6  条记录 — YAML 多文档
qk count config.toml      # 1  条记录  — TOML（整个文件视为一条记录）
qk count users.csv        # 15 条记录 — CSV
qk count events.tsv       # 20 条记录 — TSV
qk count services.logfmt  # 16 条记录 — logfmt（key=value 格式）
qk count notes.txt        # 20 条记录 — 纯文本（每行 → {"line":"..."}）
qk count app.log.gz       # 25 条记录 — 透明 gzip 解压
qk count mixed.log        # 12 条记录 — NDJSON，包含故意混入的混合类型字段
```

***

## 基本用法

```bash
# 输出所有记录（用于检查格式和数量）
qk app.log
qk data.json

# 从 stdin 管道输入
echo '{"level":"error","msg":"oops","service":"api"}' | qk
cat app.log | qk where level=error

# 查看文件中所有字段名
qk fields app.log
qk fields users.csv
qk fields k8s.log

# 查看 qk 的解析过程（调试模式）
qk --explain where level=error app.log
qk --explain where latency gt 100 app.log
```

***

## 过滤（where）

### 等值匹配

```bash
qk where level=error app.log
qk where level!=info app.log
qk where service=api app.log
qk where method=POST access.log
qk where role=admin users.csv
qk where severity=error events.tsv
```

### 数值比较（单词操作符 — 对 shell 友好，无需引号）

```bash
# 单词操作符：gt lt gte lte（永远不需要 shell 引号）
qk where latency gt 1000 app.log
qk where latency lt 100 app.log
qk where latency gte 3001 app.log
qk where latency lte 50 app.log
qk where status gte 500 access.log
qk where status lt 400 access.log
qk where score gt 90 users.csv
qk where age gte 35 users.csv
qk where duration_ms gt 1000 events.tsv

# 替代写法：将内嵌操作符用引号括起来
qk where 'latency>1000' app.log
qk where 'status>=500' access.log
qk where 'score<80' users.csv
```

### Regex 匹配（始终用引号防止 shell glob 展开）

```bash
# 注意：* 在 zsh/bash 中是 glob 字符 — regex 模式始终要加引号
qk where 'msg~=.*timeout.*' app.log
qk where 'msg~=.*panic.*' app.log
qk where 'reason~=.*failed.*' k8s.log
qk where 'path~=/api/.*' access.log
qk where 'name~=.*admin.*' users.csv
```

### 子字符串匹配（contains）

```bash
qk where msg contains timeout app.log
qk where msg contains panic app.log
qk where reason contains failed k8s.log
qk where path contains /api/ access.log
qk where name contains ar users.csv
qk where line contains error notes.txt
```

### 前缀匹配（startswith）

```bash
qk where msg startswith connection app.log
qk where msg startswith queue app.log
qk where path startswith /api/ access.log
qk where path startswith /health access.log
qk where name startswith Al users.csv
qk where line startswith 2024 notes.txt
qk where line startswith ERROR notes.txt
```

### 后缀匹配（endswith）

```bash
qk where path endswith users access.log
qk where path endswith orders access.log
qk where msg endswith timeout app.log
qk where msg endswith pointer app.log
qk where name endswith son users.csv
qk where line endswith ok notes.txt
```

### Shell 风格通配符（glob — 始终用引号防止 shell 展开）

```bash
# 注意：* 和 ? 是 shell 元字符 — glob 模式始终要加引号
# glob 默认不区分大小写
qk where msg glob '*timeout*' app.log
qk where msg glob '*panic*' app.log
qk where path glob '/api/*' access.log
qk where name glob 'Al*' users.csv     # 匹配 Alice, Alex, Alfred...
qk where name glob '*son' users.csv    # 匹配 Jackson, Wilson...
qk where name glob 'A*n' users.csv    # 以 A 开头，以 n 结尾
qk where line glob '*ERROR*' notes.txt
qk where line glob '*warn*' notes.txt  # 不区分大小写：匹配 WARN, Warn, warn
```

### 字段存在性检查

```bash
qk where request exists app.log
qk where response.error exists app.log
qk where metrics exists app.log
qk where user exists app.log
qk where probe exists k8s.log
```

### 多条件 — 逗号写法（可读性强的 AND）

```bash
# 逗号是 'and' 的别名
qk where level=error, service=api app.log
qk where level=error, latency gt 1000 app.log
qk where level=error, service=api, latency gt 1000 app.log
qk where status gte 500, method=GET access.log
qk where severity=error, region=us-east events.tsv
qk where role=admin, active=true users.csv
```

### 多条件 — 显式 and/or

```bash
qk where level=error and service=api app.log
qk where level=error or level=warn app.log
qk where status gte 500 and method=GET access.log
qk where level=error and service=db and latency gt 3000 app.log
```

***

## 嵌套 JSON — 两层

```bash
# app.log 包含：context.region, context.env, request.method, request.path, response.status
qk where context.region=us-east app.log
qk where context.env=prod app.log
qk where response.status=504 app.log
qk where response.status gte 500 app.log
qk where request.method=POST app.log
qk where request.path contains /api/ app.log

# access.log 包含：client.ip, client.country, server.host, server.region
qk where client.country=US access.log
qk where server.region=us-east access.log
qk where client.country!=US access.log
qk where server.host=web-01 access.log

# services.yaml 包含：resources.cpu, healthcheck.path
qk where status=running services.yaml
qk where enabled=true services.yaml

# data.json 包含：address.country, address.zip
qk where address.country=US data.json
qk where city=New\ York data.json
```

### 嵌套字段的多条件查询

```bash
qk where response.status gte 500, service=api app.log
qk where client.country=US, status gte 500 access.log
qk where context.env=prod, level=error app.log
```

***

## 嵌套 JSON — 三层

```bash
# k8s.log 包含：pod.labels.app, pod.labels.team, pod.labels.version
qk where pod.labels.app=api k8s.log
qk where pod.labels.team=platform k8s.log
qk where pod.labels.team=infra k8s.log
qk where pod.namespace=production k8s.log
qk where container.restart_count gt 0 k8s.log

# app.log 包含：request.headers.x-trace（三层）
qk where request.headers.x-trace exists app.log

# 三层嵌套与其他条件组合
qk where pod.labels.app=api, level=error k8s.log
qk where pod.labels.team=infra, level=warn k8s.log
qk where container.restart_count gte 3, pod.namespace=production k8s.log
```

***

## 字段投影（select）

```bash
# 最后一个过滤条件后的逗号可省略，两种写法均可：
qk where level=error select ts service msg app.log
qk where level=error, select ts service msg app.log   # 带尾逗号写法

# 更多示例
qk where level=error, select ts msg latency app.log
qk where status gte 500, select ts method path status access.log
qk where pod.labels.app=api, select ts msg reason k8s.log
qk where role=admin, select name city score users.csv
qk where severity=error, select ts event region duration_ms events.tsv
qk select name role city users.csv
qk select ts event severity duration_ms events.tsv
qk select ts level service msg latency app.log
```

***

## 计数与聚合

### 计数（count）

```bash
qk count app.log
qk where level=error, count app.log
qk count by level app.log
qk count by service app.log
qk count by method access.log
qk count by status access.log
qk count by severity events.tsv
qk count by event events.tsv
qk count by role users.csv
qk count by city users.csv
qk count by level k8s.log
qk count by pod.labels.team k8s.log
qk count by pod.labels.app k8s.log
qk where level=error, count by service app.log
qk where level=error, service=api, count by host app.log
qk where status gte 500, count by method access.log
qk where severity=error, count by event events.tsv
```

### 多字段分组统计

同时按多个字段分组，等价于 SQL 的 `GROUP BY a, b`。字段可以用空格或逗号分隔。

```bash
# 按 level + service 组合统计
qk count by level service app.log
qk count by level, service app.log   # 逗号语法也支持

# 先过滤再多字段分组
qk where env=prod, count by level service app.log

# DSL 等价写法
qk '| group_by(.level, .service)' app.log
```

输出（最多的组合排在最前）：
```json
{"level":"error","service":"api","count":42}
{"level":"error","service":"db","count":11}
{"level":"warn","service":"api","count":7}
```

### 去重计数（count unique）

统计某字段在所有（过滤后）记录中有多少个不同的值。

```bash
# 有多少个不同的服务？
qk count unique service app.log

# 触发 5xx 错误的 IP 有多少个？
qk where status gte 500, count unique ip access.log

# 生产环境中不同的事件类型数
qk where env=prod, count unique event events.tsv

# DSL 等价写法
qk '| count_unique(.service)' app.log
qk '.status >= 500 | count_unique(.ip)' access.log
```

输出：
```json
{"count_unique":7}
```

### 按时间分桶统计

使用时间后缀（`s` 秒、`m` 分钟、`h` 小时、`d` 天）将事件分组到固定时间窗口。
时间戳字段默认为 `ts`，可用显式字段名覆盖。

```bash
# 按 5 分钟分桶（自动读取 ts 字段）
qk count by 5m app.log

# 按 1 小时分桶
qk count by 1h app.log

# 按 1 天分桶
qk count by 1d app.log

# 指定时间戳字段名
qk count by 1h timestamp app.log

# 先过滤再分桶
qk where level=error, count by 5m app.log

# DSL 等价写法
qk '| group_by_time(.ts, "5m")' app.log
qk '| group_by_time(.timestamp, "1h")' app.log
```

输出格式（每个桶一条记录）：

```json
{"bucket":"2024-01-15T10:00:00Z","count":42}
{"bucket":"2024-01-15T10:05:00Z","count":17}
```

时间戳字段支持三种格式：

- RFC 3339 字符串：`"2024-01-15T10:05:30Z"` 或 `"2024-01-15T10:05:30+08:00"`
- Unix epoch 秒（整数 ≥ 1 000 000 000）
- Unix epoch 毫秒（整数 ≥ 1 000 000 000 000）

缺少或无法解析时间戳的记录会被静默跳过。

### 按日历单位分桶统计

使用日历对齐的桶（`hour`、`day`、`week`、`month`、`year`）将事件分组。
与固定时长桶（`5m`、`1h`）不同，这些桶对齐到 UTC 整点/午夜/月初等边界。

```bash
# 按自然日统计（对齐 UTC 零点）
qk count by day ts app.log

# 按日历月统计
qk count by month ts app.log

# 按日历年统计
qk count by year ts app.log

# 按整点小时统计
qk count by hour ts app.log

# 按 ISO 周统计（周一对齐）
qk count by week ts app.log

# 先过滤再分桶
qk where level=error, count by day ts app.log

# DSL 等价写法
qk '| group_by_time(.ts, "day")' app.log
qk '| group_by_time(.ts, "month")' app.log
```

输出格式：
```json
{"bucket":"2024-01-15","count":1234}
{"bucket":"2024-01-16","count":987}
```

| 单位    | 语法                    | 对齐方式            | 示例桶值             |
|---------|------------------------|---------------------|----------------------|
| `hour`  | `count by hour ts`     | UTC 整点            | `2024-01-15T10:00Z`  |
| `day`   | `count by day ts`      | UTC 零点            | `2024-01-15`         |
| `week`  | `count by week ts`     | ISO 周一 00:00Z     | `2024-W03`           |
| `month` | `count by month ts`    | 当月 1 日 00:00Z    | `2024-01`            |
| `year`  | `count by year ts`     | 1 月 1 日 00:00Z    | `2024`               |

### DSL 时间属性提取

从时间戳字段提取时间分量，作为新字段追加到每条记录，便于后续过滤或分组：

```bash
# 添加 hour_of_day 字段（0–23）
qk '| hour_of_day(.ts)' app.log

# 添加 day_of_week 字段（"Monday"…"Sunday"）
qk '| day_of_week(.ts)' app.log

# 添加 is_weekend 字段（true/false）
qk '| is_weekend(.ts)' app.log

# 组合：按星期统计错误分布
qk '.level == "error" | day_of_week(.ts) | group_by(.day_of_week)' app.log

# 找出高峰小时
qk '| hour_of_day(.ts) | group_by(.hour_of_day)' app.log

# 仅统计周末流量
qk '| is_weekend(.ts) | .is_weekend == true | count()' app.log
```

`| hour_of_day(.ts)` 的输出示例：
```json
{"ts":"2024-01-15T10:32:00Z","level":"info","msg":"ok","hour_of_day":10}
```

### DSL 字符串与数组函数

对字段原地修改，或从字符串/数组中派生新的数值字段。

```bash
# 转小写后再分组（忽略大小写差异）
qk '| to_lower(.level) | group_by(.level)' app.log

# 转大写
qk '| to_upper(.method)' access.log

# 替换子字符串
qk '| replace(.msg, "localhost", "prod-host")' app.log

# 将逗号分隔的字符串字段拆分为 JSON 数组
qk '| split(.tags, ",")' app.log

# 用 map 获取字符串或数组的长度
qk '| map(.msg_len = length(.msg))' app.log
qk '| map(.tag_count = length(.tags))' app.log  # 数组也支持

# 数组成员检查（contains 同时支持字符串子串和数组元素）
qk '.tags contains "prod"' app.log
```

字符串函数速查：

| 阶段 | 语法 | 效果 |
|---|---|---|
| `to_lower` | `to_lower(.field)` | 转小写，原地修改 |
| `to_upper` | `to_upper(.field)` | 转大写，原地修改 |
| `replace` | `replace(.field, "old", "new")` | 替换所有匹配，原地修改 |
| `split` | `split(.field, ",")` | 拆分为 JSON 数组，原地修改 |
| `length` | `map(.n = length(.field))` | 字符数（字符串）或元素数（数组） |

### DSL 算术运算 — `map` 阶段

计算一个新字段，值来自算术表达式。支持 `+`、`-`、`*`、`/`，运算符优先级标准（先乘除后加减），支持括号。

字段引用使用点号表示法（`.field`）。若字段缺失或不是数字，该条记录的输出字段静默跳过。

```bash
# 毫秒转秒
qk '| map(.latency_s = .latency / 1000.0)' app.log

# 字节转兆字节
qk '| map(.mb = .bytes / 1048576.0)' app.log

# 两个字段相加
qk '| map(.total = .req_bytes + .resp_bytes)' access.log

# 带括号的复杂表达式
qk '| map(.normalized = (.score - .min) / (.max - .min))' scores.ndjson

# 组合：计算 → 过滤 → 聚合
qk '| map(.latency_s = .latency / 1000.0) | .latency_s > 5.0 | avg(.latency_s)' app.log
```

`| map(.latency_s = .latency / 1000.0)` 的输出示例：
```json
{"ts":"2024-01-15T10:00:00Z","level":"info","latency":2340,"latency_s":2.34}
```

### 求和 / 均值 / 最小值 / 最大值

```bash
# 求和（sum）
qk sum latency app.log
qk where level=error, sum latency app.log
qk where service=api, sum latency app.log
qk sum duration_ms events.tsv
qk sum salary users.csv

# 均值（avg）
qk avg latency app.log
qk where level=error, avg latency app.log
qk where service=db, avg latency app.log
qk avg score users.csv
qk where severity=error, avg duration_ms events.tsv

# 最小值 / 最大值（min / max）
qk min latency app.log
qk max latency app.log
qk where service=api, min latency app.log
qk where service=api, max latency app.log
qk min score users.csv
qk max score users.csv
qk where department=Engineering, max salary users.csv
qk min status access.log
qk max status access.log
```

***

## 排序与限制

```bash
# 排序（sort）
qk sort latency desc app.log
qk sort latency asc app.log
qk sort ts desc app.log
qk sort score desc users.csv
qk sort age asc users.csv
qk sort duration_ms desc events.tsv
qk where level=error, sort latency desc app.log
qk where service=api, sort latency desc app.log
qk where severity=error, sort duration_ms desc events.tsv

# 限制 / 头部（limit / head 互为别名）
qk limit 5 app.log
qk head 5 app.log
qk sort latency desc limit 3 app.log
qk sort latency desc head 5 access.log
qk where level=error, sort latency desc limit 1 app.log
qk where level=error, sort latency desc limit 5 app.log
qk where status gte 500, sort latency desc limit 3 access.log
qk where score gt 80, sort score desc limit 5 users.csv

# 跳过（skip，仅 DSL 模式 — 用于分页）
qk '| skip(5) | limit(5)' app.log    # 第 6 至 10 条记录
```

***

## DSL 表达式层

DSL 模式在第一个参数以 `.`、`not `  或 `|` 开头时自动激活。

### 过滤表达式

```bash
# 等值匹配
qk '.level == "error"' app.log
qk '.service == "api"' app.log
qk '.method == "POST"' access.log
qk '.role == "admin"' users.csv

# 不等于
qk '.level != "info"' app.log

# 数值比较（DSL 模式：对整个表达式加引号，不需要对操作符加引号）
qk '.latency > 1000' app.log
qk '.latency < 100' app.log
qk '.status >= 500' access.log
qk '.score > 90' users.csv
qk '.age <= 30' users.csv

# 嵌套字段访问
qk '.response.status >= 500' app.log
qk '.client.country == "US"' access.log
qk '.pod.labels.app == "api"' k8s.log
qk '.pod.labels.team == "infra"' k8s.log
qk '.address.country == "US"' data.json

# 子字符串匹配（字符串）和数组成员检查（数组）
qk '.msg contains "timeout"' app.log
qk '.msg matches ".*panic.*"' app.log
qk '.reason contains "failed"' k8s.log
qk '.tags contains "prod"' app.log        # 同时支持 JSON 数组元素检查

# 字段存在性检查
qk '.request exists' app.log
qk '.probe exists' k8s.log

# 布尔逻辑
qk '.level == "error" and .latency > 1000' app.log
qk '.level == "error" or .level == "warn"' app.log
qk 'not .level == "info"' app.log
qk '.status >= 500 and .method == "GET"' access.log
qk '.pod.labels.app == "api" and .level == "error"' k8s.log
```

### 管道阶段（Pipeline Stages）

```bash
# pick — 只保留指定字段
qk '.level == "error" | pick(.ts, .service, .msg)' app.log
qk '.status >= 500 | pick(.ts, .method, .path, .status)' access.log
qk '| pick(.name, .role, .score)' users.csv

# omit — 删除指定字段
qk '.level == "error" | omit(.host, .context)' app.log
qk '| omit(.address)' data.json

# count — 计数
qk '.level == "error" | count()' app.log
qk '| count()' users.csv

# sort_by — 排序
qk '| sort_by(.latency desc)' app.log
qk '| sort_by(.score desc)' users.csv
qk '| sort_by(.age asc)' users.csv

# group_by — 单字段分组
qk '| group_by(.level)' app.log
qk '| group_by(.service)' app.log
qk '| group_by(.method)' access.log
qk '| group_by(.pod.labels.team)' k8s.log
qk '| group_by(.role)' users.csv

# group_by — 多字段分组
qk '| group_by(.level, .service)' app.log
qk '| group_by(.method, .status)' access.log

# limit 和 skip
qk '| limit(5)' app.log
qk '| skip(10) | limit(5)' app.log   # 分页：第 3 页（每页 5 条）

# dedup — 保留每个唯一值的第一次出现
qk '| dedup(.service)' app.log
qk '| dedup(.role)' users.csv
qk '| dedup(.event)' events.tsv

# sum / avg / min / max
qk '| sum(.latency)' app.log
qk '.level == "error" | sum(.latency)' app.log
qk '| avg(.latency)' app.log
qk '| min(.latency)' app.log
qk '| max(.latency)' app.log
qk '| avg(.score)' users.csv
qk '| max(.score)' users.csv

# count_unique — 去重计数
qk '| count_unique(.service)' app.log
qk '.level == "error" | count_unique(.msg)' app.log
qk '.status >= 500 | count_unique(.ip)' access.log

# group_by_time — 时间分桶（固定时长和日历单位）
qk '| group_by_time(.ts, "5m")' app.log
qk '| group_by_time(.ts, "1h")' app.log
qk '| group_by_time(.ts, "day")' app.log
qk '| group_by_time(.ts, "month")' app.log
qk '| group_by_time(.ts, "week")' app.log

# hour_of_day / day_of_week / is_weekend — 时间属性提取
qk '| hour_of_day(.ts)' app.log
qk '| day_of_week(.ts)' app.log
qk '| is_weekend(.ts)' app.log
qk '.level == "error" | hour_of_day(.ts) | group_by(.hour_of_day)' app.log
qk '| day_of_week(.ts) | group_by(.day_of_week)' app.log

# to_lower / to_upper — 大小写转换（原地）
qk '| to_lower(.level)' app.log
qk '| to_upper(.method)' access.log
qk '| to_lower(.level) | group_by(.level)' app.log

# replace — 字符串替换（原地）
qk '| replace(.msg, "localhost", "prod-1")' app.log
qk '| replace(.env, "production", "prod")' app.log

# split — 字符串拆分为 JSON 数组（原地）
qk '| split(.tags, ",")' app.log
qk '| split(.tags, ",") | .tags contains "prod"' app.log

# map — 算术表达式（+、-、*、/、length）
qk '| map(.latency_s = .latency / 1000.0)' app.log
qk '| map(.mb = .bytes / 1048576.0)' access.log
qk '| map(.total = .req_bytes + .resp_bytes)' access.log
qk '| map(.msg_len = length(.msg))' app.log
qk '| map(.tag_count = length(.tags))' app.log
qk '| map(.latency_s = .latency / 1000.0) | .latency_s > 5.0 | avg(.latency_s)' app.log
```

### 链式管道

```bash
qk '.level == "error" | pick(.ts, .service, .msg) | sort_by(.ts desc)' app.log
qk '.response.status >= 500 | pick(.ts, .service, .response.status) | limit(5)' app.log
qk '.status >= 500 | pick(.method, .path, .status) | group_by(.method)' access.log
qk '.pod.labels.team == "platform" | pick(.ts, .msg, .level) | sort_by(.ts asc)' k8s.log
```

### 纯管道（不带过滤条件）

```bash
qk '| group_by(.level)' app.log
qk '| sort_by(.latency desc)' app.log
qk '| sort_by(.score desc) | limit(5)' users.csv
qk '| group_by(.pod.labels.team)' k8s.log
qk '| group_by(.country)' access.log
```

***

## 按格式分类的命令

### NDJSON（app.log, access.log, k8s.log）— 默认格式

```bash
qk where level=error app.log
qk where level=error, service=api app.log
qk where level=error, service=api, latency gt 1000 app.log
qk where level=error, select ts service msg app.log
qk where level=error, select ts service msg latency app.log
qk where level=error, count by service app.log
qk where level=error, sort latency desc limit 5 app.log
qk where level=error, avg latency app.log
qk where response.status gte 500 app.log
qk where response.status gte 500, service=api app.log
qk '.level == "error" | pick(.ts, .service, .msg, .latency)' app.log
qk count by service app.log
qk avg latency app.log
```

### JSON 数组（data.json）

```bash
# 自动从 [ 前缀检测 — 每个数组元素成为一条记录
qk data.json
qk where role=admin data.json
qk where city=New\ York data.json
qk where active=true data.json
qk where score gt 80 data.json
qk where address.country=US data.json
qk where role=admin, active=true data.json
qk where role=admin, score gt 90 data.json
qk where role=admin, select name city score data.json
qk where score gt 80, sort score desc data.json
qk where active=true, count by role data.json
qk where active=true, avg score data.json
qk count by role data.json
qk count by city data.json
qk sort score desc limit 3 data.json
qk avg score data.json
qk max score data.json
```

### YAML 多文档（services.yaml）

```bash
# 每个 --- 文档成为一条记录
qk services.yaml
qk where status=running services.yaml
qk where enabled=true services.yaml
qk where status=degraded services.yaml
qk where env=production services.yaml
qk where status=running, enabled=true services.yaml
qk where env=production, status=running services.yaml
qk where enabled=true, select name port replicas services.yaml
qk where status=running, count by env services.yaml
qk count by status services.yaml
qk select name status replicas services.yaml
```

### TOML（config.toml）

```bash
# 整个文件 = 一条记录；用点号访问嵌套节
qk config.toml
qk select server.port server.workers database.pool_max config.toml
qk '.server.port > 8000' config.toml
qk '.logging.level == "info"' config.toml
qk '.feature_flags.enable_new_dashboard == true' config.toml
```

### CSV（users.csv）

```bash
# 首行作为字段名；数值自动转换（30 → Number，而非 String）
qk users.csv
qk where role=admin users.csv
qk where city=New\ York users.csv
qk where active=true users.csv
qk where department=Engineering users.csv
qk where score gt 80 users.csv
qk where age lt 30 users.csv
qk where name startswith Al users.csv
qk where name endswith son users.csv
qk where name glob 'Al*' users.csv
qk where role=admin, department=Engineering users.csv
qk where active=true, score gt 80 users.csv
qk where active=true, age lt 30 users.csv

# 无标题行的 CSV — 使用 --no-header；列名自动命名为 col1, col2, col3...
# --no-header 必须放在查询表达式之前（clap trailing_var_arg 语义）
qk --no-header users_no_header.csv
qk --no-header head 5 users_no_header.csv
qk --no-header where col3=Engineering users_no_header.csv
qk --no-header count by col4 users_no_header.csv
qk --no-header sort col6 desc limit 5 users_no_header.csv
qk where role=admin, select name city score salary users.csv
qk where department=Engineering, sort salary desc users.csv
qk where active=true, count by role users.csv
qk where active=true, count by department users.csv
qk where department=Engineering, avg salary users.csv
qk where role=admin, max salary users.csv
qk count by role users.csv
qk count by city users.csv
qk count by department users.csv
qk sort score desc users.csv
qk sort salary desc limit 5 users.csv
qk avg score users.csv
qk max salary users.csv
qk sum salary users.csv
```

### TSV（events.tsv）

```bash
# 制表符分隔；从 .tsv 扩展名自动检测
qk events.tsv
qk where severity=error events.tsv
qk where event=login events.tsv
qk where region=us-east events.tsv
qk where duration_ms gt 1000 events.tsv
qk where severity=error, region=us-east events.tsv
qk where event=login, region=us-east events.tsv
qk where severity=error, select ts event service region events.tsv
qk where severity=error, count by event events.tsv
qk where severity=error, sort duration_ms desc limit 3 events.tsv
qk where severity=error, avg duration_ms events.tsv
qk count by event events.tsv
qk count by severity events.tsv
qk count by region events.tsv
qk sort duration_ms desc limit 5 events.tsv
qk avg duration_ms events.tsv
qk max duration_ms events.tsv
```

### logfmt（services.logfmt）

```bash
# key=value 格式；常见于 Go 服务（Logrus、Zap）
qk services.logfmt
qk where level=error services.logfmt
qk where service=api services.logfmt
qk where latency gt 1000 services.logfmt
qk where level=error, service=db services.logfmt
qk where level=error, service=api services.logfmt
qk where level=error, latency gt 1000 services.logfmt
qk where level=error, select ts service msg services.logfmt
qk where level=error, count by service services.logfmt
qk where level=error, sort latency desc services.logfmt
qk where level=error, avg latency services.logfmt
qk where service=api, sort latency desc limit 3 services.logfmt
qk count by level services.logfmt
qk count by service services.logfmt
qk avg latency services.logfmt
qk max latency services.logfmt
qk sort latency desc limit 5 services.logfmt
```

### Gzip（app.log.gz）

```bash
# 透明解压 — 无需手动 gunzip
qk app.log.gz
qk count app.log.gz
qk where level=error app.log.gz
qk where level=error, service=api app.log.gz
qk where level=error, select ts service msg app.log.gz
qk where level=error, count by service app.log.gz
qk where latency gt 1000 app.log.gz
qk count by service app.log.gz
qk avg latency app.log.gz

# 对压缩与未压缩文件执行相同查询 — 结果必须一致
qk count by level app.log
qk count by level app.log.gz
```

### 纯文本（notes.txt）

```bash
# 每行 → {"line": "..."} — 使用 'line' 作为字段名
qk notes.txt
qk head 5 notes.txt
qk count notes.txt

# 精确子字符串匹配
qk where line contains error notes.txt
qk where line contains timeout notes.txt
qk where line contains WARN notes.txt

# 前缀 / 后缀匹配
qk where line startswith 2024 notes.txt
qk where line startswith ERROR notes.txt
qk where line endswith ok notes.txt
qk where line endswith done notes.txt

# Shell 风格通配符（不区分大小写，始终加引号）
qk where line glob '*ERROR*' notes.txt
qk where line glob '*warn*' notes.txt     # 不区分大小写：匹配 WARN, Warn, warn
qk where line glob '*timeout*' notes.txt
qk where line glob '2024*ERROR*' notes.txt  # 以 2024 开头且包含 ERROR

# Regex（始终加引号防止 shell glob 展开）
qk where 'line~=.*error.*' notes.txt
qk where 'line~=.*\[ERROR\].*' notes.txt
qk where 'line~=(WARN|ERROR)' notes.txt

# 结合 grep 处理 qk 无法表达的文本模式
qk notes.txt | grep -i error
```

***

## 输出格式

```bash
# --fmt 必须放在查询表达式之前
qk --fmt ndjson where level=error app.log    # NDJSON（默认）
qk --fmt pretty where level=error app.log    # 带空行的缩进 JSON
qk --fmt table where level=error app.log     # 对齐表格（类似 psql）
qk --fmt csv where level=error app.log       # CSV（可在 Excel 中打开）
qk --fmt raw where level=error app.log       # 保持原始行不变

# 美化输出所有字段
qk --fmt pretty data.json
qk --fmt pretty services.yaml
qk --fmt pretty config.toml

# 表格输出便于对比
qk --fmt table count by level app.log
qk --fmt table count by service app.log
qk --fmt table sort score desc users.csv
qk --fmt table where level=error select ts service msg latency app.log

# CSV 输出用于 Excel / Google Sheets
qk --fmt csv users.csv                      # 重新导出过滤后的 CSV
qk --fmt csv where level=error app.log      # 将错误导出为 CSV
qk --fmt csv sort salary desc users.csv
```

***

## 颜色控制

```bash
qk --color where level=error app.log         # 强制开启 ANSI 颜色
qk --no-color where level=error app.log      # 强制关闭颜色（适用于管道）

# 在终端中自动启用颜色，管道时自动禁用
qk where level=error app.log | cat           # 管道 — 无颜色
qk where level=error app.log | qk count by service  # 管道 — 无颜色
```

***

## 多文件查询

```bash
# 同时查询多个文件（并行处理）
qk where level=error app.log access.log k8s.log
qk count by level app.log k8s.log services.logfmt
qk where level=error count by service app.log k8s.log

# Glob 模式（如需防止 shell 展开则加引号）
qk where level=error *.log
qk count *.log
```

***

## qk + jq：处理 JSON 编码的字符串字段

`encoded.log` 的字段**值本身是 JSON 字符串** — 这是某些日志管道中的常见模式。

```bash
# 先查看原始数据
qk encoded.log

# 解码一个字段，再用 qk 过滤
cat encoded.log | jq -c '.payload = (.payload | fromjson)' | qk where payload.level=error

# 解码两个字段，基于解码内容过滤
cat encoded.log | jq -c '{service, ts, payload: (.payload | fromjson), meta: (.metadata | fromjson)}' \
  | qk where meta.env=prod, payload.level=error

# qk 预过滤 → jq 解码 → qk 聚合
cat encoded.log | qk where service=api \
  | jq -c '.payload = (.payload | fromjson)' \
  | qk count by payload.level

# 从解码后的 payload 中提取单个字段
cat encoded.log | qk where service=api | jq -r '.payload | fromjson | .msg'

# 完整管道：qk 过滤 → jq 解码 → qk 按解码字段计数
cat encoded.log | jq -c '.payload = (.payload | fromjson)' | qk count by payload.level
```

***

## 管道组合

```bash
# 两个 qk 命令串联
qk where level=error app.log | qk count by service
qk sort latency desc app.log | qk limit 5

# 三个阶段
qk where level=error app.log | qk sort latency desc | qk limit 1

# 与 jq 结合
qk where level=error app.log | jq '.latency'
qk where level=error app.log | jq '{service: .service, ms: .latency}'
qk where level=error app.log | jq -s 'map(.latency) | add'

# 与 grep 结合（处理 qk 无法表达的文本模式）
qk where service=api app.log | grep timeout

# 与 sort 和 uniq 结合（处理 qk 未知的字段值统计）
qk where level=error app.log | jq -r '.service' | sort | uniq -c | sort -rn

# 处理日志文件的最后 1000 行
tail -n 1000 /path/to/app.log | qk where level=error

# 注意：目前不支持 tail -f。qk 需要读到 EOF 才开始处理，
# `tail -f file | qk ...` 会无限阻塞。请使用 tail -n 代替。
```

***

## 大文件性能测试

这些测试内置于 qk 测试套件，按需运行。大文件在**测试运行时由代码动态生成**，无需预先存储任何 fixture 文件。文件写入系统临时目录，测试结束后自动删除。

### 运行大文件测试

```bash
# 先构建 release 版本（比 debug 快 10-20 倍）
cargo build --release

# 运行全部 8 个大文件测试，打印指标
cargo test --test large_file --release -- --ignored --nocapture

# 单独运行某一个测试
cargo test --test large_file --release large_file_streaming_filter_2gb -- --ignored --nocapture
```

### 各测试覆盖内容

| 测试名 | 生成文件大小 | 操作 | 核心断言 |
|--------|------------|------|---------|
| `large_file_streaming_filter_2gb` | ~2 GB stdin | `where level=error` | 结果 = 25% 记录，耗时 < 120 s |
| `large_file_streaming_latency_filter_2gb` | ~2 GB stdin | `where latency gt 500` | 结果 ≈ 50.4% 记录 |
| `large_file_count_by_200mb` | ~200 MB 文件 | `count by level` | 4 组，各 25% |
| `large_file_count_total_200mb` | ~200 MB 文件 | `count` | 精确总数 |
| `large_file_sum_latency_200mb` | ~200 MB 文件 | `sum latency` | 公式精确匹配 |
| `large_file_avg_latency_200mb` | ~200 MB 文件 | `avg latency` | 与 504.5 误差 < 0.5 |
| `large_file_corrupt_lines_resilience_50mb` | ~50 MB + 200 条损坏行 | `count` | 只统计合法记录，stderr 有警告 |
| `large_file_avg_null_field_50mb` | ~50 MB | `avg nonexistent_field` | `{"avg":null}`，stderr 有警告 |

### 流式 vs 批量 — 内存模型

| 操作 | 内存模型 | 2 GB 是否安全？ | 说明 |
|------|---------|--------------|------|
| `where FIELD=VALUE`（stdin） | O(1) — 流式 | ✅ 安全 | 通过 stdin 管道触发流式路径 |
| `where FIELD=VALUE`（文件路径） | O(n) — 批量 | ⚠️ 有风险 | 文件路径始终走批量模式，约 500 字节/记录 |
| `count by FIELD` | O(n) — 批量 | ⚠️ 有风险 | 需要全部记录才能分组 |
| `sum/avg/min/max FIELD` | O(n) — 批量 | ⚠️ 有风险 | 需要全部记录才能聚合 |
| `sort FIELD` | O(n) — 批量 | ⚠️ 有风险 | 需要完整排序缓冲区 |
| `count`（stdin） | O(n) — 批量 | ⚠️ 有风险 | 聚合操作即使走 stdin 也会强制缓冲 |

**经验法则：** 对于大于 500 MB 的文件，纯过滤查询应使用 stdin 管道：

```bash
# O(1) 内存 — 通过 stdin 走流式路径
cat /path/to/huge.log | qk where level=error

# 也是流式 — 结果直接传给下一个工具
cat /path/to/huge.log | qk where level=error | qk select ts service msg

# --fmt raw 原样透传原始行，无重新序列化开销
cat /path/to/huge.log | qk --fmt raw where level=error > errors.log
```

### 新增算子（流式模式下同样安全）

```bash
# 范围过滤 — 包含 LOW 和 HIGH 端点
cat app.log | qk where latency between 100 500

# 相对时间过滤 — now 在查询时动态解析
cat app.log | qk where ts gt now-5m
cat app.log | qk where ts gt now-1h
cat app.log | qk where ts between now-1h now
```

***

## 交互式 TUI（--ui）

`--ui` 打开实时终端界面，每次击键自动重新执行查询。

```bash
qk --ui app.log
qk --ui app.log access.log
cat app.log | qk --ui
```

| 按键 | 操作 |
|---|---|
| 输入字符 | 编辑查询（自动执行）|
| `←` `→` | 移动光标 |
| `↑` `↓` / `PgUp` `PgDn` | 滚动结果 |
| `Esc` / `Ctrl+C` | 退出 |

任何有效的快速层或 DSL 查询均可在 TUI 中使用：`where level=error`、`count by service`、`| group_by(.level, .service)`。

***

## 语法速查

```
qk [--fmt FORMAT] [--color|--no-color] [--no-header] [--explain] QUERY [FILES...]

快速层（Fast layer）：
  where FIELD=VALUE              精确匹配
  where FIELD!=VALUE             不等于
  where FIELD gt/lt/gte/lte N   数值比较（对 shell 友好）
  where FIELD contains TEXT      子字符串匹配
  where FIELD startswith PREFIX  前缀匹配
  where FIELD endswith SUFFIX    后缀匹配
  where 'FIELD glob PATTERN'     shell 通配符（* ? — 始终加引号！）
  where 'FIELD~=PATTERN'         regex（始终加引号！）
  where FIELD exists             字段存在性检查
  where A=1, B=2                 逗号 = and
  select F1 F2 ...               字段投影
  count / count by FIELD [FIELD2…]  计数（支持多字段）
  count unique FIELD             字段去重计数
  count by 5m|1h|1d FIELD        固定时长时间桶
  count by day|week|month|year FIELD  日历对齐时间桶
  where FIELD between LOW HIGH   包含端点的范围过滤
  where FIELD gt now-5m          相对时间过滤（now±Ns/m/h/d）
  fields                         查看所有字段名
  sum/avg/min/max FIELD          统计聚合
  sort FIELD asc|desc            排序
  limit N / head N               取前 N 条

标志（Flags）：
  --no-header                    将 CSV/TSV 首行视为数据而非标题行
                                 列名自动命名为 col1, col2, col3 ...
  --cast FIELD=TYPE              在查询执行前将字段转换为指定类型
                                 支持类型：number(num/float/int) string(str/text) bool(boolean) null(none) auto
                                 可重复使用：--cast f1=number --cast f2=string

DSL 层（第一个参数以 . not | 开头时激活）：
  '.field == "val" | pick(.a, .b) | sort_by(.f desc) | limit(N)'
  阶段：pick omit count() sort_by() group_by() limit() skip() dedup() sum() avg() min() max()
          group_by_time(.field, "5m"|"1h"|"day"|"month"|…)
          hour_of_day(.field)  day_of_week(.field)  is_weekend(.field)
          count_unique(.field)
          group_by(.f1, .f2)  — 多字段分组
          to_lower(.field)  to_upper(.field)
          replace(.field, "old", "new")  split(.field, ",")
          map(.out = 算术表达式)  — 运算符：+ - * /，length(.field)
```

