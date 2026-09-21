# Focus on repeatable Godot asset delivery

Decision: 2026-09-21. Status: adopted for new development; existing commands and
formats remain compatible. This does not change the v0.6.4 release or consumer pins.

The product promise is to prepare reviewed assets, update them in Godot, verify
the result and preserve a useful recovery path. Agents handle intent, generation,
game code and task orchestration. Forge must earn its installation and maintenance
cost through repeated use; a comprehensive protocol is not evidence of demand.

## Investment boundary

| Area | Decision |
| --- | --- |
| Local PNG/frame/WAV processing, coordinates, timing and engine resources | Maintain and improve from reproducible consumer cases. |
| Owned updates, rollback, final installation checks and useful evidence | Core reliability work. Do not weaken guarantees to simplify the interface. |
| Embedded usage guide | Present the task and result first; let the agent manage internal records. |
| Catalog, previews, Providers, Godot setup and export | Maintain existing users; additions need repeated consumer use or a demonstrated defect. |
| General generation/agent platform, new editor, optional character/world expansion | Pause new investment pending the comparison decision below. |

Do not remove commands, downgrade checks, rewrite receipts or migrate projects as
part of this decision. New CLI capabilities are not the default answer to a
workflow problem: first test whether a short guide change or reusable script is
sufficient. Keep libraries optional for a one-off delivery. Human review and
installation authority still apply when an agent hides protocol details.

## Evidence before expansion

Use the [comparison protocol](../qa/asset-delivery-comparison.md) for at least two
independently maintained game projects and three paired iterations per project.
The baseline may retain and improve its scripts/skills between iterations. It
must not be forced to start from scratch or reproduce Forge's internal formats.

The initial decision threshold is a proposed product gate, not a measured result:
both paths must satisfy the same user acceptance; Forge should reduce the median
paired active human time by at least 20% in each project, without more unresolved
delivery failures. Across the observation window, saved human time must also
exceed incremental Forge support/maintenance time. Include adoption/setup costs;
report machine time and monetary cost separately. Unknown effort or missing cases
produce **insufficient evidence**, not zero cost or a win.

- If this gate passes, invest in the specific repeatedly beneficial workflow.
- If native import/reusable scripts are sufficient, keep Forge as a smaller
  optional CLI/skill toolset and maintain consumers without expanding scope.
- If results are mixed, improve the measured bottleneck and repeat the affected
  pairs; do not remove failed or inconvenient runs from the record.

The public static-delivery pilot measures a narrow ready-PNG task. It validates
the measurement path and can reveal overhead; it cannot pass this product gate.

## 中文决策

当前保留素材加工、Godot 原生交付、更新回滚和必要的来源证据作为核心。资源库、预览、
Provider、引擎配置和导出维持兼容，根据重复出现的消费需求改进；暂停扩展通用生成与
智能体平台、新编辑器以及可选角色/世界能力。此决策不删除现有功能，不改写用户项目。

扩大投入前，在两个独立维护的游戏项目各做至少三轮配对实验，允许对照组复用脚本。
两组需满足相同验收；建议门槛是各项目人工耗时中位数至少降低 20%，未解决交付失败
不增加，观察期内节省人力超过新增支持维护成本。缺失数据判为证据不足。当前 PNG
试验只衡量狭窄场景的运行开销，不证明长期生产力或市场需求。
