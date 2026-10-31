# dark — M0 verification

Project: `D:\workspace\dark`. Windows 11 Pro 10.0.26200; Rust/Cargo 1.99.1, stable MSVC. Windows đã đạt các kiểm tra dưới đây. Linux đã được build và chạy thử trong WSL Ubuntu 22.04 bằng Rust stable 1.99.1, nhưng RAM idle của bản GNU chưa đạt ổn định ngưỡng 2.000.000 byte. M0 chưa đóng; còn ngân sách RAM Linux và lượt kiểm tra Windows của người dùng. macOS mới qua cargo check, chưa chạy thực tế.

Release profile giữ nguyên. Binary Windows: **170.496 byte**. Lượt headless cuối: **552.960 byte** private working set; lượt test release: **557.056 byte**. Console thật release: idle lớn nhất **561.152 byte** qua các tình huống thường; đỉnh lấy mẫu lớn nhất **561.152 byte**. Không tính process kiểm tra và console host. Đỉnh lấy mẫu không phải bằng chứng về mọi đỉnh giữa các mẫu.

Code M0 không có worker thread hoặc async runtime; event loop chạy đồng bộ trên một luồng. Windows ghi nhận 4 thread hệ điều hành trong phiên thường; số này không đồng nghĩa có 4 event loop. Các tình huống panic không lấy mẫu idle (`idle=0`, `threads=0` trong harness nghĩa là không đo các mục đó, không phải RAM bằng 0).

Output thực tế được lưu dưới đây; chỉ bỏ lớp chẩn đoán `NativeCommandError` do PowerShell 5 bọc stderr, giữ nguyên nội dung Cargo. Bản ghi gốc ở `.dark/verification/m0/` (không đưa vào Git). Các lệnh cargo đã chạy bằng đường dẫn đầy đủ `%USERPROFILE%\.cargo\bin\cargo.exe` vì PATH của phiên cũ chưa nhận Rust.

## cargo fmt --check

```text
exit_code=0
```

## cargo clippy --all-targets -- -D warnings

```text
    Checking dark v0.0.1 (D:\workspace\dark)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.17s
exit_code=0
```

## cargo test

```text
   Compiling dark v0.0.1 (D:\workspace\dark)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.50s
     Running unittests src\main.rs (target\debug\deps\dark-8ebbfe36ddd2e310.exe)

running 8 tests
test app::tests::ignores_repeats_releases_and_modified_shortcuts ... ok
test app::tests::switches_screens_and_redraws_only_on_change ... ok
test app::tests::quit_keys_stop_without_redrawing ... ok
test ui::renderer::tests::unchanged_frame_writes_nothing_and_resize_redraws ... ok
test ui::renderer::tests::changing_screen_clears_old_lines_even_when_narrow ... ok
test ui::renderer::tests::failed_flush_forces_full_redraw ... ok
test memory::tests::parses_kernel_kib_as_bytes ... ok
test memory::tests::rejects_missing_units_malformed_values_and_overflow ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\cli.rs (target\debug\deps\cli-d0bde66f41e7a4cd.exe)

running 1 test
test rejects_unknown_argument_instead_of_opening_the_terminal ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests\ram.rs (target\debug\deps\ram-d53b691807e89ebd.exe)

running 1 test
test idle::release_binary_stays_within_memory_budget_after_navigation ... ignored, RAM budget measures release binary; run cargo test --release

test result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s

exit_code=0
```

## cargo test --release -- --nocapture

```text
   Compiling dark v0.0.1 (D:\workspace\dark)
    Finished `release` profile [optimized] target(s) in 4.67s
     Running unittests src\main.rs (target\release\deps\dark-4ce4ea5b8ad1d507.exe)

running 8 tests
test memory::tests::rejects_missing_units_malformed_values_and_overflow ... ok
test memory::tests::parses_kernel_kib_as_bytes ... ok
test app::tests::ignores_repeats_releases_and_modified_shortcuts ... ok
test ui::renderer::tests::changing_screen_clears_old_lines_even_when_narrow ... ok
test ui::renderer::tests::unchanged_frame_writes_nothing_and_resize_redraws ... ok
test ui::renderer::tests::failed_flush_forces_full_redraw ... ok
test app::tests::quit_keys_stop_without_redrawing ... ok
test app::tests::switches_screens_and_redraws_only_on_change ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\cli.rs (target\release\deps\cli-a6b4f839982046da.exe)

running 1 test
test rejects_unknown_argument_instead_of_opening_the_terminal ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests\ram.rs (target\release\deps\ram-1f36a94f48804c7c.exe)

running 1 test
memory_metric=private_working_set
idle_bytes=557056
peak_sampled_bytes=557056
screens=3

test idle::release_binary_stays_within_memory_budget_after_navigation ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.51s

exit_code=0
```

## cargo build --release

```text
    Finished `release` profile [optimized] target(s) in 0.01s
exit_code=0
```

## .\target\release\dark.exe --self-test

```text
memory_metric=private_working_set
idle_bytes=552960
peak_sampled_bytes=552960
screens=3
```

## (Get-Item .\target\release\dark.exe).Length

```text
binary_size_bytes=170496
```

## powershell -NoProfile -ExecutionPolicy Bypass -File tests/ram/windows-console.ps1

```text
debug q PASS idle_max_5_samples_private_ws_bytes=557056 peak_sampled_private_ws_bytes=557056 threads=4 input_mode_before=503 input_mode_running=504 input_mode_after=503 output_mode_before=3 output_mode_after=3 initial_cursor_visible=True exit=0
debug ctrlc PASS idle_max_5_samples_private_ws_bytes=552960 peak_sampled_private_ws_bytes=552960 threads=4 input_mode_before=503 input_mode_running=504 input_mode_after=503 output_mode_before=3 output_mode_after=3 initial_cursor_visible=True exit=0
debug panic PASS idle_max_5_samples_private_ws_bytes=0 peak_sampled_private_ws_bytes=548864 threads=0 input_mode_before=503 input_mode_running=504 input_mode_after=503 output_mode_before=3 output_mode_after=3 initial_cursor_visible=True exit=101
debug qhidden PASS idle_max_5_samples_private_ws_bytes=552960 peak_sampled_private_ws_bytes=552960 threads=4 input_mode_before=503 input_mode_running=504 input_mode_after=503 output_mode_before=3 output_mode_after=3 initial_cursor_visible=False exit=0
debug panichidden PASS idle_max_5_samples_private_ws_bytes=0 peak_sampled_private_ws_bytes=548864 threads=0 input_mode_before=503 input_mode_running=504 input_mode_after=503 output_mode_before=3 output_mode_after=3 initial_cursor_visible=False exit=101
release q PASS idle_max_5_samples_private_ws_bytes=548864 peak_sampled_private_ws_bytes=548864 threads=4 input_mode_before=503 input_mode_running=504 input_mode_after=503 output_mode_before=3 output_mode_after=3 initial_cursor_visible=True exit=0
release ctrlc PASS idle_max_5_samples_private_ws_bytes=561152 peak_sampled_private_ws_bytes=561152 threads=4 input_mode_before=503 input_mode_running=504 input_mode_after=503 output_mode_before=3 output_mode_after=3 initial_cursor_visible=True exit=0
release panic PASS idle_max_5_samples_private_ws_bytes=0 peak_sampled_private_ws_bytes=557056 threads=0 input_mode_before=503 input_mode_running=504 input_mode_after=503 output_mode_before=3 output_mode_after=3 initial_cursor_visible=True exit=-1073740791
release qhidden PASS idle_max_5_samples_private_ws_bytes=557056 peak_sampled_private_ws_bytes=557056 threads=4 input_mode_before=503 input_mode_running=504 input_mode_after=503 output_mode_before=3 output_mode_after=3 initial_cursor_visible=False exit=0
release panichidden PASS idle_max_5_samples_private_ws_bytes=0 peak_sampled_private_ws_bytes=557056 threads=0 input_mode_before=503 input_mode_running=504 input_mode_after=503 output_mode_before=3 output_mode_after=3 initial_cursor_visible=False exit=-1073740791
exit_code=0
```

## Dependency M0

| Crate trực tiếp | Lý do |
|---|---|
| `crossterm 0.29`, tắt default features, bật `events`/`windows` | Đọc sự kiện đồng bộ, resize và điều khiển terminal đa nền tảng. |
| `windows-sys 0.61`, chỉ Windows | Console mode/cursor và đo EX2 private working set; không có runtime nền. |
| `libc 0.2`, chỉ macOS | Binding `proc_pidinfo`/`proc_taskinfo` để đo resident memory. |

Các crate gián tiếp được khóa trong `Cargo.lock`. Không thêm tokio, ratatui, regex, clap, serde hoặc dependency kiểm thử. API bộ nhớ ba nền tảng cùng ở `src/memory/mod.rs` vì mỗi phần còn nhỏ, được chọn bằng `cfg`; không tạo ba file rỗng chỉ để khớp cây thư mục dự kiến.

## cargo build --release --target aarch64-apple-darwin

```text
   Compiling libc v0.2.191
   Compiling signal-hook v0.3.18
   Compiling parking_lot_core v0.9.12
   Compiling log v0.4.35
   Compiling smallvec v1.16.4
   Compiling cfg-if v1.0.5
   Compiling scopeguard v1.2.0
   Compiling rustix v1.1.6
   Compiling bitflags v2.13.2
   Compiling lock_api v0.4.14
   Compiling errno v0.3.14
   Compiling mio v1.2.4
   Compiling signal-hook-registry v1.4.8
   Compiling parking_lot v0.12.5
   Compiling signal-hook-mio v0.2.5
   Compiling crossterm v0.29.0
   Compiling dark v0.0.1 (D:\workspace\dark)
warning: invoking `"xcrun" "--sdk" "macosx" "--show-sdk-path"` to find MacOSX.sdk failed: program not found
  |
  = note: the SDK is needed by the linker to know where to find symbols in system libraries and for embedding the SDK version in the final object file
  = help: pass the path of an Xcode installation via the DEVELOPER_DIR environment variable, or an SDK with the SDKROOT environment variable

error: linker `cc` not found
  |
  = note: program not found

warning: `dark` (bin "dark") generated 1 warning
error: could not compile `dark` (bin "dark") due to 1 previous error; 1 warning emitted
exit_code=101
```

## Giới hạn và phần chưa làm

- Linux: đã được người dùng cho phép cài rustup chính thức vào `/home/lhnhidev/.cargo` và `/home/lhnhidev/.rustup`, không dùng sudo; GCC có sẵn. Tất cả artifact Linux ở `/home/lhnhidev/dark-target`, không dùng `target/` của Windows. Format, Clippy, test chức năng và build đạt; test RAM release GNU còn thất bại. Output đầy đủ bên dưới.
- macOS ARM64: đã cài target standard library và `cargo check --target aarch64-apple-darwin` đạt (exit 0). Build release thất bại ở bước link vì thiếu Apple SDK (`xcrun`) và linker `cc` trên Windows. Chưa build ra binary macOS và chưa chạy thực tế; không xem type-check là build hoặc chạy macOS thành công.
- Unix phục hồi chế độ raw ban đầu qua crossterm và hiện con trỏ khi thoát; không cam kết giữ lại trạng thái con trỏ vốn ẩn vì không có getter portable.
- Không bảo đảm phục hồi terminal sau khi hệ điều hành cưỡng chế kết thúc process; panic thông thường đã kiểm tra cả debug và release.
- M0 chỉ có ba màn hình trống. Chưa làm git, diff, AI, editor hoặc đọc pipe process con; chất lượng AI chưa thể đánh giá ở milestone này và không có cắt context.
- Reviewer đã kiểm tra lại hai bản sửa Windows (cursor ban đầu ẩn và nhận/vẽ lại resize), không thấy blocker mới trên Windows.

## Linux — WSL Ubuntu 22.04 (x86_64-unknown-linux-gnu)

Rust được cài bằng `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable --component rustfmt --component clippy --no-modify-path`, với `CARGO_HOME="$HOME/.cargo"`, `RUSTUP_HOME="$HOME/.rustup"`. Không dùng sudo; `/usr/bin/cc` và GCC 11.4.0 có sẵn. PATH chỉ được thêm cho phiên kiểm tra.

Mọi lệnh bên dưới chạy tại `/mnt/d/workspace/dark` với `PATH="$HOME/.cargo/bin:$PATH"` và `CARGO_TARGET_DIR="$HOME/dark-target"`. Không thay đổi release profile, dependency, metric hoặc ngưỡng RAM. Đã thử giảm bộ đệm đọc `/proc`, nhưng không đạt ổn định nên hoàn nguyên toàn bộ thay đổi thử nghiệm; mã ứng dụng vẫn đúng bản Windows đã kiểm chứng.

**Kết quả GNU chưa đạt M0:** lần cuối `cargo test --release` thất bại vì idle **2.031.616 byte**; self-test riêng **2.043.904 byte**. Binary **395.800 byte**. RSS thay đổi theo lần chạy, vì vậy một lần dưới ngưỡng không đủ để báo đạt. Test PTY cũng còn thất bại về RAM ở các phiên thường; vẫn hoàn tất kiểm tra terminal trước khi đánh giá ngân sách. Ngưỡng đỉnh lấy mẫu được kiểm tra cả trong tình huống panic release; idle panic là n/a. Đề xuất thử target musl đang chờ người dùng duyệt; chưa cài target hoặc đổi cách liên kết.

### Linux: environment

```text
$ bash -c id -un; uname -srm; echo "CARGO_TARGET_DIR=$CARGO_TARGET_DIR"; command -v cargo; cargo --version; rustc --version; command -v cc; cc --version
lhnhidev
Linux 6.6.114.1-microsoft-standard-WSL2 x86_64
CARGO_TARGET_DIR=/home/lhnhidev/dark-target
/home/lhnhidev/.cargo/bin/cargo
cargo 1.99.1 (5f94df478 2026-08-27)
rustc 1.99.1 (58cf8f9c3 2026-10-13)
/usr/bin/cc
cc (Ubuntu 11.4.0-1ubuntu1~22.04.3) 11.4.0
Copyright (C) 2021 Free Software Foundation, Inc.
This is free software; see the source for copying conditions.  There is NO
warranty; not even for MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.

exit_code=0
```

### Linux: fmt

```text
$ cargo fmt --check
exit_code=0
```

### Linux: clippy

```text
$ cargo clippy --all-targets -- -D warnings
    Checking dark v0.0.1 (/mnt/d/workspace/dark)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.66s
exit_code=0
```

### Linux: test

```text
$ cargo test
   Compiling dark v0.0.1 (/mnt/d/workspace/dark)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.85s
     Running unittests src/main.rs (/home/lhnhidev/dark-target/debug/deps/dark-2c8a88a0e2828995)

running 8 tests
test app::tests::ignores_repeats_releases_and_modified_shortcuts ... ok
test app::tests::quit_keys_stop_without_redrawing ... ok
test memory::tests::parses_kernel_kib_as_bytes ... ok
test app::tests::switches_screens_and_redraws_only_on_change ... ok
test memory::tests::rejects_missing_units_malformed_values_and_overflow ... ok
test ui::renderer::tests::changing_screen_clears_old_lines_even_when_narrow ... ok
test ui::renderer::tests::failed_flush_forces_full_redraw ... ok
test ui::renderer::tests::unchanged_frame_writes_nothing_and_resize_redraws ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/cli.rs (/home/lhnhidev/dark-target/debug/deps/cli-d0343c925aad2ce4)

running 1 test
test rejects_unknown_argument_instead_of_opening_the_terminal ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/ram.rs (/home/lhnhidev/dark-target/debug/deps/ram-038e33324d1b88e3)

running 1 test
test idle::release_binary_stays_within_memory_budget_after_navigation ... ignored, RAM budget measures release binary; run cargo test --release

test result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s

exit_code=0
```

### Linux: test-release

```text
$ cargo test --release -- --nocapture
   Compiling dark v0.0.1 (/mnt/d/workspace/dark)
    Finished `release` profile [optimized] target(s) in 6.87s
     Running unittests src/main.rs (/home/lhnhidev/dark-target/release/deps/dark-0181510a9f0356e7)

running 8 tests
test app::tests::ignores_repeats_releases_and_modified_shortcuts ... ok
test app::tests::quit_keys_stop_without_redrawing ... ok
test app::tests::switches_screens_and_redraws_only_on_change ... ok
test memory::tests::parses_kernel_kib_as_bytes ... ok
test memory::tests::rejects_missing_units_malformed_values_and_overflow ... ok
test ui::renderer::tests::changing_screen_clears_old_lines_even_when_narrow ... ok
test ui::renderer::tests::failed_flush_forces_full_redraw ... ok
test ui::renderer::tests::unchanged_frame_writes_nothing_and_resize_redraws ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/cli.rs (/home/lhnhidev/dark-target/release/deps/cli-fc41b1ee2d1baed0)

running 1 test
test rejects_unknown_argument_instead_of_opening_the_terminal ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/ram.rs (/home/lhnhidev/dark-target/release/deps/ram-8c492cf16f6c01e7)

running 1 test

thread 'idle::release_binary_stays_within_memory_budget_after_navigation' (2998) panicked at tests/ram/idle.rs:36:5:
idle RAM exceeded 2 MB: memory_metric=rss
idle_bytes=2031616
peak_sampled_bytes=2031616
screens=3

note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test idle::release_binary_stays_within_memory_budget_after_navigation ... FAILED

failures:

failures:
    idle::release_binary_stays_within_memory_budget_after_navigation

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.51s

error: test failed, to rerun pass `--test ram`
exit_code=101
```

### Linux: build-release

```text
$ cargo build --release
    Finished `release` profile [optimized] target(s) in 0.07s
exit_code=0
```

### Linux: self-test

```text
$ /home/lhnhidev/dark-target/release/dark --self-test
memory_metric=rss
idle_bytes=2043904
peak_sampled_bytes=2043904
screens=3
exit_code=0
```

### Linux: binary-size

```text
$ stat --format=%s /home/lhnhidev/dark-target/release/dark
395800
exit_code=0
```

### Linux: terminal-final

```text
$ python3 tests/ram/linux-terminal.py "$CARGO_TARGET_DIR"
debug q PASS idle_max_5_samples_rss_bytes=2478080 peak_sampled_rss_bytes=2478080 exit=0
debug ctrlc PASS idle_max_5_samples_rss_bytes=2510848 peak_sampled_rss_bytes=2510848 exit=0
debug panic PASS idle_max_5_samples_rss_bytes=n/a peak_sampled_rss_bytes=2482176 exit=101
release q FAIL: RAM budget exceeded: idle=[2023424, 2023424, 2023424, 2023424, 2023424] peak_sampled=2023424
release ctrlc FAIL: RAM budget exceeded: idle=[2031616, 2031616, 2031616, 2031616, 2031616] peak_sampled=2031616
release panic PASS idle_max_5_samples_rss_bytes=n/a peak_sampled_rss_bytes=2002944 exit=-6
exit_code=1
```

### Thử trực tiếp TUI trong terminal WSL

Agent chính chạy release binary bằng phiên terminal tương tác: chuyển `2` → Edit, `3` → Models, `1` → Review; thay kích thước PTY thật từ 80x24 xuống 60x12 bằng TIOCSWINSZ/SIGWINCH và xác nhận frame mới đến trước phím tiếp theo; `q` thoát với exit 0. Phiên thứ hai thoát bằng Ctrl-C, exit 0. Đã kiểm tra đường dẫn `/proc/<pid>/exe` của phiên cuối không phải binary bị thay thế. Thanh --debug ở hai phiên ổn định tại **2.052.096 byte**, vẫn vượt giới hạn M0.

Dưới đây là bản ghi terminal thật dạng JSON escaped để giữ nguyên các mã điều khiển mà không làm hỏng trình xem Markdown. Phần đầu/cuối có mã điều khiển của terminal Windows/WSL host. Đây không phải output headless.

#### manual-terminal-q (exit 0)

```json
"\u001b[?9001h\u001b[?1004h\u001b[?25l\u001b[2J\u001b[m\u001b[H/dev/pts/0\r\n\u001b]0;C:\\WINDOWS\\System32\\WindowsPowerShell\\v1.0\\powershell.exe\u0007\u001b[?25h\u001b[?1049h\u001b[?25l\u001b[H[1 Review]  2 Edit  3 Models  q Quit\u001b[K\r\n\u001b[K\r\nReview - Coming soon\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\nMemory (rss): 1998848 bytes\u001b[K\u001b[53C\rMemory (rss): 2048000 bytes\u001b[K\u001b[53C\rMemory (rss): 2052096 bytes\u001b[K\u001b[53C\u001b[H1 Review  [2 Edit]  3 Models  q Quit\u001b[K\u001b[3;1HEdit - Coming soon\u001b[K\u001b[62C\u001b[H1 Review  2 Edit  [3 Models]  q Quit\u001b[K\u001b[3;1HModels - Coming soon\u001b[K\u001b[60C\u001b[H[1 Review]  2 Edit  3 Models  q Quit\u001b[K\u001b[3;1HReview - Coming soon\u001b[K\u001b[60C\u001b[H[1 Review]  2 Edit  3 Models  q Quit\u001b[K\r\n\u001b[K\r\nReview - Coming soon\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\nMemory (rss): 2052096 bytes\u001b[K\u001b[53C\u001b[12;28H\u001b[?25h\u001b[?1049l\u001b[?25l\u001b[H/dev/pts/0\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\u001b[2;1H\u001b[?25h\u001b[?9001l\u001b[?1004l"
```

#### manual-terminal-ctrlc (exit 0)

```json
"\u001b[?9001h\u001b[?1004h\u001b[?25l\u001b[2J\u001b[m\u001b[H/dev/pts/0\r\n\u001b]0;C:\\WINDOWS\\System32\\WindowsPowerShell\\v1.0\\powershell.exe\u0007\u001b[?25h\u001b[?1049h\u001b[?25l\u001b[H\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\u001b[80C\u001b[H[1 Review]  2 Edit  3 Models  q Quit\u001b[K\r\n\u001b[K\r\nReview - Coming soon\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\nMemory (rss): 2002944 bytes\u001b[K\u001b[53C\rMemory (rss): 2048000 bytes\u001b[K\u001b[53C\rMemory (rss): 2052096 bytes\u001b[K\u001b[53C\u001b[24;28H\u001b[?25h\u001b[?1049l\u001b[?25l\u001b[H/dev/pts/0\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\r\n\u001b[K\u001b[2;1H\u001b[?25h\u001b[?9001l\u001b[?1004l"
```
