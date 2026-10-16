# ComicReader 漫画阅读器

基于 Tauri 2 + Vue 3 的本地漫画阅读器，支持文件夹散图 / zip / pdf。

## 开发

```bash
npm install
npm run tauri dev
```

## 构建

```bash
npm run tauri build
```

## 自动更新

应用启动时自动检查 GitHub Releases 上的新版本并提示更新（可在「设置」页关闭）。更新包使用 minisign 签名，公钥配置在 `src-tauri/tauri.conf.json` 的 `plugins.updater.pubkey`。

### 一次性配置（仓库 Secrets）

发布带更新产物的 Release 前，需要在 GitHub 仓库 **Settings → Secrets and variables → Actions** 添加：

| Secret | 值 |
| --- | --- |
| `TAURI_SIGNING_PRIVATE_KEY` | `~/.tauri/comic-apps.key` 文件的全部内容（`cat ~/.tauri/comic-apps.key` 查看） |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | 私钥密码；当前私钥未设密码，**可以不创建此 secret**（引用不存在的 secret 会得到空字符串，正好等于无密码） |

> ⚠️ 私钥 `~/.tauri/comic-apps.key` 不要提交到仓库、不要丢失。丢失后无法签发客户端能接受的更新，只能重新生成密钥对并让所有用户手动安装一次新版本。

如果私钥丢失或更换机器，重新生成并同步更新公钥与 Secrets：

```bash
npx tauri signer generate -w ~/.tauri/comic-apps.key
# 把 ~/.tauri/comic-apps.key.pub 的公钥写回 src-tauri/tauri.conf.json 的 plugins.updater.pubkey
```

## 发版流程

以发布 **v0.2.0** 为例，共 5 步：

**第 1 步：把三个文件的版本号改成 0.2.0**

- `package.json` → `"version": "0.2.0"`
- `src-tauri/tauri.conf.json` → `"version": "0.2.0"`
- `src-tauri/Cargo.toml` → `version = "0.2.0"`

**第 2 步：更新 Cargo.lock**

```bash
cd src-tauri && cargo check && cd ..
```

这一步会把 `Cargo.lock` 里的版本号同步成 0.2.0（版本号不一致会导致打包出来的应用仍显示旧版本）。

**第 3 步：提交并推送**

```bash
git add -A
git commit -m "发布 v0.2.0"
git push
```

**第 4 步：打 tag 并推送（这一步触发 CI 发版）**

```bash
git tag v0.2.0
git push origin v0.2.0
```

tag 必须以 `v` 开头，workflow 才会触发。

**第 5 步：确认发布结果**

1. 打开仓库的 **Actions** 页面，能看到 `release` workflow 在运行，Windows 构建大约需要 10~20 分钟。
2. 跑完后打开 **Releases** 页面，应该出现 **v0.2.0**，附件里要有：
   - `.msi` / `.exe` 安装包
   - 对应的 `.sig` 签名文件
   - `latest.json`（自动更新的清单文件）

有这些产物就说明自动更新链路通了，旧版本客户端下次启动时会自动检测到新版本并提示更新。

也可在 GitHub Actions 页面手动触发 `release` workflow（workflow_dispatch）。
