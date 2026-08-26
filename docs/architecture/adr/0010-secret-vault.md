# ADR 0010: API Key 不进 SQLite

## 状态
已接受

## 背景
模型配置曾把 `apiKey` 明文写进 `app_settings`。导出、备份、崩溃转储都会带上密钥。ADR 0007 把接入系统密钥链列为后续。

## 决策
- 宿主注入 `SecretVault` 到内核服务表，与 `StorageHandle` 并列。
- `model_config` 只保存 provider / baseUrl / model。
- 密钥优先写入 OS 密钥链（`com.moshu.novel-agent`）；失败则写应用数据目录 `secrets/` 下权限 0600 的文件。测试使用内存实现。
- 读给 UI 的配置永远不含明文，只带 `apiKeySet`。保存时密钥留空表示保持原值。
- 若旧库 JSON 里还有 `apiKey`，读取时迁到密钥库并改写设置。

## 后果
- SQLite 与作品导出不再包含 API Key。
- 无密钥链的环境（CI、部分 Linux）仍可用文件回退。
- 前端续写请求不再携带密钥，由宿主从 `SecretVault` 填充。
