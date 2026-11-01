# dark

**Review từng thay đổi agent AI tạo ra: giữ phần đúng, bỏ phần sai, góp ý ngược lại cho agent. Dùng agent bạn đang có, chỉ tốn vài MB RAM.**

> **Trạng thái:** đang phát triển. M0 (khung ứng dụng) đã xong; M1 (review thật) là bước tiếp theo. Xem [PLANNING.md](docs/PLANNING.md).
> Tên "dark" là tên tạm, sẽ chốt trước khi ra mắt.

## Vì sao có dark

Agent vừa sửa 14 file. Bạn có thật sự đọc hết không?

Sau một lượt vibe, người dùng thường chỉ có hai lựa chọn: đọc hết trong một IDE nặng, hoặc chấp nhận hết và hy vọng không có lỗi. Muốn bảo agent sửa một chỗ thì phải copy tên file, số dòng, mô tả lại từ đầu.

dark làm đúng một vòng lặp:

**vibe với agent → review từng hunk → giữ / bỏ → sửa nhanh hoặc ghi chú gửi lại agent → hoàn tác khi cần**

## Tính năng (dự kiến)

- **Review theo hunk:** mọi thay đổi kể từ trước lượt AI. `a` giữ, `r` bỏ, `u` hoàn tác cả lượt.
- **Ghi chú gửi thẳng cho agent:** `/` ghi chú ngay trên dòng code, agent biết chính xác chỗ cần sửa.
- **Vibe ngay trong dark:** điều khiển **Claude Code, Codex, Gemini CLI, opencode** qua Agent Client Protocol (ACP). Agent chạy đúng harness chính chủ nên chất lượng không đổi; dark chỉ thay phần giao diện.
- **Hoặc dùng kèm:** chạy agent ở cửa sổ riêng như bình thường, dark tự phát hiện thay đổi để review.
- **Cửa sổ riêng trên Windows:** `darkw` mở thẳng cửa sổ, không cần terminal.
- **Sửa code ngay trong dark:** `Enter` tại một hunk để sửa: gõ, lưu, hoàn tác, chọn chữ, copy/paste, tìm trong file. Lưu xong diff tự cập nhật.
- **Hoặc mở editor của bạn:** `e` mở đúng file và dòng trong VS Code, Neovim... cho những lần sửa lớn.
- **An toàn:** snapshot trước mỗi lượt AI; agent sửa file hay chạy lệnh đều phải được duyệt.

Editor trong dark cố tình gọn (kiểu nano/micro): không tự hoàn thành, không repo map hay LSP. Danh sách đầy đủ: [FEATURES.md](docs/FEATURES.md).

## Nhẹ thật sự

| | Số đo |
|---|---|
| RAM idle của bản hiện tại (M0) | ~0,56 MB trên Windows, ~0,52 MB trên Linux musl |
| File chạy | ~0,2 MB (Windows), ~0,5 MB (Linux musl), không cần Node hay Python |
| Mục tiêu khi review | dưới 2 MB; `darkw` tính cả cửa sổ dưới 3 MB |

So sánh sơ bộ trên cùng một diff 30 file (Linux, bộ nhớ riêng của process):

| dark M0 | tuicr | Hunk |
|---|---|---|
| ~0,55 MB | ~14,6 MB | ~191 MB |

dark M0 chưa tải diff nên phép so này chưa công bằng cho đối thủ; sẽ đo lại sau M1 bằng [script công khai](tests/bench/compare-review-tools.py). Agent vẫn tốn RAM như khi bạn chạy chúng riêng; dark chỉ bật agent khi bạn vibe và tắt khi bạn quay lại review. Chi tiết: [ARCHITECTURE.md §6](docs/ARCHITECTURE.md#6-ngân-sách-ram).

## Phím tắt (dự kiến)

| Phím | Tác dụng |
|---|---|
| `a` / `r` | Giữ / bỏ hunk |
| `n` / `N` | Hunk kế / trước |
| `u` | Hoàn tác cả lượt AI |
| `/` | Ghi chú |
| `?` | Hỏi agent về hunk |
| `Enter` | Sửa code ngay trong dark (`Ctrl-S` lưu, `Esc` quay lại) |
| `e` | Mở editor ngoài |
| `!` | Chạy test |
| `c` | Commit |
| `v` | Vibe |
| `m` | Chọn agent |
| `q` | Thoát |

## Chạy bản hiện tại

Bản M0 chỉ có khung: ba màn trống, chuyển bằng `1`/`2`/`3`, thoát bằng `q` hoặc `Ctrl-C`.

```powershell
cargo build --release
.\target\release\dark.exe            # Linux/macOS: ./target/release/dark
.\target\release\dark.exe --debug    # hiện RAM của chính dark
.\target\release\dark.exe --help
```

Cần Rust stable; Windows dùng toolchain MSVC và bộ C++ của Visual Studio. Lệnh kiểm tra, runner Windows/WSL, bản musl và cách đo RAM: [DEVELOPMENT.md](docs/DEVELOPMENT.md).

## Tài liệu

| Tài liệu | Nội dung |
|---|---|
| [ARCHITECTURE.md](docs/ARCHITECTURE.md) | Định vị, thành phần, ACP, `darkw`, ngân sách RAM, điều khoản gói thuê bao, rủi ro |
| [FEATURES.md](docs/FEATURES.md) | Use case, bảng tính năng, phím tắt, ngoài phạm vi |
| [SEQUENCE.md](docs/SEQUENCE.md) | Sơ đồ tuần tự các luồng chính |
| [PLANNING.md](docs/PLANNING.md) | Lộ trình M1–M5, tiêu chí hoàn thành, quyết định còn mở |
| [DEVELOPMENT.md](docs/DEVELOPMENT.md) | Môi trường, lệnh kiểm tra, runner, đo RAM, benchmark |
| [M0_VERIFICATION.md](docs/M0_VERIFICATION.md) | Output kiểm chứng M0 |
| [AGENT_WORKFLOW.md](docs/AGENT_WORKFLOW.md) | Vai trò, model và quy trình của agent phát triển |

## Giấy phép

MIT hoặc Apache-2.0 (dự kiến).
