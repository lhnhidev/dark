# dark — M0 verification

Project: `D:\workspace\dark`. Windows 11 Pro 10.0.26200; Rust/Cargo 1.99.1, stable MSVC. Windows đã đạt các kiểm tra dưới đây. M0 chưa đóng: Ubuntu 22.04 trong WSL chưa có Cargo/Rust, đang chờ người dùng cho phép cài; Linux chưa chạy thử. macOS chưa được kiểm chứng runtime.

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

- Linux: WSL Ubuntu 22.04 có `/usr/bin/cc`, nhưng không có `cargo`, `rustc` hoặc `$HOME/.cargo/bin/cargo`. Chưa build hoặc chạy test; không tự cài khi chưa được phép.
- macOS ARM64: đã cài target standard library và `cargo check --target aarch64-apple-darwin` đạt (exit 0). Build release thất bại ở bước link vì thiếu Apple SDK (`xcrun`) và linker `cc` trên Windows. Chưa build ra binary macOS và chưa chạy thực tế; không xem type-check là build hoặc chạy macOS thành công.
- Unix phục hồi chế độ raw ban đầu qua crossterm và hiện con trỏ khi thoát; không cam kết giữ lại trạng thái con trỏ vốn ẩn vì không có getter portable.
- Không bảo đảm phục hồi terminal sau khi hệ điều hành cưỡng chế kết thúc process; panic thông thường đã kiểm tra cả debug và release.
- M0 chỉ có ba màn hình trống. Chưa làm git, diff, AI, editor hoặc đọc pipe process con; chất lượng AI chưa thể đánh giá ở milestone này và không có cắt context.
- Reviewer đã kiểm tra lại hai bản sửa Windows (cursor ban đầu ẩn và nhận/vẽ lại resize), không thấy blocker mới trên Windows.
