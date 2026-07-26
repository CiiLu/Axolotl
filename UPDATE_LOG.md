# Release notes v1.5.4

- 将 Find Java 按钮恢复为精确路径扫描（PATH、JAVA_HOME、已知安装目录），不再使用 BFS 关键字搜索
- 扩展 Linux 平台的 Java 搜索路径：~/.sdkman/candidates/java/、~/.asdf/installs/java/、~/.jabba/jdk/、/snap/
- 扩展 macOS 平台的 Java 搜索路径：~/Library/Java/JavaVirtualMachines/、~/.sdkman/candidates/java/
- 扩展 Windows 平台的 Java 搜索路径：%LOCALAPPDATA%/Programs/、%APPDATA%/（含 java/jdk/jre 关键字匹配）

- 新增透明背景功能，可让桌面透过启动器窗口显示，并用滑块调节界面不透明度
- 新增背景模糊开关，可将透出的桌面做磨砂玻璃处理
- 启用透明背景时会隐藏自定义背景图片
- 添加 `xz2` 依赖，支持 XZ 压缩格式
- 实现 JetBrains JDK 订阅源下载与解析功能（`fetch_jdk_feed`、`list_java_feed_vendors`、`list_java_feed_versions`、`download_java_from_feed`）
- 新增 Java 下载对话框的 i18n 条目：`select-vendor`、`no-vendors`、`select-version-feed`，并更新 `select-version` 的措辞
- 新增 Tauri 命令、JS 辅助函数和 ACL 条目，将 JetBrains JDK 订阅源接口暴露给前端
- 将 `JdkVersionInfo` 加入 prelude 直接导出，简化 Tauri 侧的导入路径
- 新增穷举式 BFS 扫描（`bfs_exhaustive_scan`），不设关键字过滤和深度限制，遍历排除目录外的所有目录
- 添加 `exhaustive` 参数贯穿 `collect_candidate_paths`、`get_all_jre`、`refresh_discovered_javas`、`get_available_jres`、`find_filtered_jres` 及 Tauri 命令
- 前端 Java 设置页新增"Find Java"（关键字 BFS 扫描）和"Deep Scan"（穷举扫描）按钮，深扫前弹出确认提示
- Java 下载弹窗点击下载后立即关闭，不再等待下载完成；下载完成后仍会发送 downloaded 事件
- 扩展 Windows 平台 Java 扫描路径：自动遍历 `C:\Program Files\` 和 `C:\Program Files (x86)\` 下所有子目录，同时添加 `C:\Program Files\Microsoft\` 路径支持 Microsoft OpenJDK
