# Release notes v1.5.2

- 简化 Java 下载流程：删除自定义 `JavaDownloadProgressModal.vue` 进度弹窗，点击版本后直接调用上游 `auto_install_java`，其内置的 `LoadingBarType::JavaDownload` 进度自动显示在 AppActionBar 中
- 移除 `auto_install_java_distribution` 和 `cancel_java_download` 函数及相关 Tauri 命令、JS 绑定和 i18n 消息键，清理未使用的 Rust 静态变量 `JAVA_CANCEL_TX`
- 清理英文和中文 locale 文件中不再使用的 Java 下载消息键

- 重构 Adoptium (Temurin) 下载后端，将 `install_adoptium_runtime` 泛化为 `install_adoptium_api_runtime`，支持通过 `vendor`、`jvm_impl`、`dir_prefix` 参数指定不同发行商
- 新增 IBM Semeru (OpenJ9) 运行时下载支持（`install_semeru_runtime` 包装函数，调用 Adoptium API 获取 IBM/OpenJ9 构建）
- 新增 Azul Zulu 版本列表查询支持（`list_java_distribution_versions` 增加 `"zulu"` 分支，通过 Azul API 获取可用版本）
- Java 发行版下载路由（`auto_install_java_distribution`）现支持 `semeru`/`openj9`/`ibm` 映射到 Semeru 下载，`zulu` 映射到 Azul 下载
- 修复 Adoptium 下载函数中的编译问题：修正 `reporter.as_ref()` 类型错误、闭包签名与 `FetchProgressFn` 不匹配、`io::rename` 改为 `tokio::fs::rename`

- 修复大型文件下载长期停留在单连接的问题：对支持 Range 的大文件主动建立 4 个初始区间，并按聚合吞吐、剩余大小和全局并发余量逐步扩展，最高有效并发为 8。
- 强化 Range 下载完整性：校验 Content-Range 与资源版本，支持有限断流续传；服务器忽略 Range 时安全回退单连接，并保留哈希校验、取消清理和原子落盘。
- 优化 MCIM 重定向下载：首批 Range 可并发通过 HTTPS 重定向，后续重试和扩容复用最终地址，避免重定向缓存锁导致连接串行启动。
- 修复 Java 检测与新手引导 migration 版本冲突，以及 `discovered_javas` 已存在导致的启动失败，并兼容受影响的既有数据库。
- 中文界面下，实例内容页与整合包内容弹窗的已装模组现以「中文名 (英文名)」显示，与「发现内容」页一致，并支持用中文搜索已装内容
- 中文界面下，新下载的模组、资源包、光影包和数据包会以「[中文名]原文件名」保存；查不到中文名时保持原样，修复/重装不会产生重复文件，导出整合包时自动还原为原文件名
- 中文界面下，「发现内容」页直接浏览（不搜索）时也会显示「中文名 (英文名)」双语标题，Modrinth 与 CurseForge 结果均生效
- 新增游戏语言自动跟随启动器语言：实例首次启动（含导入的整合包）时按游戏版本写入正确的语言代码，中日韩语言同时启用 Unicode 字体；已游玩过的实例保留游戏内语言设置。
- 修复皮肤选择器「添加皮肤」按钮在聚焦时强调色高亮描边部分边缘被裁剪、显示不完整的问题。
- 左侧导航栏切换页面时，选中高亮改为滑动过渡动画，与顶部内容类型标签栏保持一致。
- 优化 Java 检测性能：优先读取安装目录的 release 文件判断版本与架构，仅在文件缺失或架构无法识别时才回退到启动 JVM 探测，减少首次扫描时为每个候选启动进程的开销。
- 下载或启动实例时，现在会先搜索本机是否已安装所需版本的 Java，找到则复用，仅在确实没有时才下载新的运行时，避免重复下载。
- 修复 `install_adoptium_api_runtime` 和 `install_semeru_runtime` 缺少 `set_context` 调用的问题，现在下载前会正确设置错误上下文以提供更清晰的诊断信息。
- 修复 `install_semeru_runtime` 在不支持的平台/架构上返回 `Ok(None)` 而非返回错误的问题，与 Adoptium 风格保持一致。
- 修复 Minecraft 1.12.2、1.8.9 等旧版 Forge 安装时部分 Maven Central 依赖无法解析和下载的问题。
- 修复数据库备份被写入 Modrinth 目录的问题，现在改为保存到应用自己的数据目录。
- Java 设置页现改为动态列表，不再限制为 4 个固定版本槽位（Java 8/17/21/25），可自由添加和移除任意版本的 Java 运行时配置。
- 新增「深度扫描」（Deep Scan）和「扫描中……」（Scanning...）i18n 中英文消息条目。
- Java 检测现分为快速扫描（常用目录）和深度扫描（全盘 BFS 搜索）两种模式，可在 Java 选择器中按需切换。
- 新增「快速添加」（Quick add）和「自定义版本」（Custom version）i18n 中英文消息条目。
- 新增 Java distribution（发行商/实现者）信息采集：从 Java release 文件的 `IMPLEMENTOR` 字段提取发行商信息，存储到 `java_versions` 表新加的 `distribution` 列中，用于后续显示 Java 提供方信息。
- Java 设置页重构为统一表格布局：所有已配置的 Java 版本在一张表中显示（版本号、发行版名称、路径、删除按钮），下方提供四个操作按钮：寻找 Java（常用目录快速扫描）、强力查找（全盘 BFS 深度扫描）、手动添加（文件选择器）、下载 Java（版本选择弹窗）。
- Java 扫描结果现在自动添加到已配置版本列表；新增发行版检测（从 JDK release 文件中读取 IMPLEMENTOR 字段）并存储到数据库。
- Java 下载弹窗现提供 4 种发行版选择：Eclipse Temurin、IBM Semeru（OpenJ9）、Azul Zulu，以及默认的 Auto（Recommended）。
- 实例设置中的 Java 选择器重构为下拉菜单，支持三种模式：自动使用最优 Java 版本、从已配置的 Java 版本列表中选择、或自定义路径手动输入（附带检测/浏览按钮）。
- 修复 Adoptium API 下载 Java 时 404 或返回空数组导致崩溃的问题：`install_adoptium_api_runtime` 改为返回 `Option<PathBuf>`，在 API 无结果时优雅返回 `None`，供调用者回退到自动下载逻辑。
- 修复 `list_java_distribution_versions("default")` 返回「Unknown distribution: default」错误的问题：`"default"` 现已作为合法分发版标识，与 Adoptium 共享版本列表查询。
- 重构 java_versions 表主键：从 major_version 改为 path，支持同一主版本号安装多个不同发行版的 Java（如两个 Java 21），全局设置和实例设置均已适配。
- 下载 Java 弹窗移除 Mojang/Auto 选项，仅保留 Eclipse Temurin、IBM Semeru、Azul Zulu 三个显式发行版；修复 Adoptium API 404 和 unknown distribution 错误。
- 修复 `list_java_distribution_versions` 中 `semeru`/`openj9`/`ibm` 误用 Adoptium available_releases 的问题，改为直接查询 IBM 的 GitHub 仓库（ibmruntimes/semeru{ver}-binaries/releases/latest），只返回 IBM 实际发布的版本。
- 修复 Adoptium API v3 响应数据模型：`AdoptiumAsset.binary`（单数）改为 `binaries`（复数数组），`AdoptiumBinary` 新增 `architecture` 和 `os` 字段，`AdoptiumPackage.size` 由 `Option<u64>` 改为非可选 `u64`。
- 修复 `install_adoptium_api_runtime` 对 Adoptium v3 API 的兼容性：改为从 `assets` 的 `binaries` 数组中按 `architecture` 和 `os` 过滤出匹配当前平台的二进制文件，而非直接取首个 asset 的 `binary`。

- 重构 Java 设置页表格数据：将 `tableData` 从 `ref()` + 手动 `refreshTable()` 改为 `computed()`，删除 5 处冗余的 `refreshTable()` 调用，数据变更时自动响应。
- 清理 Java 设置页未使用的导入（`PlusIcon`、`auto_install_java`、`java_discovery_listener`）和未使用的变量/函数（`unlistenDiscovery`、`onDiscoveryUpdate`）。

- 修复快速扫描（Quick Scan）始终返回缓存数据的问题：为 `get_available_jres` 和 `find_filtered_jres` 新增 `force_fresh` 参数，设置页快速扫描按钮调用时传入 `forceFresh=true`，确保新安装的 Java 运行时立即出现在列表中而无需深度扫描或后台重新扫描。
- 修复快速扫描返回缓存数据的问题：新增 force_fresh 参数，"寻找 Java"按钮现在始终执行全新快速扫描，不再使用过期缓存。
- Linux 深度扫描现覆盖所有已挂载非可移动分区（sysinfo::Disks），并额外扫描 /mnt、/media、/run/media 下的子目录，不再仅限 $HOME、/opt、/usr/local。

- 修复实例设置 Java 页面白屏问题：`useMemorySlider()` 返回值被错误转换为普通类型导致 Vue 模板无法自动解包；`globalSettings` 访问缺少可选链式调用，在 settings 未加载时触发 TypeError。
- 下载 Java 弹窗现使用新的 `JavaDownloadProgressModal` 进度弹窗：点击版本开始下载后，弹窗显示非可关闭的下载进度界面，包含发行商名称、版本号、状态文本和取消按钮；取消仅关闭弹窗，下载在后台继续进行。
- 重构 `JavaDownloadProgressModal` 的 `defineExpose` API：`show()` 不再返回 Promise，改为仅显示弹窗并初始化状态文本为「Preparing download...」；`complete(path)` 和 `close()` 分别用于下载成功和失败时关闭弹窗。
- 新增 Java 下载进度弹窗 i18n 中英文消息：progress-title、progress-info、preparing、extracting、downloading-label。

- 修复 Java 版本迁移 SQL 安全性：为 `CREATE TABLE` 添加 `IF NOT EXISTS`、为 `DROP TABLE` 添加 `IF EXISTS` 守卫，`INSERT` 改为 `INSERT OR REPLACE` 并增加 `GROUP BY path` 以处理重复路径边缘情况。
- 为 `remove_java_version`、`list_java_distribution_versions`、`auto_install_java_distribution`、`cancel_java_download` 四个公开函数添加 `///` 文档注释。

- 修复 `DownloadJavaModal.vue` 中 i18n 消息 ID 不匹配问题：`app.settings.java.download.downloading.status` → `app.settings.java.download.downloading-label`，与 locale 中已有的键名保持一致。
- 补充 Java 下载进度弹窗缺失的 i18n 消息：新增 `app.settings.java.download.background` 和 `app.settings.java.download.cancelling` 的中英文条目。
- 修复 `DownloadJavaModal.vue` 中版本按钮硬编码的 `Java {{ ver }}` 前缀，改为通过 `formatMessage(messages.versionLabel, { version: ver })` 使用 i18n 消息。

- 为 Java 管理组件中的纯图标按钮添加 `aria-label` 无障碍标签：
  - `settings/JavaSettings.vue`：删除按钮添加 `aria-label="Delete Java version"`
  - `instance_settings/JavaSettings.vue`：单选按钮容器添加 `role="radio"` 和 `:aria-checked`，测试按钮添加 `aria-label="Test Java path"`，检测按钮添加 `aria-label="Detect Java installations"`
  - `JavaDownloadProgressModal.vue`：咖啡图标容器添加 `aria-hidden="true"`
  - `JavaSelector.vue`：测试按钮添加 `aria-label="Test Java installation"`，安装按钮添加 `aria-label="Install recommended Java"`，检测按钮添加 `aria-label="Detect Java"`，浏览按钮添加 `aria-label="Browse for Java executable"`
   - `DownloadJavaModal.vue`：发行版选择卡片添加 `role="button" tabindex="0"` 及键盘事件处理

- 修复 `DownloadJavaModal.vue` 中提取状态 i18n 消息 ID `app.settings.java.download.extracting.status` → `app.settings.java.download.extracting`，与 `JavaDownloadProgressModal.vue` 使用的已有键名保持一致；同时在 en-US 和 zh-CN 中添加该键名作为后备。

- 修复 `JAVA_CANCEL_TX` 使用 `std::sync::Mutex` 导致 Tauri 异步命令 `Send` 约束失败的问题：改为 `tokio::sync::Mutex` 并使用 `.lock().await`；同步函数 `cancel_java_download` 改为异步函数，其调用者 `apps/app/src/api/jre.rs` 同步调用改为 `.await`。

- 修复 Azul Zulu 下载时因 Azul API 与 CDN 报告的文件大小不一致导致的「Incorrect size for download」错误：移除 `install_azul_runtime` 中 `Integrity` 构造的 `size` 字段，下载完整性校验仅依赖 SHA-256 哈希。
