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
13. [输出格式（--fmt）](#输出格式---fmt)
14. [颜色输出（--color）](#颜色输出---color)
15. [多种文件格式](#多种文件格式)
16. [管道组合](#管道组合)
17. [常见问题](#常见问题)
18. [完整速查表](#完整速查表)

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
git clone https://github.com/YOUR_USERNAME/qk.git
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

后续所有例子都基于以下文件，先创建它们：

```bash
cat > app.log << 'EOF'
{"ts":"2024-01-01T10:00:00Z","level":"info","service":"api","msg":"server started","latency":0}
{"ts":"2024-01-01T10:01:00Z","level":"error","service":"api","msg":"connection timeout","latency":3001}
{"ts":"2024-01-01T10:02:00Z","level":"warn","service":"worker","msg":"queue depth high","latency":150}
{"ts":"2024-01-01T10:03:00Z","level":"info","service":"api","msg":"request ok","latency":42}
{"ts":"2024-01-01T10:04:00Z","level":"error","service":"worker","msg":"panic: nil pointer","latency":0}
{"ts":"2024-01-01T10:05:00Z","level":"info","service":"web","msg":"page loaded","latency":88}
EOF
```

```bash
cat > access.log << 'EOF'
{"ts":"2024-01-01T10:00:00Z","method":"GET","path":"/api/users","status":200,"latency":42}
{"ts":"2024-01-01T10:01:00Z","method":"POST","path":"/api/login","status":401,"latency":15}
{"ts":"2024-01-01T10:02:00Z","method":"GET","path":"/api/orders","status":500,"latency":3200}
{"ts":"2024-01-01T10:03:00Z","method":"DELETE","path":"/api/cache","status":200,"latency":8}
{"ts":"2024-01-01T10:04:00Z","method":"GET","path":"/api/users","status":503,"latency":9800}
{"ts":"2024-01-01T10:05:00Z","method":"GET","path":"/health","status":200,"latency":1}
EOF
```

---

## 基础用法

### 显示所有记录

```bash
qk app.log
```

预期输出（在终端中会有颜色，error=红，warn=黄，info=绿）：

```
{"ts":"2024-01-01T10:00:00Z","level":"info","service":"api","msg":"server started","latency":0}
{"ts":"2024-01-01T10:01:00Z","level":"error","service":"api","msg":"connection timeout","latency":3001}
{"ts":"2024-01-01T10:02:00Z","level":"warn","service":"worker","msg":"queue depth high","latency":150}
{"ts":"2024-01-01T10:03:00Z","level":"info","service":"api","msg":"request ok","latency":42}
{"ts":"2024-01-01T10:04:00Z","level":"error","service":"worker","msg":"panic: nil pointer","latency":0}
{"ts":"2024-01-01T10:05:00Z","level":"info","service":"web","msg":"page loaded","latency":88}
```

### 从 stdin 读取

```bash
echo '{"level":"error","msg":"oops"}' | qk
```

预期输出：

```
{"level":"error","msg":"oops"}
```

### 查看解析方式（--explain）

```bash
qk --explain where level=error app.log
```

预期输出（显示检测到的格式和解析后的查询，然后退出）：

```
mode:    Keyword
format:  Ndjson (detected)
query:   FastQuery { filters: [level = error], ... }
files:   ["app.log"]
```

---

## 过滤（where）

### 等于（=）

```bash
qk where level=error app.log
```

预期输出（2 条 error 记录）：

```
{"ts":"2024-01-01T10:01:00Z","level":"error","service":"api","msg":"connection timeout","latency":3001}
{"ts":"2024-01-01T10:04:00Z","level":"error","service":"worker","msg":"panic: nil pointer","latency":0}
```

### 不等于（!=）

```bash
qk where level!=info app.log
```

预期输出（3 条，只排除 info）：

```
{"ts":"2024-01-01T10:01:00Z","level":"error",...}
{"ts":"2024-01-01T10:02:00Z","level":"warn",...}
{"ts":"2024-01-01T10:04:00Z","level":"error",...}
```

### 数值大于（>）

```bash
qk where latency>100 app.log
```

预期输出（latency 超过 100 的 2 条）：

```
{"ts":"2024-01-01T10:01:00Z","level":"error","service":"api","msg":"connection timeout","latency":3001}
{"ts":"2024-01-01T10:02:00Z","level":"warn","service":"worker","msg":"queue depth high","latency":150}
```

### 数值小于（<）

```bash
qk where latency<50 app.log
```

预期输出（latency < 50 的 3 条）：

```
{"ts":"2024-01-01T10:00:00Z","level":"info","service":"api","msg":"server started","latency":0}
{"ts":"2024-01-01T10:03:00Z","level":"info","service":"api","msg":"request ok","latency":42}
{"ts":"2024-01-01T10:04:00Z","level":"error","service":"worker","msg":"panic: nil pointer","latency":0}
```

### 大于等于（>=）

```bash
qk where status>=500 access.log
```

预期输出（HTTP 5xx 响应）：

```
{"ts":"2024-01-01T10:02:00Z","method":"GET","path":"/api/orders","status":500,"latency":3200}
{"ts":"2024-01-01T10:04:00Z","method":"GET","path":"/api/users","status":503,"latency":9800}
```

### 小于等于（<=）

```bash
qk where latency<=42 app.log
```

预期输出（latency ≤ 42 的 3 条）：

```
{"ts":"2024-01-01T10:00:00Z",...,"latency":0}
{"ts":"2024-01-01T10:03:00Z",...,"latency":42}
{"ts":"2024-01-01T10:04:00Z",...,"latency":0}
```

### 正则匹配（\~=）

```bash
qk where msg~=.*timeout.* app.log
```

预期输出（msg 匹配 timeout 正则的 1 条）：

```
{"ts":"2024-01-01T10:01:00Z","level":"error","service":"api","msg":"connection timeout","latency":3001}
```

### 包含子字符串（contains）

```bash
qk where msg contains queue app.log
```

预期输出：

```
{"ts":"2024-01-01T10:02:00Z","level":"warn","service":"worker","msg":"queue depth high","latency":150}
```

### 字段存在（exists）

```bash
# 找所有包含 error 字段的记录（注意：这是字段名，不是 level=error）
echo '{"level":"info","msg":"ok"}
{"level":"error","msg":"bad","error":"connection refused"}' | qk where error exists
```

预期输出（只有包含名为 "error" 的字段的那条）：

```
{"level":"error","msg":"bad","error":"connection refused"}
```

### AND 多条件

```bash
qk where level=error and service=api app.log
```

预期输出（level=error 且 service=api 的 1 条）：

```
{"ts":"2024-01-01T10:01:00Z","level":"error","service":"api","msg":"connection timeout","latency":3001}
```

### OR 多条件

```bash
qk where level=error or level=warn app.log
```

预期输出（3 条：2 个 error + 1 个 warn）：

```
{"ts":"2024-01-01T10:01:00Z","level":"error",...}
{"ts":"2024-01-01T10:02:00Z","level":"warn",...}
{"ts":"2024-01-01T10:04:00Z","level":"error",...}
```

### 嵌套字段访问（点号路径）

```bash
echo '{"response":{"status":503,"latency":1200},"service":"api"}
{"response":{"status":200,"latency":30},"service":"web"}' | qk where response.status=503
```

预期输出：

```
{"response":{"status":503,"latency":1200},"service":"api"}
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

### 嵌套字段

```bash
echo '{"request":{"method":"GET","path":"/api"},"response":{"status":500}}
{"request":{"method":"POST","path":"/login"},"response":{"status":200}}' | qk '.response.status >= 500'
```

预期输出：

```
{"request":{"method":"GET","path":"/api"},"response":{"status":500}}
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

`qk` 自动检测格式，无需指定参数。

### logfmt 格式

```bash
cat > app.logfmt << 'EOF'
level=info service=api msg="server started" latency=0
level=error service=api msg="connection timeout" latency=3001
level=warn service=worker msg="queue depth high" latency=150
EOF

qk where level=error app.logfmt
```

预期输出：

```
{"level":"error","service":"api","msg":"connection timeout","latency":"3001"}
```

### CSV 格式

```bash
cat > data.csv << 'EOF'
name,age,city
alice,30,NYC
bob,25,SF
carol,35,NYC
EOF

qk where city=NYC data.csv
```

预期输出：

```
{"name":"alice","age":"30","city":"NYC"}
{"name":"carol","age":"35","city":"NYC"}
```

### YAML 格式（多文档）

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
```

预期输出（2 条 enabled=true）：

```
{"name":"api","port":8080,"enabled":true}
{"name":"web","port":3000,"enabled":true}
```

### TOML 格式

```bash
cat > config.toml << 'EOF'
port = 8080
host = "localhost"
debug = false
max_connections = 100
EOF

qk config.toml
```

预期输出（整个 TOML 文件作为一条记录）：

```
{"port":8080,"host":"localhost","debug":false,"max_connections":100}
```

```bash
qk '.port > 8000' config.toml
```

预期输出：

```
{"port":8080,"host":"localhost","debug":false,"max_connections":100}
```

### Gzip 压缩文件（透明解压）

```bash
# 先压缩一份日志
gzip -k app.log      # 生成 app.log.gz，保留原文件

# 直接查询，无需手动解压
qk where level=error app.log.gz
```

预期输出（和查询 app.log 完全相同）：

```
{"ts":"2024-01-01T10:01:00Z","level":"error","service":"api","msg":"connection timeout","latency":3001}
{"ts":"2024-01-01T10:04:00Z","level":"error","service":"worker","msg":"panic: nil pointer","latency":0}
```

### 纯文本（每行作为 line 字段）

```bash
cat > notes.txt << 'EOF'
error: connection refused at 10:01
info: server started
error: timeout after 30s
EOF

qk where line contains error notes.txt
```

预期输出：

```
{"line":"error: connection refused at 10:01"}
{"line":"error: timeout after 30s"}
```

### 同时查询多个文件 + 多种格式

```bash
qk where level=error app.log app.logfmt
```

（并行处理两个文件，输出合并）

### 通配符

```bash
qk where level=error *.log
```

（由 shell 展开通配符，qk 并行处理所有匹配文件）

---

## 管道组合

### 两个 qk 串联

```bash
# 先过滤 error，再按 service 统计
qk where level=error app.log | qk count by service
```

预期输出：

```
{"service":"api","count":1}
{"service":"worker","count":1}
```

### 三级管道

```bash
# 过滤 → 排序 → 限制
qk where level=error app.log | qk sort latency desc | qk limit 1
```

预期输出（最慢的那条 error）：

```
{"ts":"2024-01-01T10:01:00Z","level":"error","service":"api","msg":"connection timeout","latency":3001}
```

### 配合 jq

```bash
# qk 过滤，jq 做后续处理
qk where level=error app.log | jq '.latency'
```

预期输出：

```
3001
0
```

### 配合 grep

```bash
# 先用 qk 过滤格式，再用 grep 精确匹配
qk where service=api app.log | grep timeout
```

### 实时日志（tail -f）

```bash
# 实时监控日志中的 error（需要真实的日志文件）
tail -f /var/log/app.log | qk where level=error
```

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
{"count":2}
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
qk --explain      # 打印解析结果后退出
```

### 关键字模式

```bash
# 过滤
qk where FIELD=VALUE                    # 等于
qk where FIELD!=VALUE                   # 不等于
qk where FIELD>N                        # 数值大于（>=  <  <=  同理）
qk where FIELD~=PATTERN                 # 正则
qk where FIELD contains TEXT            # 包含
qk where FIELD exists                   # 字段存在
qk where A=1 and B=2                    # AND
qk where A=1 or B=2                     # OR

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

