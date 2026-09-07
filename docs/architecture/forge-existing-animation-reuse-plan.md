# 已有角色动画的零生成复用

本路线将已批准的独立方向 Pack 组合成新的 Godot 审核候选。现阶段是
`scripts/character/prepare_directional_reuse.py` 提供的实验辅助流程，适用于现有
right/down/up 三方向恢复资产，不属于默认 CLI 的通用多方向生成能力。

## 输入与交付

- 输入：三个保有原始 PNG、原生帧时长、原生 pivot、人工批准记录及恢复来源链的
  目录型 `.gsfpack`，以及逐帧哈希绑定的已有方向尺度测量报告。
- 输出：独立 Godot 项目、运行时校准合同、原始资产哈希快照、CLI 安装证据和
  实际引擎运行报告。项目使用 Forge 安装器输出的外部 PNG/atlas。
- 不调用生成 Provider；不生成替代动作；不修改源 Pack、帧像素、帧序、原生时长或
  原批准记录。下游运行时参数属于新的候选，不能继承源动画的人工批准。

## 允许的校准

right/down 保持倍率 1；up 的整个动画采用一个固定均匀倍率，由右向与上向的
稳定尺度中位数比值得到。该倍率乘在安装场景的外层，保留场景内部原生 pivot。
没有逐帧平移、拉伸、重绘、插帧或镜像。所有动画使用 `speed_scale = 1`，不同方向
保留各自周期和非均匀帧时长。

当前尺度代理是围巾连通区域质心到 alpha 足底边界的距离，不是骨骼、真实落脚接触
或解剖尺寸测量。拟合后的中位数一致只能证明数值校准，不能证明角色比例视觉一致。
旧的源尺度失败结论保留；新候选的视觉审核单独记录。

## 执行

依赖 Python 3.9+、Pillow（逐帧 atlas 像素复核）、当前 Forge CLI 和受安装器支持的
Godot 4.6.x。输出目录必须尚不存在，且不能与来源目录重叠。

```bash
python3 scripts/character/prepare_directional_reuse.py prepare \
  --source-root /absolute/recovered-pack-exports \
  --reuse-gate /absolute/existing-video-reuse-gate.json \
  --output /absolute/new-candidate \
  --forge /absolute/forge \
  --godot /absolute/Godot

/absolute/Godot --headless --path /absolute/new-candidate/godot-project -- --qa-auto-quit
/absolute/Godot --path /absolute/new-candidate/godot-project -- --qa-auto-quit

python3 scripts/character/prepare_directional_reuse.py verify \
  --output /absolute/new-candidate

python3 scripts/character/prepare_directional_reuse.py package \
  --output /absolute/new-candidate
```

准备脚本检查源批准的帧哈希和时长，在独立 JobStore/PlanStore 中逐个验证并安装
三个 Pack。安装计划必须明确预计和最大 Provider 请求均为零，否则拒绝执行。
重复试验使用新的输出目录，不覆盖已有候选或源资产。
打包命令核对已验证项目的文件哈希后生成 ZIP；已有 ZIP 时拒绝覆盖。ZIP 不含
`.godot` 缓存，macOS 启动脚本会先运行 Godot 的 `--headless --editor --import`，
待外部纹理导入完成后再进入审核场景。

## 验证与审核

Godot 场景并排播放原始倍率与校准倍率；每组同方向严格同期。下方角色实际通过
`CharacterBody2D.move_and_slide()` 依次右、下、上、右移动。引擎测量全部帧覆盖、
帧时长、纹理区域、原生 pivot、固定缩放、世界锚点、碰撞与位移速度。

验证脚本再独立核对：原始资产目录未变、安装纹理字节相同、实际 SpriteFrames
引用的每块 atlas 像素与批准 PNG 一致、Godot 文本资源不内嵌像素且小于 1 MiB。
有界面运行产生截图；headless 必须记录截图未执行，不能替代实际渲染。

打开项目时默认播放自动比较。右/下/上方向键改为手动移动，空格暂停，B 切换下方
角色的原始/校准倍率，A 重播自动演示。关闭或 Q/Esc 退出。手动干预后若要形成完整
自动验证记录，应重新启动一次 `--qa-auto-quit`。

人工需要比较循环、方向切换时的体型、脚底接触、披风与轮廓。该候选不提供左向
或 idle；不把静止在走路帧称为 idle，不把右向镜像标为已完成的左向动画。
整体视觉批准与生产晋级是后续对具体候选的决定，不由数值门禁自动代替。

## 后续范围

当前先验证已有动作整合。动作模板、分层骨骼驱动、缺帧插值分别需要独立原型和
验收，不作为此流程的隐含后端。现有审核记录和默认生成路线不因本实验被重写。
