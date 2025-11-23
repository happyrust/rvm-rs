# Hierarchy Tools Implementation Status

## Overview

层次工具模块（正则扁平化、保留、丢弃）的基础结构已创建，但完整实现需要更多时间和测试。

## 已完成

✅ **基础结构**:
- 模块入口（`hierarchy/mod.rs`）
- `HierarchyError` 错误类型定义

## 待实现

以下功能已规划但需要进一步实现：

### 1. 正则表达式扁平化 (`hierarchy/flatten_regex.rs`)
- [ ] `RegexFlattener` 结构体
- [ ] 正则表达式编译和验证
- [ ] 递归子节点处理
- [ ] 几何体和属性移动
- [ ] 最低层级保留逻辑

### 2. 保留组扁平化 (`hierarchy/flatten_keep.rs`)
- [ ] `KeepFlattener` 结构体
- [ ] 标签文件解析
- [ ] 节点标记（选中和祖先）
- [ ] 新 Store 创建
- [ ] 剪枝复制逻辑

### 3. 组丢弃 (`hierarchy/discard.rs`)
- [ ] `GroupDiscarder` 结构体
- [ ] 标签文件解析
- [ ] 递归子节点剪枝
- [ ] 节点移除

### 4. Store 扩展
需要为 Store 添加以下方法：
- [ ] `clone_node` - 克隆节点
- [ ] `clone_geometry` - 克隆几何体
- [ ] `remove_node` - 移除节点
- [ ] `move_geometries` - 移动几何体
- [ ] `move_attributes` - 移动属性
- [ ] `take_children` - 获取并清空子节点

## 实现建议

如果需要完整实现这些功能，建议按以下顺序进行：

1. **首先**: 扩展 Store 的克隆和移动方法
   - 实现节点克隆
   - 实现几何体克隆
   - 实现节点移除

2. **然后**: 实现丢弃功能（最简单）
   - 解析标签文件
   - 递归移除节点

3. **接着**: 实现正则扁平化
   - 编译正则表达式
   - 递归处理节点
   - 移动几何体和属性

4. **最后**: 实现保留操作（最复杂）
   - 标记选中节点
   - 创建新 Store
   - 复制保留的节点

## 命令行集成

需要在 `main.rs` 中添加以下选项：
- `--keep-regex=<pattern>` - 正则表达式扁平化
- `--keep-groups=<file>` - 保留组列表
- `--discard-groups=<file>` - 丢弃组列表

## 测试策略

- 创建包含复杂层次结构的测试 RVM 文件
- 测试正则表达式匹配
- 测试标签文件解析
- 验证几何体和属性保留
- 验证层次结构一致性

## 依赖

需要添加 `regex` crate：
```toml
[dependencies]
regex = "1.10"
```

## 参考

完整的设计文档位于：`.kiro/specs/rvm-hierarchy-tools/design.md`

