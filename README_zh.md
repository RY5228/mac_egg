# CellE Rust (egg) 实现

本仓库是论文 **CellE: Automated Standard Cell Library Extension via Equality Saturation** 的 Rust 部分实现，核心基于 [`egg`](https://github.com/egraphs-good/egg) 做等式饱和（equality saturation），用于标准单元网表重写与高频子电路挖掘。

- 论文链接：<https://arxiv.org/abs/2603.12797>
- 论文标题：CellE: Automated Standard Cell Library Extension via Equality Saturation

## 功能概览

- 读取标准单元网表（Verilog）与工艺库（Liberty）
- 通过规则文件（JSON）在 e-graph 上做重写
- 将重写后的 e-graph 序列化为 JSON
- 在 e-graph 上进行频繁子图挖掘（GSpan）
- 导出候选融合单元为 `.blif`

## 环境要求

- Rust 工具链（建议使用最新 stable）
- Linux/macOS（项目包含 Linux 下的部分测试/可视化路径）

## 构建

```bash
cargo build
```

## 命令行使用

```bash
cargo run -- \
  --input <netlist.v> \
  --library <library.lib> \
  --output <output_dir> \
  --rules <rule1.json> \
  --rules <rule2.json> \
  --min-support 10 \
  --max-size 5 \
  --max-num-inputs 3 \
  --top-k 10
```

也可以跳过“网表 + 重写”阶段，直接从已序列化的 e-graph 开始挖掘：

```bash
cargo run -- \
  --input-egraph <rewritten_egraph.json> \
  --library <library.lib> \
  --output <output_dir> \
  --top-k 10
```

可用参数可通过下列命令查看：

```bash
cargo run -- --help
```

## 快速示例（使用仓库内测试数据）

```bash
cargo run -- \
  --input test/add2_map_abc.v \
  --library test/asap7sc6t_SELECT_LVT_TT_nldm.lib \
  --output output \
  --rules rules/6t.json \
  --rules rules/6t_comm_rules.json \
  --rules rules/6t_dmg_rules.json \
  --rules rules/6t_inv_rules.json \
  --top-k 10 \
  -v
```

## 输入与输出

输入：
- `--input`：标准单元网表（`.v`）
- `--input-egraph`：序列化 e-graph（`.json`），与 `--input` 二选一
- `--library`：Liberty 库（`.lib`）
- `--rules`：重写规则（`.json`，可重复指定）
- `--cell-area`：可选，单元面积 JSON（用于按面积排序）

输出（在 `--output` 目录下）：
- `rewritten_egraph.json`（或由 `--output-egraph` 指定路径）
- `FUSED_CELL_*.blif`（频繁子电路候选）

## 代码结构

- `src/main.rs`：CLI 入口，串联重写与挖掘流程
- `src/rule.rs`：规则加载与转换
- `src/io/`：Verilog / Liberty / AIGER / Bench 读写
- `src/mining.rs`：频繁子电路挖掘
- `src/language.rs`：e-graph 语言定义
- `extraction-gym/`：提取相关子模块

## 引用

如果你在研究或工程中使用本实现，请引用 CellE 论文：

```text
CellE: Automated Standard Cell Library Extension via Equality Saturation
https://arxiv.org/abs/2603.12797
```
