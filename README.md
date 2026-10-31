# dark

**Trạm review và vibe code trong terminal, dưới 10 MB RAM.**

dark giúp vibe coder xem AI đã sửa gì, giữ hoặc bỏ từng thay đổi, hỏi AI khi không hiểu, và giao task cho agent, với nhiều model AI tùy chọn. Mục tiêu là chất lượng gọi model ngang hoặc tốt hơn opencode, trong khi process của app chỉ dùng vài MB RAM.

> **Trạng thái:** pre-MVP. Hiện mới có tài liệu thiết kế và khung project. Xem [PLANNING.md](docs/PLANNING.md).

## Vì sao có dark

Sau một phiên vibe code, việc khó nhất thường là **biết AI đã đổi những gì và có nên giữ không**. Các IDE và harness hiện có thường nặng (hàng trăm MB tới vài GB RAM) và nhiều tính năng mà vibe coder hiếm khi dùng. dark tập trung vào đúng vòng lặp:

**AI sửa → xem diff → giữ / bỏ / hỏi AI → chạy test → commit**

## Tính năng chính (MVP)

- **Review theo hunk:** danh sách file AI đã sửa kể từ snapshot, diff từng hunk, `a` giữ, `r` bỏ.
- **AI nhận xét ngay dưới dòng:** chỉ ra lỗi, đề xuất sửa; áp dụng patch chỉ sau khi bạn duyệt.
- **Nhiều model:** Anthropic, Gemini, và mọi API OpenAI-compatible (OpenAI, OpenRouter, DeepSeek, Ollama, LM Studio...).
- **Agent đầy đủ:** đọc, tìm, sửa file, chạy lệnh; tự chạy lint/typecheck/test sau mỗi lần sửa.
- **Ngữ cảnh tốt:** repo map, đọc `AGENTS.md`/`CLAUDE.md`, tóm tắt hội thoại dài, prompt caching.
- **Editor gọn:** sửa, lưu, undo, tìm file, chạy test.
- **An toàn:** checkpoint git trước mỗi lượt AI, hoàn tác bất cứ lúc nào; API key chỉ đọc từ biến môi trường.

Danh sách đầy đủ: [FEATURES.md](docs/FEATURES.md).

## Vì sao nhẹ

- Viết bằng **Rust**, chạy trong terminal (TUI), một luồng duy nhất.
- Việc nặng chạy ở **process con**: HTTPS qua `curl`, `git`, `ripgrep`, test runner, LSP (tùy chọn).
- Tính năng chưa dùng thì chưa nạp; dữ liệu lớn (lịch sử chat, repo map) nằm trên đĩa.
- **Không đánh đổi chất lượng AI lấy RAM:** model luôn nhận đầy đủ context.

| Tình huống | RSS mục tiêu |
|---|---|
| Đang review | 1,5–2,5 MB |
| Đang stream câu trả lời AI | 2–3 MB |
| Agent đang làm việc | 4–8 MB |

Chi tiết: [ARCHITECTURE.md](docs/ARCHITECTURE.md#6-ngân-sách-ram).

## Phím tắt

| Phím | Tác dụng |
|---|---|
| `a` / `r` | Giữ / bỏ hunk |
| `n` / `N` | Hunk kế / trước |
| `?` | Hỏi AI |
| `p` | Áp dụng patch AI |
| `ctrl-p` | Tìm file |
| `ctrl-s` | Lưu |
| `!` | Chạy test |
| `m` | Chọn model |
| `c` | Commit |

## Cài đặt (dự kiến)

Yêu cầu: Rust (stable), `git`, `curl` (có sẵn trên Windows 10+, macOS, hầu hết Linux). Khuyến nghị cài thêm `ripgrep`.

```bash
git clone <repo>/dark
cd dark
cargo build --release
```

## Cấu hình

Tạo `~/.config/dark/providers.toml` (Windows: `%APPDATA%\dark\providers.toml`):

```toml
default = "anthropic/claude-sonnet"

[providers.anthropic]
kind = "anthropic"
key_env = "ANTHROPIC_API_KEY"

[providers.openrouter]
kind = "openai-compatible"
base_url = "https://openrouter.ai/api/v1"
key_env = "OPENROUTER_API_KEY"

[providers.local]
kind = "openai-compatible"
base_url = "http://localhost:11434/v1"
```

Đặt API key trong biến môi trường, ví dụ (PowerShell):

```powershell
$env:ANTHROPIC_API_KEY = "sk-..."
```

## Sử dụng (dự kiến)

```bash
cd my-project
dark            # mở màn Review so với snapshot gần nhất
dark "thêm đăng nhập bằng Google"   # giao task cho agent, xong thì mở Review
dark --debug    # hiện RSS, token, chi phí
```

## Tài liệu

| Tài liệu | Nội dung |
|---|---|
| [ARCHITECTURE.md](docs/ARCHITECTURE.md) | Bối cảnh, quyết định công nghệ, thành phần, ngân sách RAM, rủi ro |
| [FEATURES.md](docs/FEATURES.md) | Sơ đồ use case, bảng tính năng MVP, phím tắt, ngoài phạm vi |
| [SEQUENCE.md](docs/SEQUENCE.md) | Sơ đồ tuần tự các luồng chính |
| [PLANNING.md](docs/PLANNING.md) | Milestone, tiêu chí hoàn thành, cách đo chất lượng, quyết định còn mở |

## Giấy phép

MIT hoặc Apache-2.0 (dự kiến).
