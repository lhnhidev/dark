# M1 — Khung giao diện và Review

Ngày kiểm chứng: 01/11/2026. Mã ứng dụng được kiểm chứng tại `a46f7cc`; các commit sau đó chỉ sửa probe/cập nhật bản ghi và tài liệu. M1 chờ chủ dự án chạy thử; chưa push, chưa tag và chưa đóng milestone.

## Kết quả

Đã có bảy tab, theme tập trung, khung active, panel Staged/Changes/Untracked, diff split/unified, word diff, điều hướng, accept/reject, snapshot riêng, xác nhận phục hồi và mở editor ngoài. Các chức năng M2/M3/M5 còn lại được hiển thị chờ đúng milestone, không thực thi ngầm.

Hai runner đầy đủ đều kết thúc exit code 0:

```text
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/verify-windows.ps1 -OutputDirectory .dark/verification/m1/accepted/windows
bash scripts/verify-linux.sh musl .dark/verification/m1/accepted/musl
```

Bao gồm `cargo fmt --check`, Clippy mọi target với `-D warnings`, test debug/release, build debug/release, self-test, help/version, kiểm kích thước và terminal thật. Rust 1.99.1 stable, edition 2021; release profile giữ nguyên.

| Kiểm tra | Windows x86_64 MSVC | Linux x86_64 musl trong WSL Ubuntu 22.04 |
|---|---:|---:|
| Unit test ứng dụng | 26 đạt | 26 đạt |
| CLI | 3 đạt | 3 đạt |
| RAM release | 2 đạt | 2 đạt |
| Backend review/snapshot | 18 đạt | 19 đạt |
| Workbench và các module nạp trong test | 28 đạt | 28 đạt |
| Terminal thật | 10/10 đạt | 6/6 đạt |

Không cộng các hàng thành số test khác nhau: test workbench có nạp lại một số unit test. Hai test RAM được bỏ qua có lý do ở debug và chạy thật ở release. Terminal kiểm chuyển tab, resize trước phím tiếp, q/Ctrl-C/panic, khôi phục chế độ terminal và con trỏ. Windows probe tắt hộp báo crash của hệ điều hành trong môi trường test; panic release tự thoát với mã lỗi, không kill để giả kết quả.

Log đầy đủ là output cục bộ trong `.dark/verification/m1/accepted/{windows,musl}/`, không commit. macOS chưa được kiểm chứng lại cho M1; không suy từ Linux sang macOS. GNU không dùng để báo đạt ngân sách musl.

## Bộ nhớ và binary

Fixture release tạo repo riêng với **30 file, 3.000 dòng thay đổi**, tải diff thật, đi qua các file và vẽ ở 160/120/100/80 cột. Mỗi mẫu sau thao tác lấy bộ nhớ của chính dark, không cộng process Git/editor vào cùng chỉ số.

| Số đo | Windows | Linux musl |
|---|---:|---:|
| Loại chỉ số | EX2 private working set | VmRSS |
| Review: lớn nhất trong 5 mẫu idle | 1.429.504 byte | 806.912 byte |
| Review: đỉnh lấy mẫu sau thao tác | 1.572.864 byte | 806.912 byte |
| Self-test khung: idle | 581.632 byte | 712.704 byte |
| Binary release | 479.232 byte | 738.168 byte |

Review đạt **< 2.000.000 byte**, các đỉnh quan sát đạt **< 10.000.000 byte**; binary đạt < 3.000.000 byte. Đỉnh lấy mẫu không phải cam kết về đỉnh toàn bộ đời process hoặc mọi repo có thể có.

Bản M0 Windows được lưu trước triển khai: binary 212.480 byte, self-test private working set 581.632 byte. Binary M1 tăng 266.752 byte cho toàn bộ tính năng. Không lấy chênh lệch RAM khung M0 và review M1 làm chi phí của riêng dependency vì workload khác nhau.

Process con được lấy mẫu riêng khi ghi terminal Linux: bản 160×44 quan sát đỉnh mẫu RSS dark 856.064 byte và tổng cây con 823.296 byte; bản 80×24 quan sát dark 761.856 byte, không bắt được process con trong các mẫu. Không quan sát được không có nghĩa không chạy Git; quá trình rất ngắn có thể lọt giữa mẫu. Chưa có số private working set riêng cho cây Git trên Windows; không thay bằng chỉ số khác để báo đã đo. Các trường hợp terminal Windows thường quan sát 3–4 thread toàn process, dù mã ứng dụng không tạo worker/runtime async.

## Chi phí word diff riêng

`similar = 2.7.0`, `default-features = false`; không thêm dependency bắc cầu từ crate này. `scripts/measure-word-diff.ps1` dùng fixture `tests/bench/word-diff-probe.rs`, cùng input Unicode, cùng buffer và cùng release flags, 5 lượt mỗi biến thể. Biến thể sau gọi chính `word_spans` của ứng dụng; không có process con.

| Probe Windows | Trước word diff | Sau word diff | Chênh lệch |
|---|---:|---:|---:|
| Binary | 122.368 | 185.856 | +63.488 byte |
| Median private working set idle | 434.176 | 454.656 | +20.480 byte |
| Median đỉnh lấy mẫu | 434.176 | 454.656 | +20.480 byte |

Đây là phép tách riêng chi phí xử lý/token/buffer và crate word diff, không thay cho bài đo ứng dụng đầy đủ. Raw kết quả ở `.dark/verification/m1/word-diff/results.txt`.

Word diff dùng Myers trên token giữ nguyên dấu câu/khoảng trắng, bảo toàn biên grapheme. Dòng >1.000 Unicode scalar, hunk >200 dòng, hoặc quá 10 ms/cặp bỏ highlight theo chữ và giữ màu theo dòng; nội dung diff/patch không bị cắt để giảm ngữ cảnh AI.

## Giao diện và đối chiếu ảnh

- Chuẩn: [UI_SPEC](UI_SPEC.md) và [ảnh Review](ui/1-review.png).
- Bản ghi PTY thật: [160×44 HTML](ui/m1/160x44.html), [80×24 HTML](ui/m1/80x24.html); [ANSI/text và thông tin binary](ui/m1/README.md).
- `.html` là tái dựng từ ANSI đã ghi, không phải ảnh chụp. Đã đọc đối chiếu các frame text, các nhãn và phép đo layout/style tự động; chưa tuyên bố đã kiểm pixel bằng ảnh chụp của terminal Windows.
- 160×44 dùng split; 80×24 dùng unified, tab rút gọn, đường dẫn giữ hậu tố phân biệt. Layout còn được test ở 120/100 cột và dưới 80×24. Palette kiểm từng hex, thử 256 màu/không màu; hash dòng bao gồm style và chỉ commit sau flush thành công.

Các khác biệt có chủ đích so với ảnh:

1. Không có khung ghi chú AI/agent trong M1 (M5). `a accept`/`x reject` vẫn hiện trên thanh phím ở cả hai độ rộng.
2. Vùng commit bị vô hiệu, nhãn `Có ở M3`, chữ `line_number`; Space/s/c/d chỉ báo chưa hỗ trợ, không sửa index.
3. Enter ở diff báo M2 và gợi ý editor ngoài; `?`/`/` báo M5. Tab 2–5 còn là placeholder; View/Help đúng Coming soon.
4. Viền bao quanh pane theo văn bản UI_SPEC, thay vì chỉ đường ngăn như một số chỗ trong ảnh HTML minh họa. Màn hẹp cắt bớt gợi ý thứ yếu để dành chỗ cho tiến độ; tên file/metadata lấy từ fixture thật, không dùng dữ liệu giả trong ảnh.
5. Code dùng một màu `text`; chưa tô cú pháp (M9). Snapshot có giờ mở phiên và ghi rõ diff so với HEAD.

Không đổi bảng màu, tỉ lệ khung hay thiết kế tab. UI_SPEC chỉ bổ sung hành vi M1 đã duyệt và làm rõ gợi ý phím khi chưa có ghi chú AI.

## Snapshot và an toàn dữ liệu

- Snapshot raw-byte tree trong `refs/dark/snapshots/<session>/<turn>`, index tạm riêng. Giữ index thật và HEAD đúng byte, kể cả file có staged/unstaged và LF/CRLF/BOM; test quyền file Unix và đường dẫn Unicode.
- Mốc review HEAD khác mốc phục hồi lúc mở phiên. `u` nêu rõ thời điểm này và cả thay đổi tự làm; liệt kê file, cho cuộn, kiểm token trước/sau lập danh sách và trước ghi. File đổi sau xác nhận thì không khôi phục. Untracked mới ngoài snapshot được giữ.
- Hunk chạm thay đổi đã stage bị chặn; chỉ reject hunk unstaged tách biệt, không tự unstage. Reject kiểm patch đang xem còn khớp và kiểm tra áp ngược trước ghi; không để patch áp một phần.
- Giữ mặc định 10 phiên qua `DARK_SNAPSHOT_KEEP`, bảo vệ phiên đang mở; ref hết hạn và metadata tương ứng được dọn, Git tự GC object mất ref theo cơ chế thông thường. Không chạy `git gc` cưỡng bức.
- File untracked >50 MiB hoặc số untracked >1.000 phải có lựa chọn bỏ qua/hủy trước ghi snapshot. Kiểm lại ở các snapshot trước reject/undo. Output tạm được dọn sau thành công/đóng repo, dữ liệu phục hồi khi lỗi có chính sách giữ theo phiên.
- `refs/dark/*` không nằm trong branch/tag và không được gửi trong push mặc định; test bằng remote bare tạm trên máy. `--mirror` và refspec tùy chỉnh vẫn có thể gửi ref riêng. Không có push nào lên origin thật trong milestone này.
- Undo chuẩn bị thay thế/backup trên đĩa, dùng rename theo file và rollback khi lỗi; test Windows khóa file đích để xác minh rollback. Không hứa atomic toàn bộ nhiều file khi máy/process bị ngắt; journal/ref được giữ khi lỗi. Không có khóa ngăn mọi chương trình ngoài thay file sau kiểm tra cuối.

Các hồi quy quan trọng đã tìm và sửa trong lúc kiểm chứng: bản sao index phải giữ mtime để Git phát hiện racy-clean; ID phiên retry phải cùng đơn vị thời gian để retention giữ đúng phiên mới; accept không đếm trùng hunk/file; hộp undo không được ghi đè file đổi sau danh sách xác nhận.

## Benchmark và giới hạn còn lại

Đã chạy lại `tests/bench/compare-review-tools.py` trên Linux musl: 30 file, 960 dòng thêm/510 dòng xóa, PTY 160×45, một warm-up và 3 lượt 8 giây. dark thực sự xuất diff (79.408 byte/lượt). Median private 888.832 byte, RSS 892.928 byte, PSS 888.832 byte; đỉnh mẫu RSS **cả cây process** 6.168.576 byte, tối đa 3 process. Số cây process này không dùng để thay chỉ số ngân sách app riêng. Log: `.dark/verification/m1/benchmark.txt`.

WSL hiện không có executable tuicr/Hunk, nên chỉ chạy lại dark, không cài thêm và không công bố bảng so sánh mới với đối thủ. Bảng cũ trong ARCHITECTURE vẫn được ghi rõ là lịch sử M0.

Giới hạn hiện tại: patch >512 KiB hoặc untracked >256 KiB mở bằng editor ngoài, chưa paging; nhị phân/không UTF-8 và đường dẫn đặc biệt không phục hồi an toàn được báo rõ/chặn ghi. Rename đang thể hiện thành hai phần thêm/xóa. Chưa tự theo dõi thay đổi nền (M3); đổi file/tab, reject/undo và quay lại từ editor sẽ đọc lại dữ liệu. Dấu đã xem thuộc phiên hiện tại, không persist qua lần mở app.

## Chủ dự án chạy thử

```powershell
cd D:\workspace\dark
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/try-m1.ps1
```

Script tạo clone độc lập dưới `.dark/verification/m1/manual-...`, mở baseline trước M1 rồi áp thay đổi M1 thành working tree để review, không sửa repo nguồn. Clone thử không có remote; giữ lại sau khi thoát để kiểm tra. `-PrepareOnly` chỉ chuẩn bị và in đường dẫn, không mở giao diện.

Đã chạy `-PrepareOnly` và `--self-test-review` trên clone chứa chính thay đổi M1 (31 file ở thời điểm kiểm): private working set idle 1.409.024 byte, đỉnh lấy mẫu 1.777.664 byte, exit 0. Đây là kiểm tra bổ sung trên code M1, không thay fixture 30 file/3.000 dòng của tiêu chí RAM.

Thử review chính thay đổi M1 ở cửa sổ 160×44 rồi 80×24; kiểm tab, nhóm file, phím a/x và thông báo hunk staged (dùng Git bên ngoài trên clone thử để stage một hunk). Nội dung hộp `u` phải rõ về mốc lúc mở phiên. Muốn mở repo đang làm việc trực tiếp thì chạy `target/verify-windows/x86_64-pc-windows-msvc/release/dark.exe --debug` tại repo đó. Sau phản hồi mới quyết định đóng M1; không push/tag tự động.
