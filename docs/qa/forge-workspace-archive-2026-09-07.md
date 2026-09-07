# 未合并历史工作备份

本分支保存同步日常工作区到 main 前的历史源码、schema、脚本、skill 和文档。
这是待继续整理的 WIP 快照，未作为已验证实现合并到 main，也不代表已发布能力。

完整保全记录见 [文件清单与 SHA-256](artifacts/forge-workspace-archive-2026-09-07/manifest.json)。
原 692 个变更文件均已另存到仓库外的 `pending-files.tar.gz` 并逐文件验证。
其中 394 个源码和文档文件进入本分支；298 个 QA artifacts 保存在本地完整档案，
包括完整临时生成工作区、Pack 和 Godot 工程，按 QA 产物政策不上传 Git。
Git 已忽略的源素材、交付包和缓存仍留在原项目目录。

恢复历史开发时，应从本分支另建工作区，再按清单从本地档案恢复所需 QA 文件；
不要把快照直接覆盖到 main。没有为备份提交重跑全部功能测试，也未调用真实 Provider。
