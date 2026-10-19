# 本地 Codex 实验语音接入

2026-10-19。用户要求新增“本地 Codex（实验）”，复用 Agent 的本地配置和登录，只填草稿、不自动发送，并继续排查到真实转写成功。

## 已确认的根因

1. 既有隔离 app-server 使用 env_clear，Windows 下未传 SystemRoot / WINDIR。最小环境的 account/read 报 workspace routing discovery failed；仅补回这两个系统路径就能验证 chatgpt 账户。修复同时应用于 Agent 配置探测、运行子进程和语音子进程，仍不传 OPENAI_API_KEY 或其他凭据环境变量。
2. Codex WebSocket realtime 路径要求 API key，不能满足本地登录要求。Codex 自有 WebRTC call creation 可复用 ChatGPT 登录。本次实现使用 v3 + WebRTC，由 Codex 自行鉴权和 SDP 信令；原生 Rust WebRTC 仅处理媒体，不读取、提取或复制登录 token。
3. Windows Codex 会规范化 verbatim 前缀和重复路径分隔符。受保护配置验证现比较规范化词法路径及真实文件，并继续验证指令文件内容；仍拒绝 parent traversal/别名。程序发现优先 PATH 的原生 codex.exe；无 HOME 时可用 USERPROFILE 定位 .codex。
4. Windows MSIX 环境还把应用数据路径重定向到更深的 Packages/.../LocalCache 目录。本机 provider-home 的真实路径含前缀共290字符，CreateProcess 的 current directory 直接报267（NotADirectory），独立D盘短目录测试未覆盖此问题。hagency-platform新增 process_current_directory：必要时用系统 GetShortPathNameW 获取8.3名，核对 canonical identity不变，再用于Codex配置探测和实际子进程启动；未创建junction或改动权限。短名称不可用且仍超长则拒绝，不改用其他工作目录。Windows长路径回归通过，实际应用provider目录 sealed_shared_overrides已由失败变为通过。
5. 桌面完整依赖图同时启用rustls ring与aws-lc，DTLS默认verifier无法自动选择CryptoProvider，在后台握手任务panic，UI只表现为超时；单独引擎测试只有ring，因而未暴露。现Peer初始化显式选ring（尊重已安装provider）；可选TSP初始化也容忍此前已选定provider。新增offer回归检查provider已安装，并用hagency+hagency-app联合构建的真实资格测试验证，不能只测独立引擎图。
6. 最后应用层静态诊断确认owner_authorization_required。Hagency当前授权证明至多30秒，旧快照不会随后台续期自动延长；语音结果会被过期快照误拒绝。NativeOwner语音入口改为处理期间按5秒或剩余期限一半续验，固定完整DeviceIdentity（含issuer/subject/owner/device/generation），授权期限为硬停止线；续验失败/超时、撤销或身份改变即丢弃并取消media。结束时还重新核对身份及本地Codex选择。新增跨旧期限成功、撤销/换身份丢弃、卡住不能延长证明三项回归。
7. 本机没有已选本地账户，现有 Agent 均分配在另一设备，无法借那些 Agent 的资源页配置本机。语音页补充“连接本地 Codex”入口，调用与 Agent 完全相同的 owner-scoped Provider UseLocal 流程，关联结果也供本机 Agent 共用，不移动 Agent 执行设备。

## 产品行为

- Settings → Speech → Local Codex (experimental)，中文“本地 Codex（实验）”。保留已有语言、上下文和录音时限；隐藏 HTTP 端点、转写模型和 API key 表单。
- 先在本机 Agent 资源页或语音页连接本地 Codex；使用相同 ProviderPaths、shared choice、程序发现、密封配置和 account/read。只接受 chatgpt 登录，不支持 API key 账户。
- Makepad 默认输入设备采集内存 PCM16 mono 16kHz；停止后原生 WebRTC 降采样为 8kHz PCMU/G.711，20ms 分帧发送。音频由 Codex 在线语音服务处理，并非离线识别。
- 每次使用独立 ephemeral/read-only thread，无 turn/start、Room 发送接口或工具执行；拒绝 server tool requests。v3 要求 audio output，但接收音频直接丢弃，不播放；助手转写及原始 item 均丢弃，只接受同一 thread 最终 role=user 文本，经现有 Dictation 写入草稿。
- 停止后增加2秒静音帮助VAD结束；最长再等30秒，最后 user final 后2秒安静且无待定 user delta 才完成。总时限为录音长度加60秒，UI backstop为配置时限加75秒；最长录音仍120秒。HTTP时限保持原逻辑。
- Escape、账户或房间切换继续取消任务并丢弃迟到结果。失败、空转写不改原草稿。取消/完成关闭 peer 并结束独立 app-server，不调用可能把会话尾部交给 coding turn 的 realtime/stop。
- JSONL帧缓冲改为进程持有，媒体进度取消异步读取时保留半行，新增部分行取消回归。WebRTC peer drop负责取消路径的连接清理。
- “测试服务”发送合成静音，不开麦克风。成功表示连接及音频通路可用，真实准确率另外用已知语音样本验证。不存在自动回退到其他服务。

## 实际在线验证

Codex App 自带 0.163.0-alpha.10.2。所有以下成功结果均使用当前本地 ChatGPT 登录、生产 transcribe 入口和密封配置；未导入 API key，未记录用户环境声音、未向 Room 发送消息。

- 英文 Windows SAPI 合成输入：Hagency speech input test. Keep this in the draft.
  实际输出：Hagency speech input test. Keep this in the draft
  显式资格测试1通过，用时37.30秒。报告 D:/BuildCache/hagency-client/codex-speech-native-webrtc-live-1/qualification.json，available=true。
- 中文 Windows SAPI Huihui 合成输入：这是本地语音转写测试。请只填入草稿，不要自动发送。
  实际输出：这是本地语音转写测试,请只填入草稿,不要自动发送
  显式资格测试1通过，用时38.23秒。报告 D:/BuildCache/hagency-client/codex-speech-native-webrtc-live-zh-1/qualification.json，available=true。
- 此前 unavailable / account rejected 结论已被以上修复和真实转写成功结果取代；不能再描述为本机无法转写。仍不声称完成真实麦克风或其他操作系统的准确率测试。

## 回归与复现

- 引擎完整lib208通过、0失败、5显式在线等忽略；Codex适配定向21通过、0失败、3忽略；语音协议/媒体4通过。新增Windows环境白名单、部分JSONL取消、PCMU编码及原生SDP回归。
- 桌面连接入口后的完整lib477通过、0失败、2忽略（codex-webrtc-final-app-tests.log）；最后Windows路径修复的引擎复验与实窗结果见下方。
- 构建、缓存、日志、合成音频均放D:/BuildCache/hagency-client。tools/dev.ps1设置CARGO_TARGET_DIR并通过cargo metadata核对D盘。未执行Docker构建。
- 用tools/dev.ps1编译hagency lib测试，设置HAGENCY_CODEX_SPEECH_TEST_HOME为不存在的D盘私有测试目录，HAGENCY_CODEX_SPEECH_TEST_PCM指向已知合成PCM16/16kHz/mono样本；显式运行console::owner_provider::speech::tests::live_codex_speech_qualification --exact --ignored --nocapture。可选HAGENCY_CODEX_SPEECH_TEST_LOCALE及HAGENCY_CODEX_SPEECH_TEST_EXPECTED覆盖语言和断言词。
- 静态报告仅保存错误码或已知合成样本转写，不输出登录信息、SDP或服务端原始错误。

- 真实ASR有时在中文词间插空格或把一句拆成多个最终片段；资格报告保留原文，测试对预期词进行空白归一化后比对。此前有两次识别返回available=true但严格连续子串断言失败，属于格式断言而非网络失败，不能把这些失败计入通过数。

## 最终构建与实窗

- 媒体修复后的GUI bin构建成功，codex-webrtc-crypto-app-build.log，2m29s；可见PID68696。完整依赖图引擎208通过/5忽略，桌面477通过/2忽略；末次语音协议4通过/1在线忽略。1912中英词条一致，238源文件样式0违规，diff检查通过。
- 联合依赖图的最终中文显式资格测试通过，37.40秒，codex-speech-verified-live-zh/qualification.json：available=true，实际“这是本地语音转写测试,请只填入草稿,不要自动发送”。
- 实窗账户关联成功，未重新登录、未导入token/key，未移动Agent执行设备；保存LocalCodex、zh-CN、60秒成功，并从持久字段核对。测试使用静音，不录环境音；最终probe结果另记。

- 授权层还有瞬时matrix_authorization_unavailable。按ServerLogin现有有限证明语义，503只在原证明仍有效时允许重试，硬期限不变；401/撤销/身份改变仍立即拒绝。返回时核对当前有效证明及完整设备pin，不再强制一次额外网络续验误拒绝已获有效证明的结果。新增瞬时失败可完成但不能延长到期的回归。

- SDK恢复与Agent调用一致：周期检查调用sync_matrix，然后取得相同DeviceIdentity的有限证明。精确诊断还发现server_renew_deadline/sign_in_required，公开HTTP Date检测到服务端与本机小幅偏差。deadline接受最多5s时钟差，但本地有效期始终min(服务器所报剩余时间,30s)，过期或超35s的异常证明仍拒绝；新增32s返回值被截30s、短证明不延长、过期和60s异常拒绝的回归。未修改系统时间或放宽30s本地权限窗口。

## 最终验收通过（以前文失败为历史记录）

- 账户续期deadline的原31秒绝对上限对小幅服务端时钟领先过于严格；上限容差改为5秒且本地权限严格截30秒后，连续server_renew_deadline拒绝消失，项目列表正常恢复。
- 当前可见最终版本PID42432，codex-speech-clock-tolerance-build.log构建成功1m24s。重启后LocalCodex/zh-CN/60秒仍保留。实际设置页Test service成功显示“Codex accepted test audio without a voice error. Test dictation to verify recognition accuracy.”；codex-speech-clock-tolerance.{stdout,stderr}.log无Codex拒绝、proof rejected或panic。此项使用合成静音、未开麦克风。
- 实际有声识别证据为上述英/中合成语音资格报告；完整桌面依赖图的中文最后一次通过37.40s，实际“这是本地语音转写测试,请只填入草稿,不要自动发送”。不是把静音probe成功当作准确率成功。
- 最终引擎完整lib213通过、0失败、5显式在线等忽略（codex-speech-clock-tolerance-tests.log）；其中新增4项持续授权回归及1项时钟容差/30秒硬期限回归。桌面完整lib477通过/0失败/2忽略，Codex适配21通过/3忽略，Windows长路径1通过，中英1912词条一致/无缺失，样式238文件0违规，diff检查通过。
- hello聊天在所有启动前后均为空草稿、时间线没有新消息。本轮未录用户环境音、未操作登录或权限对话框、未提取/另存登录凭据、未使用API key、未改Agent执行设备。ASR可出现中文词间空白/分段，准确率不承诺逐字完美；其他OS及真实麦克风尚未实测。
- 所有构建/测试缓存、临时脚本、合成音频、日志均D:/BuildCache/hagency-client；无Docker、无schema手工变更，未提交/推送/发布。


## 实时流式听写改造（同日后续，覆盖前述停止后批处理行为）

用户要求边录边上传、边返回边更新草稿，停止只收尾，继续本地登录、禁止自动发送并测延迟。

- console::speech 定义20ms/640字节PCM16/16k单声道帧和Ready/Transcript回调，队列最多100帧（2秒）。开始听写即建立独立Codex+WebRTC；Ready后才打开麦克风/进入Listening，避免连接期间积压整段音频。设置页明确提示等待聆听标志再说话。
- Makepad回调边混音/重采样边try_send，每320样本发送；仅保留不足一帧尾部和累计计数。停止关闭采集并提交尾帧/丢弃sender，原有worker继续最后校正。队列满或连接断开即停止并报静态错误，不静默丢音或改用其他服务。HTTP录音服务继续原有停止后WAV行为。
- Peer::Encoder跨采集边界保留半个降采样对和不足RTP帧，PCMU帧按采集时钟发送；不再在停止后按原速重播整段录音。测试输入/生成静音使用绝对interval+Skip，避免Windows定时粒度和Delay让15.28秒样本被拉到19秒。
- 只处理相同threadId、role=user的delta/done；增量累积为当前段文本，每次即发Transcript(is_final=false)，最终完整段覆盖临时段并commit，随后新段单独累计。应用继续使用当前锁定robius-speech166991b的Dictation（该版本支持最终文本权威替换），不修改依赖缓存。
- NativeOwner持续有限证明续验/硬期限与完整设备pin保持，回调再验证共享选择没有更换，Service逐回调检查账户epoch。取消/房间切换/账户切换/后台终止连接和隔离子进程，迟到事件不再写入。回调没有Room发送能力，不请求turn/start或工具执行，助手文本/音频仍丢弃。
- 停止后继续发送生成静音直到用户final，最多30秒，final且没有后续user delta后600ms结束；结合最后有声输入时间，避免前一段final过早关闭而截断最后一段。应用Finishing backstop35秒。总连接硬限195秒，原录音10–120秒限制仍生效。
- 初轮（仅1秒尾部静音、然后无RTP）真实中文资格虽通过且停止前26次更新，但final延迟13.708秒；改为持续静音以驱动服务端结束检测。此初轮数字仅诊断，不代表最终收尾表现。v3是frameless协议，不直接套用普通Realtime API的input_audio_buffer.commit，也不调用可能交接coding turn的realtime/stop。
- 新增跨任意采集分块PCMU一致性、同thread用户增量/最终修正、采集停止前已发送/尾帧/背压、真实RoomInputBar临时识别→最终修正→下一段且没有发送触发的回归。初版UI测试错误地要求所有actions为空，而普通TextInput Changed本就合法；现精确检查发送按钮及Returned触发均未出现，不抑制正常输入更新。

最终测量/构建与验证结果另附下方。


### 收尾机制实证

持续静音没有解决v3最终确认延迟：第二轮首字1.839s，停止到complete15.571s，但最后文本在stop后0.475s已显示。检查OpenAI Codex上游 realtime_websocket/protocol.rs、methods.rs确认frameless v3原生支持session.close。不是普通Realtime的input_audio_buffer.commit，也不是Codex的thread/realtime/stop。

第三轮真实实验（codex-stream-close-zh/qualification.json）在停止后等250ms让末尾RTP到达，再通过同一WebRTC oai-events data channel发送session.close；不调用coding turn/工具/Room接口。成功得到最终user文本，首字1.643s，stop后0.342s最后文字到达，stop后1.616s连接收尾；停止前25次更新，总26个partial。冷连接9.572s，录音15.271s。这一轮仍用本地ChatGPT登录、中文SAPI合成音频，没有环境录音或API key。

正式路径采用上述session.close，并在发close后仅等user final/closed，不再往关闭中的peer写RTP以免媒体错误抢先于最终文本。probe仍持续生成静音且不close以测试通路。最终结果还要求final时间不早于最后有声输入，不能拿前一段最终文本冒充后续音频已识别。等待最终校正时草稿可正常编辑；总收尾backstop35s仍保留。


### 正式流式路径最终在线资格通过

以hagency+hagency-app完整桌面依赖图编译引擎资格测试，正式代码路径（没有测试close开关），输入按20ms真实时钟送入同一有界channel/Peer/Transcript回调：

| 样本 | 冷连接Ready | 开始上传→首字 | 停止前更新 | 最后partial距停止 | 停止→最终user文本 | 停止→完成 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 中文，两遍已知句子，中间1秒静音 | 7.459s | 2.018s | 26 | +0.325s | +0.941s | 1.580s |
| 英文，两遍已知句子，中间1秒静音 | 5.026s | 1.594s | 15 | 见原始events | +0.946s | 1.587s |

- 中文报告 D:/BuildCache/hagency-client/codex-stream-verified-zh/qualification.json，available=true，共27个partial，录音15.263s；实际“这是本地语音转写测试,请只填入草稿,不要自动发送。这是本地语音转写测试,请只填入草稿,不要自动发送”。显式在线资格1通过，24.55s。
- 英文报告 D:/BuildCache/hagency-client/codex-stream-verified-en/qualification.json，available=true，共16个partial，录音10.183s；实际“Hagency speech input test. Keep this in the draft.Hagency speech input test. Keep this in the draft.”。显式在线资格1通过，17.05s。中英sample重复用于证明第一句在第二句仍录制时已出字，不是停止后分块伪流式。
- 报告Ready时间包含生产Process的密封配置检查、account/read及WebRTC建连；不包含Hagency顶层SDK账户恢复时间。真实麦克风的硬件与声学延迟未测，以上均本机SAPI合成样本、当前网络，不能作固定时延承诺。
- 正式代码的引擎完整lib215通过/0失败/5忽略（stream-verified-engine-tests.log，18.12s），桌面完整lib479通过/0失败/2忽略（stream-verified-app-tests.log，22.01s）；新增4项有效回归。完整依赖图编译stream-verified-compile.log成功1m45s。中英1915词条键一致（静态非空tr/i18n_text无缺失），238文件样式0违规，diff检查通过。
- 未使用API key、提取登录token、开启实际麦克风或发送Room消息。没有提交/推送。GUI build与重启验收另记。


### 最终GUI交付

- stream-final-build.log的hagency-app/hagency.exe build成功1m26s，输出D:/BuildCache/hagency-client/target/debug/hagency.exe。旧PID42432正常退出，新PID69724可见启动，自动恢复账户（LoginSuccess）；未操作登录/权限UI。
- 旧hello未发送草稿在退出前于输入框选择并复制，新窗口输入框确认为空后粘贴原样恢复；未发送。语音设置实际显示Local Codex (experimental)/Simplified Chinese/60 seconds且新版先连接再边录边上传提示已生效，无需改变已保存配置。
- 最终服务probe结果另记；生成静音不开麦。实时有声时延证据来自上面的正式代码资格报告及真实生产草稿控件回归，不能把静音probe称为麦克风实测。


- 最终PID69724实窗Test service成功：“Codex accepted test audio without a voice error. Test dictation to verify recognition accuracy.”。使用生成静音/相同NativeOwner有限证明和本地登录；无账户拒绝/运行panic。ICE关闭有agent already closed的候选清理warn，未影响成功结果。结束返回hello聊天，原未发送草稿保持，时间线没有本轮新消息。

## 2026-10-20 分类提交与验收边界

本批既有改动按依赖顺序提交到本地 main：1b2f819c（共享 Codex 启动、Windows 路径与有限授权修复）、eb0b66fc（隔离的 Codex WebRTC 转写引擎及依赖）、01b803c2（客户端连接设置、实时采集及草稿更新，含 TLS provider 共存处理）；验证文档与开发记忆另成一笔。此前“未提交”表述保留为当时记录，以本节状态为准。本轮只做分类提交，不推送。

本次没有修改既有生产代码，也没有新增麦克风采集或在线调用；验证依据仍是上文的合成语音在线报告、引擎215项通过及最新包含侧栏的客户端487项通过。真实麦克风准确率、长录音/弱网实机联调、其他操作系统和发布包仍待验收；OHOS上层能力判断与底层实现条件不一致尚未修正。Codex独立语音模型选择、Claude直接语音和翻译不在已实现功能内。以上限制不因提交而视为完成。
