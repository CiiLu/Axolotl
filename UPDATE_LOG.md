# Release notes v1.5.4

- 新增透明背景功能，可让桌面透过启动器窗口显示，并用滑块调节界面不透明度
- 新增背景模糊开关，可将透出的桌面做磨砂玻璃处理
- 启用透明背景时会隐藏自定义背景图片
- 修复 `java_globals.rs` 中 rebase 冲突解决错误：恢复 `distribution` 字段、`get_all()` 返回 `Vec<JavaVersion>`、`delete(&str)` 方法、`ON CONFLICT(path)` 及所有查询中的 `distribution` 列
