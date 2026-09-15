- 一旦发现两个地方需要表达同一事实，就不应该开始复制；应该立即停下来，让这个事实只存在于一个类型中，再由编译器生成或约束所有其他能力。
- 编写Rust代码过程中，要对Free function的使用慎之又慎，你如果非要使用，先想好以下几个问题，然后说服我
  1. 你这个东西之所以叫helper，它足够通用吗？它是不是真的一个module到处都要用，还是你只是懒得抽象，把一大堆free function拼在一起当命令式编程凑在一起
  2. 你真的做不出任何抽象，这个东西是不是在可见的未来抽象的收益也是不值得抽象了吗？
  3. 这是一个完全没有内部状态、不依赖任何外部环境的纯数据转换过程（Input -> Output）吗？
  4. 如果只是为了组织和转换一种形式，是什么组织你把它放到对应的scope去实现给特定的trait和sturct？
  5. 你是在协调多个独立实体或提供无状态工具时，如果不是的话，为什么要用？
  6. 这个功能是不是一个极其底层的、与业务实体完全解耦的数学公式、哈希计算或格式化工具，如果不是你为什么要用？
  7. 这个函数是不是代表了当前整个模块（Module）对外暴露的核心能力，而不是某个具体数据结构的专属行为？如果不是为什么要用？
  8. 这个逻辑是不是同时操作了多个地位完全平等的独立类型，以至于把它强塞进任何一个类型的 impl 块中，都显得喧宾夺主、强词夺理？如果不是的话，为什么要用？

- 业务代码文件中不写测试；不要把 `#[cfg(test)] mod tests` 混在 `src/*.rs` 里。
- 测试统一放在对应 crate 的 `crates/<crate>/tests/*.rs` 集成测试目录。

## 1. 核心测试哲学

* **重构抗性（Refactoring Resilience）**：测试必须只验证公开的行为契约，绝不耦合内部实现细节。任何不改变对外公开契约的内部重构，测试必须 100% 保持通过，且无需修改一行测试代码。
* **黑盒行为验证**：断言公开接口的最终状态变迁、返回的领域结果或具体的错误类型。严禁断言内部执行顺序、私有辅助函数调用或中间局部变量。
* **拒绝同义反复（Zero Tautology）**：坚决不写任何单纯重复编译器、类型系统、第三方库或纯静态数据定义的无效测试。

---

## 2. 测试分层与准入矩阵

| 维度 | 单元测试（Unit Test / `rstest`） | 集成测试（Integration Test / `tests/`） |
| --- | --- | --- |
| **覆盖目标** | 业务决策逻辑、状态机流转、领域不变式（Invariants）、复杂算法、自研过程宏/解析器。 | 数据持久化实现（SQL/ORM）、数据库物理约束（唯一索引、外键级联）、事务隔离、完整 API Pipeline。 |
| **运行环境** | 纯内存（In-Memory）。零网络、零真实磁盘、零未 Mock 的系统时钟。 | 真实的临时隔离容器（如 `testcontainers-rs`）或独立测试数据库。 |
| **执行耗时** | 单个用例 `< 10ms`，全套用例秒级完成。 | 单个用例 `< 2s`。 |
| **Mock 策略** | 仅隔离不可控的外部 I/O。**严禁 Mock 领域实体或为了比对 SQL 字符串而 Mock 数据库驱动。** | 严禁 Mock 真实外部依赖；直接与真实容器化存储/服务交互并断言持久化结果。 |

---

## 3. 绝对红线与自动驳回标准（Strict Prohibitions）

在生成代码或提交 Code Review 时，一旦出现以下反模式，**一律直接驳回**：

1. **白盒 SQL / 字符串比对测试**
* *禁止*：Mock 数据库连接后断言生成的 SQL 包含 `"INSERT"`、`"ON CONFLICT DO NOTHING"` 或 `"SELECT"` 等文本子串。
* *正道*：业务逻辑直接测内存实现（如 `InMemoryStore`）；持久化逻辑直接在真实的容器化 Postgres 中测试写入与约束表现。


2. **框架与 Serde 功能重复验证**
* *禁止*：专门写单测去断言使用了 `#[serde(deny_unknown_fields)]` 的结构体在遇到遗留/未知字段时抛错。
* *正道*：通过 OpenAPI / JSON Schema 工具链在 CI 阶段进行静态契约校验，不把框架本身的行为包装成单测。


3. **纯数据载体与胶水代码测试**
* *禁止*：为没有计算逻辑的 Getter/Setter、纯 DTO 转换函数或只有单行转发的 Proxy 方法编写单测。


4. **不稳定与全局环境依赖**
* *禁止*：在单测中直接调用宿主机时钟（如 `Utc::now()`）、未固定 Seed 的随机数，或依赖测试之间的执行先后顺序。



---

## 4. `rstest` 规范与最佳实践

统一使用 `rstest` 消除测试样板代码，提升测试用例的声明性与表达力。

### 推荐规范

* **Fixture 依赖注入（`#[fixture]`）**：
封装可复用的测试环境与领域对象构造，彻底取代冗长的静态工厂辅助类。
```rust
#[fixture]
fn in_memory_store() -> InMemoryTaskStore {
    InMemoryTaskStore::default()
}

#[fixture]
fn active_task() -> Task {
    Task::new(42, "execution-1", TaskStatus::Running)
}

#[rstest]
#[tokio::test]
async fn duplicate_persist_is_idempotent(
    in_memory_store: InMemoryTaskStore,
    active_task: Task,
) {
    // Act
    in_memory_store.persist(&active_task).await.unwrap();
    in_memory_store.persist(&active_task).await.unwrap();

    // Assert
    let loaded = in_memory_store.load(active_task.id()).await.unwrap();
    assert_eq!(loaded, Some(active_task));
}

```


* **表驱动测试（`#[case]`）**：
使用参数化表格表达复杂的领域规则和多边界计算，消除重复的测试函数。
```rust
#[rstest]
#[case(Tier::Standard, 100, 100.0)]
#[case(Tier::Vip, 100, 80.0)]
#[case(Tier::Vip, 500, 350.0)] // 20% 会员折扣 + 满减优惠
#[case(Tier::Svip, 100, 70.0)]
fn discount_calculation_applies_correct_tier_rules(
    #[case] tier: Tier,
    #[case] amount: u64,
    #[case] expected: f64,
) {
    assert_eq!(DiscountEngine::calculate(tier, amount), expected);
}

```


* **笛卡尔积状态覆盖（`#[values]`）**：
用于全量穷举状态机变迁与业务不变式。
```rust
#[rstest]
fn terminal_task_states_cannot_transition_to_active(
    #[values(TaskStatus::Completed, TaskStatus::Failed, TaskStatus::Cancelled)]
    current: TaskStatus,
    #[values(TaskStatus::Pending, TaskStatus::Running)]
    target: TaskStatus,
) {
    assert!(!current.can_transition_to(target));
}

```



### `rstest` 使用红线

* **严禁在测试体内编写断言分流逻辑**：禁止在 `#[case]` 中传入布尔标志并在测试函数内部通过 `if/else` 分别断言 `is_ok()` 与 `is_err()`。成功路径与异常拦截必须拆分为独立的测试函数。
* **保持 Fixture 扁平**：Fixture 嵌套层级严格限制在 2 层以内，禁止跨多个文件建立隐式且深度的对象依赖网。

