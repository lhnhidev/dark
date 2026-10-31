# dark

**Trạm review và vibe code trong terminal, dưới 10 MB RAM.**

dark giúp vibe coder xem AI đã sửa gì, giữ hoặc bỏ từng thay đổi, hỏi AI khi không hiểu, và giao task cho agent, với nhiều model AI tùy chọn. Mục tiêu là chất lượng gọi model ngang hoặc tốt hơn opencode, trong khi process của app chỉ dùng vài MB RAM.

> **Trạng thái:** pre-MVP, đang kiểm chứng M0. Hiện có khung terminal với ba màn hình trống; git, diff, AI và editor chưa được triển khai. Xem [PLANNING.md](docs/PLANNING.md).

## Chạy khung M0

Project trên máy phát triển: `D:\workspace\dark`. Cần Rust stable; Windows dùng toolchain MSVC và Visual Studio C++ Build Tools.

```powershell
cargo build --release
.\target\release\dark.exe
.\target\release\dark.exe --debug
```

Linux/macOS chạy `./target/release/dark`. Phím `1`/`2`/`3` chuyển Review/Edit/Models; `q` hoặc `Ctrl-C` thoát. `--debug` hiện bộ nhớ của chính ứng dụng, không tính terminal hoặc tiến trình con. Windows đo private working set; Linux/macOS đo RSS. Nếu API không hỗ trợ, thanh trạng thái hiện `n/a` kèm lý do.

```text
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo test --release -- --nocapture
```

Test RAM được bỏ qua có thông báo ở bản debug; bản release kiểm tra idle dưới 2.000.000 byte và đỉnh lấy mẫu dưới 10.000.000 byte. `dark --self-test` chạy không cần terminal, dùng cùng trạng thái app và renderer để kiểm tra điều hướng/resize rồi đo bộ nhớ. Nó không thay thế kiểm tra terminal thật.

Kiểm tra console thật trên Windows, gồm đổi màn hình, resize, thoát và panic ở cả debug/release:

```powershell
cargo build
cargo build --release
powershell -NoProfile -ExecutionPolicy Bypass -File tests/ram/windows-console.ps1
```

Trong WSL/Linux, dùng thư mục build riêng để không lẫn với `target/` của Windows:

```bash
cd /mnt/d/workspace/dark
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_TARGET_DIR="$HOME/dark-target"
cargo build
cargo build --release
python3 tests/ram/linux-terminal.py "$CARGO_TARGET_DIR"
```

Script Linux cần Python 3 và PTY, kiểm tra phím/resize/thoát/panic trên terminal thật, đo RSS qua `/proc` và phục hồi thuộc tính terminal. Các cờ đo chỉ áp dụng ngân sách cho bản release. Trên Linux ngoài WSL, thay đường dẫn project cho đúng nơi checkout.

## Vì sao có dark

Sau một phiên vibe code, việc khó nhất thường là **biết AI đã đổi những gì và có nên giữ không**. Các IDE và harness hiện có thường nặng (hàng trăm MB tới vài GB RAM) và nhiều tính năng mà vibe coder hiếm khi dùng. dark tập trung vào đúng vòng lặp:

**AI sửa → xem diff → giữ / bỏ / hỏi AI → chạy test → commit**

## Tính năng chính (MVP dự kiến)

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

## Phím tắt (MVP dự kiến)

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

## Cấu hình (dự kiến, chưa dùng ở M0)

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
| [M0_VERIFICATION.md](docs/M0_VERIFICATION.md) | Output kiểm tra M0, kích thước binary, bộ nhớ và nền tảng chưa kiểm chứng |
| [AGENT_WORKFLOW.md](docs/AGENT_WORKFLOW.md) | Vai trò, model và quy trình làm việc của agent |

## Giấy phép

MIT hoặc Apache-2.0 (dự kiến).
