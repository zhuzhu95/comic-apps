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
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | 私钥密码；当前私钥未设密码，填空字符串即可 |

> ⚠️ 私钥 `~/.tauri/comic-apps.key` 不要提交到仓库、不要丢失。丢失后无法签发客户端能接受的更新，只能重新生成密钥对并让所有用户手动安装一次新版本。

如果私钥丢失或更换机器，重新生成并同步更新公钥与 Secrets：

```bash
npx tauri signer generate -w ~/.tauri/comic-apps.key
# 把 ~/.tauri/comic-apps.key.pub 的公钥写回 src-tauri/tauri.conf.json 的 plugins.updater.pubkey
```

## 发版流程

1. 提升版本号（保持 `package.json`、`src-tauri/tauri.conf.json`、`src-tauri/Cargo.toml` 三处一致，可用 `npm version patch|minor|major` 后手动同步另两处）。
2. 提交改动并打 tag 推送：

   ```bash
   git tag v0.2.0
   git push && git push --tags
   ```

3. CI（`.github/workflows/release.yml`）自动完成：构建 Windows 安装包与签名更新包 → 创建对应 tag 的 GitHub Release → 上传产物并生成 `latest.json`。
4. 旧版本客户端下次启动时会自动检测到新版本并提示更新。

也可在 GitHub Actions 页面手动触发 `release` workflow（workflow_dispatch）。
