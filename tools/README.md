# tools/ —— 开发期辅助脚本

这里的脚本只用于开发与排查，不属于应用本体。

## 注意：注入输入的脚本可能"卡键"

若脚本按下某个修饰键（Ctrl / Win / Alt / Shift）后**在中途被打断**，
那个键会一直处于按下状态，整个键盘的行为都会跟着变
（例如 Win 卡住时，单按 A 等于 Win+A，会弹出右下角的快捷设置）。

因此：
- 注入按键的脚本一律用 `try/finally` 保证释放；
- 用完后如果发现键盘行为异常，立刻运行：

```powershell
powershell -NoProfile -File tools/release-stuck-keys.ps1
```

它会向所有虚拟键和鼠标键发送抬起事件（对没按下的键是无害的空操作），
并复查修饰键是否已全部释放。

## 分类

- **只读诊断**：find-widget / dump-windows / dump-progman / zorder /
  who-is-on-top / hit-test / check-pin / find-desktop / dump-desktop-tree
- **截图**：screenshot（整屏）、capture-hwnd / capture-window（按窗口句柄）
- **注入输入**（有卡键风险，已加 `try/finally`）：hotkey-test / click-test / drag-test
- **救援**：release-stuck-keys
