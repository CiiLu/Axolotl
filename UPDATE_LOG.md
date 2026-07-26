# Release notes v1.5.4

- 新增透明背景功能，可让桌面透过启动器窗口显示，并用滑块调节界面不透明度
- 新增背景模糊开关，可将透出的桌面做磨砂玻璃处理
- 启用透明背景时会隐藏自定义背景图片
- 添加 `xz2` 依赖，支持 XZ 压缩格式
- 实现 JetBrains JDK 订阅源下载与解析功能（`fetch_jdk_feed`、`list_java_feed_vendors`、`list_java_feed_versions`、`download_java_from_feed`）
- 新增 Java 下载对话框的 i18n 条目：`select-vendor`、`no-vendors`、`select-version-feed`，并更新 `select-version` 的措辞
- 新增 Tauri 命令、JS 辅助函数和 ACL 条目，将 JetBrains JDK 订阅源接口暴露给前端
- 将 `JdkVersionInfo` 加入 prelude 直接导出，简化 Tauri 侧的导入路径
