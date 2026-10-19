# 语音接入与设置

2026-10-19。首批实现系统原生、本地 Whisper、OpenAI 兼容语音 API；本轮新增本地 Codex（实验），实现与实测限制见 [本地 Codex 语音记录](codex-speech-2026-10-19.zh.md)。

## 接入能力

| 路径 | 识别引擎与处理位置 | 语言与配置 | 当前交互 |
| --- | --- | --- | --- |
| Windows 原生 | robius-speech → 进程内 SAPI；本机处理 | 指定 BCP-47 语言后选择已安装的识别器；留空跟随系统默认 | 实时听写 |
| macOS / iOS 原生 | Apple Speech | 语言、设备、权限影响可用性；可优先要求本机识别 | 实时听写，实际是否离线由系统能力决定 |
| Android 原生 | 系统安装的 SpeechRecognizer 服务 | 系统服务、语言包、权限决定能力；本机优先是平台选项 | 实时听写 |
| Linux / 无原生识别的平台 | 没有内置原生 ASR | 配置支持的音频采集与 HTTP 服务 | 本地或远端服务转写 |
| 本地 Whisper | 连接本机 whisper.cpp 的完整 `/inference` URL | 模型由服务端加载；应用传递语言、上下文及 WAV | 停止录音后转写 |
| OpenAI 兼容 API | OpenAI、兼容第三方平台或本地兼容服务器 | 完整 `/audio/transcriptions` URL、模型、API 密钥环境变量名称、语言、上下文 | 停止录音后转写 |

设置位于 Settings → Speech，提供服务、语言快捷项/自定义 BCP-47 标签、本机识别偏好、服务端点、模型、鉴权环境变量、上下文、录音时限、保存与服务测试。麦克风使用系统默认输入设备，应用不自动切换到其他麦克风或系统音频回环。

旧账户缺少 `speech` 字段时默认继续使用系统原生听写、系统默认语言。用户说中文应明确选择简体中文；该设置不随界面语言变化。此前本机系统 SAPI 默认是英语，因此这项独立设置直接解决语言无法指定的问题。应用不修改 Windows 的全局语音设置。

## 数据和生命周期

- 配置保存到当前账户的 AppPreferences；账户 JSON 仅存 API 环境变量的名称，不存密钥值。环境变量须在启动 Hagency 前设置，未配置时明确报错。无鉴权的兼容本地服务可将变量名称留空。
- 原生仍复用 robius-speech 的增量转写、麦克风状态与现有草稿替换逻辑。指定语言不支持时交由原生识别器明确报错，不偷偷换回英语。
- HTTP 路径通过 Makepad 音频 API 采集默认真实输入设备，将多声道混合、重采样为 16kHz 单声道 PCM16 WAV。录音只存在内存，不写临时音频文件。
- 点击停止或达到时长上限后提交所选服务；Escape 取消采集或中断在途请求。已发出的网络数据无法撤回，但不会继续处理迟到结果。切换房间、账户或关闭时沿用现有取消机制，网络任务属于账户生命周期。
- 用户草稿由既有 Dictation 管理；不会自动发送消息，不把语音内容解释为工具指令。
- HTTP 仅允许本机地址；远端必须 HTTPS。本地 Whisper 必须使用回环地址；API 路径不跟随重定向，不将鉴权内容放进 URL。
- HTTP 录音上限 10–120 秒，UI 提供 30/60/120 秒；连接超时 10 秒、请求超时 120 秒，响应最多 1MiB。无自动云端回退。
- `whisper-1` 等既有模型用单数 `language`；`gpt-transcribe` 使用 `languages[]`，不同时发送两种字段。兼容服务须支持 multipart `file` 和 JSON `text` 响应。
- 服务测试发送一秒程序生成的静音，不开启麦克风；它验证协议和鉴权，不衡量识别准确率。原生模式仅检查平台接口是否可用，语言和麦克风要到实际听写时进一步验证。
- 界面显示固定的错误说明，不显示服务端原始错误、密钥或音频内容；录音内容和识别结果不写日志。

## Codex、Claude 与第三方服务的边界

Codex App Server 存在实验性实时语音协议，但它是有版本和账户条件的交互会话，不等同于通用文件转写 API；稳定的 App Server 文档也不能保证任意本地安装都提供该能力。Claude Code 的程序调用可以处理已有文本，适合作为转写后的翻译/整理步骤。它们的本地登录不是可直接复用于第三方语音端点的 API 密钥，本轮不读取或复制其凭据。

用户选择的三类通用路径已经实现；后续已新增 Codex 实验语音桥接，并明确显示其账户/版本限制，实际成功可用性以独立实测记录为准。Claude 文本后处理、厂商私有的 Azure/讯飞/腾讯/阿里等协议尚未实现为通用兼容接口。如后续增加，需要独立的能力探测、鉴权及协议适配。当前只要厂商提供所需 OpenAI 兼容转写协议，即可通过自定义端点/模型接入。

## 依据

- OpenAI 文件转写：https://developers.openai.com/api/docs/guides/speech-to-text
- OpenAI 转写接口：https://developers.openai.com/api/reference/resources/audio/subresources/transcriptions/methods/create
- Codex App Server：https://developers.openai.com/codex/app-server
- Codex 实验实时语音协议：https://github.com/openai/codex/blob/main/codex-rs/app-server-protocol/src/protocol/v2/realtime.rs
- Claude Code 程序调用：https://code.claude.com/docs/en/headless
- Apple Speech：https://developer.apple.com/documentation/speech/sfspeechrecognizer
- Android SpeechRecognizer：https://developer.android.com/reference/android/speech/SpeechRecognizer
- whisper.cpp 服务协议：https://github.com/ggml-org/whisper.cpp/blob/master/examples/server/README.md
- 当前客户端原生桥接：robius-speech，固定提交 `166991b1b18188ee2d8441f7118721cee2a0c6eb`；Windows、Apple、Android 各后端源码已核实。

## 验证边界

Windows 是本轮可实际启动的目标；其他平台由现有原生桥与 Makepad 音频抽象提供实现，本轮不声称完成了那些系统上的设备实测。HTTP 回归使用回环模拟服务和合成 PCM，覆盖 WAV、协议字段、响应解析、鉴权失败与错误脱敏；没有借用用户真实付费 API 凭据，也没有录下用户环境中的声音。

原生窗口中实际切换了三类表单，通过“测试服务”分别请求本机模拟服务的 `/inference` 与 `/v1/audio/transcriptions`，均收到有效 JSON 转写响应。请求包含合成 WAV；兼容接口包含 `model` 字段，两条请求均未带真实鉴权。此项只能证明应用的配置、请求和响应链路，不能替代真实 Whisper 模型、远端账户或麦克风准确率测试。

语音页展开下拉菜单时，指针事件仅交给当前弹出菜单，防止触发菜单下方的复选框或按钮。回归包含真实绘制后的控件命中对照、手机与桌面宽度、保存事件、音频限长/取消、重采样及两类 HTTP 协议。测试服务器仅在验证期间运行，最终配置不保留它的临时端点。

最终 Windows 验证：完整 library 测试 473 通过、0 失败、2 原有忽略；桌面二进制构建通过。238 源文件样式审计零违规，主题守卫和中英文翻译完整性检查通过。当前账号已通过真实界面保存为 System + zh-CN，持久配置核对通过，本机优先保持开启，两个服务端点保留默认值。最终程序停留在语音设置页；系统接口能力检查通过，真实听写准确率仍须用实际声音验证。
