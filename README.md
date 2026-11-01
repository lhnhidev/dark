# dark

**Review từng thay đổi agent AI tạo ra: giữ phần đúng, bỏ phần sai, sửa ngay, góp ý ngược lại cho agent. Dùng agent bạn đang có, chỉ tốn vài MB RAM.**

> **Trạng thái:** đang phát triển. M0 (khung ứng dụng) đã xong; M1 (khung giao diện + Review) là bước tiếp theo. Xem [PLANNING.md](docs/PLANNING.md).
> Tên "dark" là tên tạm, sẽ chốt trước v0.2.

![Tab Review](docs/ui/1-review.png)

## Vì sao có dark

Agent vừa sửa 14 file. Bạn có thật sự đọc hết không?

Sau một lượt vibe, người dùng thường chỉ có hai lựa chọn: đọc hết trong một IDE nặng, hoặc chấp nhận hết và hy vọng không có lỗi. Muốn bảo agent sửa một chỗ thì phải copy tên file, số dòng, mô tả lại từ đầu.

dark làm đúng một vòng lặp:

**vibe với agent → review từng hunk → giữ / bỏ / sửa ngay → góp ý ngược lại cho agent → commit hoặc hoàn tác**

## Giao diện: 7 tab

| Tab | Làm gì |
|---|---|
| **1 Review** | Bên trái: danh sách file đổi kiểu git panel của Zed (stage, commit). Bên phải (80%): diff 2 cột, mỗi hunk có ghi chú ngắn của agent; `a` accept, `x` reject, `Enter` sửa, `?` hỏi agent |
| **2 Edit** | Cây thư mục và editor gọn: sửa, lưu, hoàn tác, copy/paste, tìm |
| **3 Vibe** | Chọn agent: Claude Code, Codex, Gemini CLI, opencode đã cài trên máy, hoặc model qua API key. Bản đầu chạy 1 agent, về sau tối đa 6 agent cùng lúc |
| **4 Git commit** | Lịch sử commit, cây nhánh, xem diff từng commit |
| **5 Models** | Thêm model bằng API key (lưu trong kho mật khẩu của hệ điều hành) |
| **6 View, 7 Help** | Sắp có |

Màu theo theme của [Hunk](https://github.com/modem-dev/hunk) (GitHub Dark Default). Thiết kế chi tiết và ảnh mẫu: [UI_SPEC.md](docs/UI_SPEC.md), [`docs/ui/`](docs/ui/).

## Nhẹ thật sự

| | Số đo |
|---|---|
| RAM idle của bản hiện tại (M0) | ~0,56 MB trên Windows, ~0,52 MB trên Linux musl |
| File chạy | ~0,2 MB (Windows), ~0,5 MB (Linux musl), không cần Node hay Python |
| Mục tiêu | Review < 2 MB; sửa file lớn < 4 MB; `darkw` tính cả cửa sổ < 3 MB; giới hạn cứng 10 MB |

So sánh sơ bộ trên cùng một diff 30 file (Linux, bộ nhớ riêng của process):

| dark M0 | tuicr | Hunk |
|---|---|---|
| ~0,55 MB | ~14,6 MB | ~191 MB |

dark M0 chưa tải diff nên phép so này chưa công bằng cho đối thủ; sẽ đo lại sau M1 bằng [script công khai](tests/bench/compare-review-tools.py). Agent vẫn tốn RAM như khi bạn chạy chúng riêng; dark chỉ bật agent khi bạn vibe và tắt khi rảnh. Chi tiết: [ARCHITECTURE.md §6](docs/ARCHITECTURE.md#6-ngân-sách-ram).

## Lộ trình

| Giai đoạn | Phiên bản | Nội dung |
|---|---|---|
| GĐ1 | v0.1 | Review, Edit, Git commit, gọi agent CLI đã cài |
| GĐ2 | v0.2 | Agent chạy ngay trong dark (ACP), ghi chú AI cho từng hunk, Models với API key, cửa sổ riêng `darkw` trên Windows |
| GĐ3 | v1.x | Tối đa 6 agent cùng lúc, tiết kiệm token, agent riêng của dark |

Chi tiết: [PLANNING.md](docs/PLANNING.md).

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
| [UI_SPEC.md](docs/UI_SPEC.md) | **Chuẩn giao diện:** màu, bố cục từng tab, phím, quy tắc màn hẹp; ảnh mẫu trong [`docs/ui/`](docs/ui/) |
| [FEATURES.md](docs/FEATURES.md) | Bảng tính năng theo tab, skill và tiết kiệm token, ngoài phạm vi |
| [PLANNING.md](docs/PLANNING.md) | Ba giai đoạn, M1–M9, tiêu chí hoàn thành, quy trình, điều kiện phát hành |
| [ARCHITECTURE.md](docs/ARCHITECTURE.md) | Thành phần, ACP, git, editor, kho mật khẩu, ngân sách RAM, rủi ro |
| [SEQUENCE.md](docs/SEQUENCE.md) | Sơ đồ tuần tự các luồng chính |
| [DEVELOPMENT.md](docs/DEVELOPMENT.md) | Môi trường, lệnh kiểm tra, runner, đo RAM, benchmark |
| [M0_VERIFICATION.md](docs/M0_VERIFICATION.md) | Output kiểm chứng M0 |
| [AGENT_WORKFLOW.md](docs/AGENT_WORKFLOW.md) | Vai trò, model và quy trình của agent phát triển |

## Giấy phép

MIT hoặc Apache-2.0 (dự kiến). Bảng màu lấy từ theme mặc định của Hunk (Copyright (c) Modem Labs Inc., MIT).
