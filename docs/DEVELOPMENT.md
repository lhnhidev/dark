# dark — Development and verification

Project hiện tại: `D:\workspace\dark`; trong WSL là `/mnt/d/workspace/dark`. Luật chính ở [AGENTS.md](../AGENTS.md); vai trò, model, duyệt kế hoạch và pipeline ở [AGENT_WORKFLOW.md](AGENT_WORKFLOW.md).

## Môi trường

- Rust stable, edition 2021. Windows dùng `stable-x86_64-pc-windows-msvc` và bộ C++ của Visual Studio. Nếu PATH chưa nhận Rust, dùng `%USERPROFILE%\.cargo\bin\cargo.exe` và `rustc.exe` trực tiếp; xác nhận phiên bản trước khi code.
- Không tự cài Rust khi chưa được người dùng cho phép. `scripts/setup-wsl-rust.sh` chỉ chạy khi được duyệt cài trong WSL/Linux; dùng rustup chính thức dưới `~/.cargo` và `~/.rustup`, không sudo. Nếu thiếu `cc`, dừng và đưa người dùng lệnh `sudo apt update && sudo apt install build-essential` để họ tự chạy.
- Dependency mới phải có lý do trong kế hoạch đã duyệt và đi qua bài test RAM. Hiện không dùng tokio, regex, clap, serde. Editor trong dark chỉ ở mức sửa nhanh ([FEATURES.md §4.3](FEATURES.md#43-tab-2--edit)); không làm tính năng IDE, repo map, LSP (xem [FEATURES.md §6](FEATURES.md#6-ngoài-phạm-vi-cố-tình-không-làm)). API key chỉ đọc từ biến môi trường; không ghi vào cấu hình hay log.

## Windows

Ngoại trừ 1…7 đổi tab trực tiếp, các phím lệnh đều cần Alt; bộ kiểm terminal dùng Alt+q/Alt+Ctrl+C, thử phím cũ không chạy lệnh và kéo đường phân cách thật. `--self-test-review` cũng thử Alt+- / Alt+= / Alt+0 và đường xử lý kéo trên fixture RAM 30 file. Mouse capture phải tắt khi thoát/panic/nhường editor, Windows khôi phục console mode gốc.

```powershell
cd D:\workspace\dark
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/verify-windows.ps1
```

Runner kiểm tra format, Clippy, test debug/release, build debug/release, self-test, help/version, kích thước binary và console thật. Output mặc định: `.dark/verification/m0-review/after/windows`; có thể đổi bằng `-OutputDirectory PATH`. Output stdout/stderr được ghi riêng, giữ exit code thật của từng lệnh; bất kỳ bước không đạt làm runner trả lỗi.

Runner ép `CARGO_TARGET_DIR` riêng tại `target/verify-windows` và chỉ định target host từ `rustc -vV`; binary được kiểm tra nằm trong `target/verify-windows/<host>/{debug,release}`. Như vậy cấu hình target-dir bên ngoài không làm runner đo nhầm binary cũ. Target mặc định của project không bị thay đổi.

`tests/ram/windows-console.ps1` kiểm tra điều hướng, resize trước phím, Alt+q, Alt+Ctrl+C, panic và phục hồi input/output mode, con trỏ, màn hình. Tham số `-TargetDirectory PATH` chọn thư mục chứa `debug/dark.exe` và `release/dark.exe`, dùng cùng bài thử cho binary baseline và binary sau sửa.

## WSL/Linux: GNU phát triển, musl phát hành

Giữ nguyên target mặc định. GNU dùng `CARGO_TARGET_DIR="$HOME/dark-target"`; musl dùng `CARGO_TARGET_DIR="$HOME/dark-target-musl"` và cờ rõ ràng `--target x86_64-unknown-linux-musl`. Không trộn với `target/` Windows hoặc ghi đè bản GNU để thử musl. Release profile luôn giữ nguyên.

```bash
cd /mnt/d/workspace/dark
export PATH="$HOME/.cargo/bin:$PATH"
rustup target add x86_64-unknown-linux-musl
bash scripts/verify-linux.sh musl
```

Lệnh thêm target chỉ cần chạy khi được phép và chưa có target. Môi trường đã thử liên kết bằng `cc` sẵn có và target rustup, không cần musl-tools hoặc sudo. Nếu máy khác thiếu linker/công cụ cần sudo, dừng và đưa lệnh cho người dùng tự thực hiện.

Runner musl chạy đủ format, Clippy, test debug/release, build debug/release, self-test, help/version, kích thước binary và PTY thật; output mặc định ở `.dark/verification/m0-review/after/musl`. Có thể chỉ định thư mục output thứ hai. `bash scripts/verify-linux.sh gnu` dùng thư mục GNU riêng; test idle GNU có thể thất bại như kết quả lịch sử trong [M0_VERIFICATION.md](M0_VERIFICATION.md), không được đổi ngưỡng để báo đạt.

Đo so sánh trang resident, private và PSS:

```bash
python3 tests/ram/linux-memory.py "$HOME/dark-target/release/dark" "$HOME/dark-target-musl/x86_64-unknown-linux-musl/release/dark"
```

Script cần Python 3, chạy cùng thao tác trên PTY, chờ 1,2 giây rồi lấy năm mẫu idle trong mỗi lượt, lặp năm lượt mỗi chế độ thường/--debug. Exit 1 khi GNU vượt ngưỡng không có nghĩa musl thất bại; xem kết quả riêng từng biến thể. VmRSS vẫn là số quyết định ngân sách; private/PSS chỉ bổ sung. Private Linux không chỉ là heap và không tương đương chính xác private working set Windows.

## Thử trực tiếp terminal Linux

```bash
bash scripts/linux-interactive.sh musl
```

Nhấn 1…7 (hoặc Alt+1…7) để đổi màn hình, Alt+q hoặc Alt+Ctrl+C để thoát. Trong terminal WSL thứ hai, `python3 scripts/linux-terminal-resize.py 12 60` đổi kích thước PTY thật; kiểm tra redraw đến trước khi gửi thêm phím. Runner lưu PID/TTY ở `.dark/verification/manual`; helper xác nhận TTY đúng process trước khi đổi. GNU chọn bằng đối số `gnu`.

## Dữ liệu và ngân sách kiểm thử

- Unit test nằm trong `#[cfg(test)]`, integration test trong `tests/`, bài RAM dưới `tests/ram/`. Debug RAM test phải được đánh dấu bỏ qua với lý do chạy bản release.
- Windows đo private working set qua EX2; Linux đọc VmRSS. Đo không hỗ trợ phải báo n/a và lý do, không thay số đo khác dưới cùng tên.
- Ngân sách theo milestone ở [PLANNING.md](PLANNING.md) và [ARCHITECTURE.md §6](ARCHITECTURE.md#6-ngân-sách-ram). M0: idle **< 2.000.000 byte**, đỉnh app **< 10.000.000 byte**, binary **< 3.000.000 byte**. Đỉnh lấy mẫu không phải đỉnh bảo đảm của toàn bộ đời process.
- Snapshot binary trước sửa và log là output, được giữ ở `.dark/`; mọi script thực thi phải nằm trong `scripts/` hoặc `tests/` và được commit. Đường dẫn `.dark/*.sh` trong log cũ chỉ là lịch sử; dùng runner tracked ở trên để tái lập.
- `.gitattributes` giữ file shell/Python ở LF để checkout bằng Git trên Windows không làm script WSL bị lỗi CRLF.
- Unicode M0 dùng tab stop 8 cột cố định; cấu hình tab thuộc M1. Không đánh đổi chất lượng AI hoặc chức năng để giảm RAM. Benchmark lại allocator musl khi workload cấp phát nhiều ở các milestone sau.
- macOS hiện chỉ qua cargo check, chưa chạy thực tế. M0 đã đóng (tag `m0`). Không push hoặc tạo tag khi người dùng chưa yêu cầu.

## Self-test và test RAM

`dark --self-test` chạy không cần terminal: dùng cùng trạng thái app và renderer, đi qua các màn hình và resize rồi lấy nhiều mẫu bộ nhớ. Bài test trong `tests/ram/` chỉ áp ngân sách khi chạy `cargo test --release`; bản debug được bỏ qua kèm lý do. Self-test không thay thế kiểm tra terminal thật.

Từ M1, `dark --self-test-review` mở repo ở thư mục hiện tại, tạo snapshot, tải diff và vẽ ở 160/120/100/80 cột, rồi báo RAM của dark. Lệnh này tạo output/snapshot như phiên Review bình thường; chỉ chạy bài tự động trong repo fixture riêng. `tests/ram/review.rs` tạo repo 30 file/3.000 dòng đổi và áp đúng ngưỡng release, không thay bằng self-test khung trống.

Các cấu hình M1 dùng biến môi trường: `DARK_SNAPSHOT_KEEP` (số phiên giữ, mặc định 10), `DARK_TAB_WIDTH` (1–16, mặc định 8), `DARK_EDITOR` (ưu tiên hơn `VISUAL`/`EDITOR`, hỗ trợ `{file}`/`{line}`). Chưa đọc file TOML của các milestone sau. `NO_COLOR` tắt màu khi biến được đặt, kể cả giá trị rỗng.

Tự cập nhật M1 dùng native notifications, không filesystem polling. Trong WSL với `/mnt/d` hoặc `/mnt/c`, cần bản Windows mới: build `cargo build --release` trên Windows, rồi build musl trong WSL như trên. Bản Linux build từ repo Dark tự tìm `target/release/dark.exe` tại đường dẫn nguồn build; khi đặt binary ở nơi khác, đặt `dark.exe` cạnh `dark` hoặc dùng `DARK_WATCH_HELPER` (đường dẫn WSL tới exe). Không cần cầu nối khi project ở filesystem Linux. Cầu nối dùng chế độ nội bộ `--watch-bridge`, không phải agent.

Kiểm thử PTY tự cập nhật: `python3 scripts/test-watch-refresh.py "$HOME/dark-target-musl/x86_64-unknown-linux-musl/release/dark"`. Thêm thư mục cha fixture trên `/mnt/d` để thử interop; thêm `--windows-writer` để ghi từ Windows, `--kill-owner` để kiểm cầu nối thoát khi parent bị kill. Fixture luôn riêng, không sửa project thật. Script đo app bằng VmRSS và báo riêng proxy Linux; private working set Windows qua EX2 được báo riêng. `--self-test-review` cũng giữ watcher đang hoạt động khi đo ngân sách release. Bản build-only macOS chưa có native watch.

Đo riêng dependency word diff trên Windows: `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/measure-word-diff.ps1`. Probe cùng dữ liệu/flags trước và sau, không dùng số đó thay cho RAM ứng dụng thật; xem [M1_VERIFICATION.md](M1_VERIFICATION.md).

Ghi lại terminal Linux thật: `python3 scripts/capture-review.py /absolute/path/to/dark`. Fixture được tạo trong thư mục tạm; output `.ansi`, `.txt`, `.html` ở `docs/ui/m1/`. HTML tái dựng từ ANSI đã ghi, không phải ảnh chụp. Các script terminal Windows/Linux cũng dùng fixture riêng để tránh tạo snapshot trong repo làm việc.

## Benchmark so với công cụ review khác

`tests/bench/compare-review-tools.py` tự tạo repo mẫu 30 file có diff, mở từng công cụ trong PTY, gửi một phím, chờ idle rồi đo private, RSS, PSS của cả cây process. Chỉ chạy trên Linux (trên Windows dùng WSL); chỉ cần Python 3 và git.

```bash
python3 tests/bench/compare-review-tools.py \
    --dark "$HOME/dark-target-musl/x86_64-unknown-linux-musl/release/dark" \
    --tuicr "$(command -v tuicr)" \
    --hunk "$(command -v hunk)"
```

Chỉ đo công cụ được truyền đường dẫn. Có một lượt khởi động không tính điểm; lượt nào công cụ gần như không hiển thị gì bị loại. Trước khi công bố số, xem màn hình của từng công cụ một lần (`--keep-repo`) để chắc nó đã hiện diff, và kiểm tra lại dòng lệnh của tuicr/Hunk theo phiên bản mới nhất. Từ M1, dark phải tải đúng repo mẫu này thì phép so mới công bằng.

## Kiểm tra giao diện

Giao diện phải khớp [UI_SPEC.md](UI_SPEC.md) và ảnh trong [`docs/ui/`](ui/). Ảnh 1280×800 px tương đương terminal 160 cột × 44 dòng.

- Windows Terminal: đặt cửa sổ 160×44 (Settings → Startup → Launch size), hoặc chạy `mode con: cols=160 lines=44` trong conhost.
- WSL/Linux: kiểm tra bằng PTY cố định kích thước (như `tests/ram/linux-terminal.py`) để chụp output và so bố cục.
- Kiểm tra thêm ở 120, 100 và 80 cột theo quy tắc màn hẹp (UI_SPEC §3.4).
- Theme có unit test đúng từng mã hex; mọi khác biệt so với ảnh phải ghi trong báo cáo milestone.
