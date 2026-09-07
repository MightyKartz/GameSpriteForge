# 现有三方向角色动画零生成整合验收

日期：2026-09-07。结论：工程验证通过，已交付独立 Godot 比较候选；整体视觉审核
尚未作决定，未晋级生产。新增 Provider 请求为 0。

## 实施结果

- 新增 `scripts/character/prepare_directional_reuse.py`，校验批准帧与来源链，通过现有
  Forge CLI 进行 Pack 验证、零请求安装计划和安装执行，保留独立 JobStore/PlanStore。
- 新增 `scripts/character/godot/` 审核场景，比较原始倍率与校准倍率，并实际执行
  `CharacterBody2D.move_and_slide()` 的右、下、上、右移动。
- 新增七项脚本回归测试和[使用说明](../architecture/forge-existing-animation-reuse-plan.md)，
  在产品使用 skill 的可选工作流中加入入口。没有修改 Rust 源码或默认 CLI 功能。

本轮复用了现存 09-02 recovery exports 的 52 帧，而非依赖已不存在的 V18 历史实验
目录。源帧、源 Pack、原批准记录与原生时序均保持不变。

| 方向 | 帧数 | 原生周期 | 原生 pivot | 整段固定校准倍率 |
| --- | ---: | ---: | --- | ---: |
| right | 12 | 2041 ms | (769, 1213) | 1 |
| down | 24 | 1833 ms | (720, 1351) | 1 |
| up | 16 | 2000 ms | (720, 1357) | 0.9399019056 |

所有方向 `speed_scale = 1`。没有沿用历史统一 1800 ms 的加速配置。倍率位于安装
场景外层，各帧内部位置、纹理区域和时长保持原状。

## 实际执行的检查

1. 独立来源审计：52 张帧 PNG、三个 manifest、Pack 内帧、原人工批准、恢复证明和
   来源哈希匹配；52 帧重测的稳定尺度距离与 09-03 记录在六位小数精度一致。
2. Python 回归测试：7 项通过，覆盖源/Pack 哈希篡改、批准时序不匹配、输出路径
   保护、非零或缺失请求预算拒绝，并验证不安全计划无法进入 execute。
3. CLI：三个 Pack 均 `valid=true`；三次本地安装均 succeeded，实际请求计数均为 0。
4. Godot 4.6.3：headless 自动运行与窗口 OpenGL 渲染均通过。headless 未截图，
   窗口运行另有真实 PNG 截图。
5. 每段实际移动约 184 px，测得速度约 79.999 px/s；最大位移误差小于 0.003 px。
   四次切换的源 pivot 世界坐标跳变量均为 0；共享碰撞参数不变。
6. 实际 SpriteFrames 完整覆盖 52 帧；帧时长相对原始合同误差小于 0.05 ms。
   原始/校准成对预览的帧号及帧内进度差均为 0。
7. 独立读取引擎报告的实际 atlas 路径、region、margin：52 块 RGBA 像素均与批准
   PNG 一致。13 张安装纹理字节一致；7 个 `.tres/.tscn` 均为外部纹理且小于 1 MiB。
8. 引擎负例：在独立临时合同中将 up 首帧时长加 10 ms，正确返回
   `native_resource_contract` 失败和退出码 1；没有改动源素材。
9. 最终 ZIP 在独立临时目录解压，从没有 `.godot` 缓存的状态执行启动脚本：
   本地纹理导入完成，自动移动验证通过，退出码 0。ZIP 中启动脚本保有可执行权限。

循环事件测量受渲染回调时刻量化影响，采用报告中记录的有界容差（上限 100 ms）；
不将其表述为逐事件零误差。原生帧时长的资源合同另按 0.05 ms 验证。

## 校准解释与视觉边界

上向倍率来自 `737.589513 / 784.751588`。稳定尺度是围巾颜色连通区域质心到
alpha 足底边界的代理指标，不是独立的骨骼或解剖测量。拟合中位数一致是计算结果，
不能当作体型、轮廓、真实落脚点或无滑步的人工验收。

已检查真实截图：三方向均正常渲染，校准前后并排可比较，标题及角色没有裁切。
整体效果仍标 `visualReview: pending`。旧 up 源尺度门禁失败保留；本轮没有改写
09-03 的历史结论或将三个源动画的批准复制成新整合批准。

本轮只实现现有三方向整合；没有增加左向、idle、骨架、动作迁移、RIFE/FILM
插帧或 SpriteCook 接入。源 Provider 的历史调用不计为本轮请求，也不声称历史成本为零。
未运行完整 Rust 测试、默认发布矩阵或远端 CI；此次改动属于 Python/Godot 辅助流程、
文档和指令，实际验证范围如上。

## 复现与交付

在仓库根目录执行（重复运行应选新的 output）：

```bash
python3 scripts/character/test_prepare_directional_reuse.py -v

python3 scripts/character/prepare_directional_reuse.py prepare \
  --source-root generated-assets/experiments/three-direction-compatibility-review-v1-20260902/recovered-pack-exports \
  --reuse-gate generated-assets/experiments/direction-geometry-lock-v1-20260903/existing-video-reuse-gate.json \
  --output generated-assets/experiments/directional-reuse-v1-20260907 \
  --forge target/debug/forge \
  --godot /Applications/Godot.app/Contents/MacOS/Godot

/Applications/Godot.app/Contents/MacOS/Godot --headless \
  --path generated-assets/experiments/directional-reuse-v1-20260907/godot-project -- --qa-auto-quit

/Applications/Godot.app/Contents/MacOS/Godot \
  --path generated-assets/experiments/directional-reuse-v1-20260907/godot-project -- --qa-auto-quit

python3 scripts/character/prepare_directional_reuse.py verify \
  --output generated-assets/experiments/directional-reuse-v1-20260907

python3 scripts/character/prepare_directional_reuse.py package \
  --output generated-assets/experiments/directional-reuse-v1-20260907
```

完整项目、安装证据、来源快照、校准合同及 ZIP 保留在忽略目录
`generated-assets/experiments/directional-reuse-v1-20260907/`。ZIP 不含 `.godot` 缓存
或临时 JobStore/PlanStore；macOS 解压后可用 `open-review.command` 启动。
独立解压测试发现直接运行未导入的源项目会缺少纹理缓存，已在启动脚本中加入
首次本地导入步骤；最终交付再次从无 `.godot` 缓存的目录验证。

仓库保留精简证据：

- [机器摘要](artifacts/forge-directional-reuse-2026-09-07/summary.json)
- [Godot 窗口报告](artifacts/forge-directional-reuse-2026-09-07/runtime-report.json)
- [Godot headless 报告](artifacts/forge-directional-reuse-2026-09-07/runtime-report-headless.json)
- [错误时长负例](artifacts/forge-directional-reuse-2026-09-07/negative-duration-summary.json)
- [独立解压验证](artifacts/forge-directional-reuse-2026-09-07/portable-verification.json)
- [三方向比较截图](artifacts/forge-directional-reuse-2026-09-07/review.png)
- [上向移动截图](artifacts/forge-directional-reuse-2026-09-07/up-phase.png)

最终文件哈希、链接和凭据扫描结果见
[证据清单](artifacts/forge-directional-reuse-2026-09-07/evidence-manifest.json)。
