# Focus on repeatable Godot asset delivery

Decision: 2026-09-21. Status: adopted for new development; existing commands and
formats remain compatible. This does not change the v0.6.4 release or consumer pins.

Follow-up: the [agent resource production roadmap](agent-resource-production-roadmap.md)
refines the investment order around agents as the direct users. Prioritize the
animation preparation, inspection, correction and Godot delivery loop, with audio
and static assets sharing its task experience. Updates and recovery remain core
guarantees. The roadmap preserves compatibility and versions the evidence gate below;
its planned capabilities are not claims about the current release.

后续优先级以 [面向 Agent 的资源生产计划](agent-resource-production-roadmap.md) 为准：
先打通角色动画的处理、检查、修正和 Godot 交付，再统一音频与静态素材体验。
更新恢复作为底层保障；兼容性要求保持，收益门槛按下述 v2 前瞻修订。

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

**2026-09-21 amendment, before real M4 pairs:** agents are the direct users.
The original human-time gate is retained in the [v1 protocol](../qa/asset-delivery-comparison-v1.md).
New observations use **agent-delivery-v2** and the [implementation plan](agent-first-m4-plan.md).
Do not reinterpret the historical PNG pilot as v2 evidence.

The initial v2 gate is a product decision threshold, not a measured result:
median paired agent task/setup elapsed savings of at least 20% in each project,
with no lower technical/autonomous completion rates, no additional unresolved
delivery failures or unplanned human rescue, and no source/history/pin corruption.
Use all started tasks for success rates and effort; timing medians require at least
three mutually accepted real pairs per project in the same stream. Net observed
agent time savings must exceed additional agent maintenance. Human effort and
maintenance remain separately visible; missing model bills prohibit cost claims.
Missing primary timing, correctness, autonomy or maintenance leaves the gate
**insufficient evidence**. The v2 protocol defines windows, failure accounting,
generation separation and maintenance allocation; do not select another winning
metric after observing outcomes.

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

扩大投入前，在两个独立维护的游戏项目各做至少三轮真实配对，允许对照组复用脚本。
2026-09-21 在真实配对前把原人工主门槛归档为 v1：直接使用者是 Agent，v2 以各项目
Agent 任务及设置耗时配对节省中位数至少 20% 为初始门槛，自主/技术成功率不降低，
未解决失败及非预期人工救援不增加，净节省覆盖新增 Agent 维护投入。人工投入单列，
缺失主指标或维护数据判为证据不足；费用未知不宣称成本下降。当前 PNG 实验只衡量
狭窄场景运行开销，不证明 Agent 生产力、长期收益或市场需求。
