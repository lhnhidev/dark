# M1 — Khung giao diện và Review

## Sửa tương thích 02/11/2026 — phím thu cột trên WSL

Legacy Alt+[ gửi `ESC [` trùng tiền tố CSI, nên Crossterm 0.29 có thể chờ phần còn lại thay vì tạo KeyEvent. Unit test trước chỉ truyền KeyEvent đã giải mã, chưa phát hiện vấn đề này. Phím chính nay là **Alt+- thu, Alt+= giãn, Alt+0 về mặc định**; alias ngoặc/vạch chéo giữ cho terminal gửi được phím rõ ràng. Không đoán bằng timeout và không thay parser, protocol, dependency hoặc thread.

Đã bổ sung kiểm thử chuỗi terminal thật cho `ESC -`, `ESC =`, `ESC 0`, cùng `CSI 91;3u` biểu diễn Alt+[ rõ ràng; kiểm mỗi lần đường phân cách di chuyển đúng. Console Windows thử cả ba phím và kéo chuột. Script Windows dùng VkKeyScanW để gán đúng virtual key: trước sửa script, mã ký tự `-` bị gán thành VK_INSERT, làm lần thử đầu thất bại dù mã ứng dụng nhận phím đúng.

Hai runner đầy đủ cuối cùng ở `.dark/verification/m1/portable-resize/final-{windows,musl}/` đạt exit 0 (fmt, Clippy, test debug/release, build, RAM và terminal). Terminal Windows 10/10, Linux PTY 6/6. Lượt musl đầu có hai test retention snapshot thất bại; chạy lại riêng review đạt 22/22 và toàn runner cuối đạt, không thay mã snapshot để làm test qua. Log lượt đầu và rerun vẫn giữ; chưa chốt nguyên nhân tính không ổn định này.

Fixture 30 file / 3.000 dòng đổi, watcher và đường xử lý chỉnh cột vẫn hoạt động khi đo:

| Chỉ số bản sửa | Windows EX2 private working set | Linux musl VmRSS |
|---|---:|---:|
| Idle: lớn nhất trong 5 mẫu | 1.429.504 byte | 925.696 byte |
| Đỉnh lấy mẫu sau thao tác | 1.576.960 byte | 925.696 byte |
| Binary release | 558.080 byte | 848.760 byte |

Không coi đỉnh lấy mẫu là đỉnh toàn bộ đời process; không thay metric hoặc ngưỡng. Các số ở phần bên dưới là lịch sử các lần bàn giao trước. Bản ghi 160/80 cột cũng được cập nhật nhãn phím mới; bản sửa không đo lại riêng cây helper/Git Windows. Tại giới hạn tối thiểu 24 cột, phím thu không làm cột nhỏ hơn nữa.

## Bổ sung 02/11/2026 — kéo cột và toàn bộ phím Alt

Chủ dự án đã duyệt chỉnh độ rộng danh sách bằng chuột và yêu cầu **mọi phím tắt đang có đều giữ Alt**. `Alt+[` thu, `Alt+]` giãn 4 cột, `Alt+\` về mặc định; giữ độ rộng chọn trong phiên và kẹp theo cửa sổ (danh sách ≥24, diff ≥40 cột). Kéo đường phân cách bằng nút trái không tải lại Git/diff. `Alt+f` đổi khung để tránh Alt+Tab của Windows; `Alt+q`/`Alt+Ctrl+C` thoát; hộp xác nhận dùng `Alt+y/n`. Phím chữ và Ctrl+C cũ không chạy lệnh. Alt+mũi tên có Alt+j/k dự phòng khi terminal giữ phím điều hướng.

Đã sửa và có hồi quy giữ đúng dòng code đang đọc khi kéo qua ngưỡng split/unified hoặc đổi bằng Alt+v. Không kéo khi modal, khung ẩn, tab khác hoặc màn hình quá nhỏ; resize/đổi tab/mở modal hủy trạng thái kéo. Mouse capture bật khi vào terminal, tắt khi thoát/panic/nhường editor; Windows khôi phục console mode ban đầu. Không thêm dependency hoặc thread, release profile giữ nguyên.

Hai runner đầy đủ kết thúc exit 0, gồm fmt, Clippy `-D warnings`, test debug/release, build debug/release, RAM, help/version và terminal thật:

```text
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/verify-windows.ps1 -OutputDirectory .dark/verification/m1/resize-alt/windows
bash scripts/verify-linux.sh musl .dark/verification/m1/resize-alt/musl
```

Windows console **10/10**, Linux PTY **6/6** đạt: kéo chuột thật, Alt đổi tab/thoát, phím thường không thoát hay ghi, resize trước phím tiếp và phục hồi terminal sau panic. Unit ứng dụng 132/133, CLI 3/3, RAM release 2/2, workbench 152/153 (Windows/Linux; có test module được nạp lại, không cộng thành test độc lập). WSL đọc ổ D với writer Linux và Windows tiếp tục qua kiểm tra tự cập nhật bằng phím Alt. Reviewer độc lập không còn lỗi chặn sau sửa vị trí cuộn.

Fixture RAM **30 file / 3.000 dòng đổi** giữ watcher hoạt động, vẽ ở 160/120/100/80 cột; nay thử thêm phím chỉnh cột, reset và đường xử lý kéo ở 160/80 cột trước đo idle. Không lấy probe hay khung trống thay tiêu chí:

| Số đo bản này | Windows, EX2 private working set | Linux musl, VmRSS |
|---|---:|---:|
| Review: lớn nhất trong 5 mẫu idle | 1.503.232 byte | 937.984 byte |
| Review: đỉnh lấy mẫu sau thao tác | 1.503.232 byte | 937.984 byte |
| Binary release | 558.080 byte | 848.760 byte |

Idle vẫn **<2.000.000 byte**, đỉnh lấy mẫu **<10.000.000 byte**. Binary tăng 3.072 byte (Windows) / 4.096 byte (musl) so với `fb02f67`; đó là cả thay đổi kéo cột và phím Alt. Không quy chênh lệch hai lượt RAM thành chi phí cố định riêng của thao tác kéo; trang resident và cách lấy mẫu có dao động. Đây không phải đỉnh bảo đảm cho mọi repo.

Bản ghi PTY [160×44](ui/m1/160x44.html), [80×24](ui/m1/80x24.html) đã tạo lại bằng binary mới; thông tin SHA-256 và process con ở [README bản ghi](ui/m1/README.md). **Khác ảnh tham chiếu có chủ đích:** nhãn tab và gợi ý dùng Alt, thanh trạng thái có phím chỉnh cột; độ rộng mặc định giữ nguyên, chỉ đổi khi người dùng kéo hoặc bấm phím. Màu và viền không đổi.

Log mới ở `.dark/verification/m1/resize-alt/`; các số ở phần tự cập nhật và báo cáo ban đầu bên dưới là lịch sử. Kiểm tra biên dịch macOS ARM64 đạt, chưa chạy macOS thật. Chưa đo riêng lại toàn bộ cây Git trên Windows cho bản này; không dùng chỉ số khác để giả là đã đo. M1 vẫn chờ chủ dự án chạy thử và duyệt đóng, không tự tạo tag.

Phần kiểm chứng ban đầu ngày 01/11/2026 dưới đây ghi lại bản `a46f7cc`, không phải số đo của mã mới nhất. Bổ sung tự cập nhật ngày 02/11/2026 nằm ở phần kế tiếp. M1 vẫn chờ chủ dự án chạy thử và duyệt đóng; chưa tạo tag `m1`.

## Bổ sung 02/11/2026 — tự cập nhật diff

Theo kế hoạch đã được chủ dự án duyệt, C6 được đưa từ M3 lên M1. Windows dùng `ReadDirectoryChangesW`, Linux dùng inotify; WSL đọc ổ Windows dùng `dark.exe` chạy ngầm làm cầu nối. Không thêm dependency, không thay release profile, không tạo thread ứng dụng. Khoảng chờ terminal 75 ms chỉ để rút thông báo đã có, không quét file/Git định kỳ. Gộp thông báo 150 ms, chặn trì hoãn quá 500 ms khi ghi liên tục.

Diff giữ file/hunk và vị trí cuộn khi còn xác định được; hộp xác nhận không bị thay thế bởi refresh nền. Snapshot phục hồi vẫn là lúc mở phiên. Nếu có commit từ bên ngoài, baseline review chuyển theo HEAD hiện tại; diff cũ bị kiểm tra stale trước reject. Tính năng không sửa thiết kế: bố cục, màu, viền và phím không đổi.

### Kiểm chứng bản bổ sung

```text
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/verify-windows.ps1 -OutputDirectory .dark/verification/m1/auto-refresh/final-windows
bash scripts/verify-linux.sh musl .dark/verification/m1/auto-refresh/final-musl
```

Các runner bao gồm format, Clippy `-D warnings`, test debug/release, build debug/release, self-test, help/version, binary và terminal thật. Windows console còn kiểm sửa file, thay thế file khi lưu và file mới **không gửi phím**; PTY bổ sung thử native Linux, WSL ổ D với writer Linux lẫn Windows, idle không tự redraw liên tục, xác nhận stale và thoát `q`. Test SIGKILL xác nhận trực tiếp PID cầu nối Windows đã kết thúc, không chỉ proxy Linux.

| Kiểm tra | Windows x86_64 MSVC | Linux x86_64 musl, WSL |
|---|---:|---:|
| Unit test ứng dụng | 128 | 129 |
| CLI | 3 | 3 |
| RAM release | 2 | 2 |
| Review/snapshot | 21 | 22 |
| Watcher | 5 | 8 |
| Workbench và module được nạp lại | 144 | 145 |

Không cộng các hàng thành số test độc lập. Hồi quy watcher gồm atomic save, thư mục mới/xóa, linked worktree, `.dark` lồng hợp lệ, `.DARK` gốc trên Windows, file tracked trong thư mục bị ignore, force-add sau khi mở watcher, thay ignore rules và tái tạo `.git/info`. Reviewer độc lập không còn lỗi chặn sau sửa và kiểm thử.

### RAM và binary của bản bổ sung

Fixture ngân sách có **30 file / 3.000 dòng đổi**, vẽ 160/120/100/80 cột như trước. `--self-test-review` lần này giữ watcher đang hoạt động trong toàn bộ phép đo; không dùng khung trống hay probe để báo đạt.

| Số đo | Windows (EX2 private working set) | Linux musl (VmRSS) |
|---|---:|---:|
| Review: lớn nhất trong 5 mẫu idle | 1.355.776 byte | 921.600 byte |
| Review: đỉnh lấy mẫu sau thao tác | 1.527.808 byte | 921.600 byte |
| Binary release | 555.008 byte | 844.664 byte |

Hai nền tảng đạt idle **< 2.000.000 byte**, đỉnh lấy mẫu **< 10.000.000 byte**. Đây không phải đỉnh bảo đảm trong toàn bộ đời process hoặc mọi repo.

PTY trên fixture nhỏ, đo riêng các tiến trình khi dùng WSL đọc ổ D:

| Tiến trình/chỉ số | Linux ghi trên ổ D | Windows ghi trên ổ D |
|---|---:|---:|
| Dark, VmRSS idle | 851.968 byte | 815.104 byte |
| Proxy interop Linux, VmRSS | 2.207.744 byte | 2.207.744 byte |
| Cầu nối Windows, trung vị 5 mẫu EX2 private working set | 638.976 byte | 643.072 byte |

Không cộng hoặc đổi tên các chỉ số khác nhau để báo đạt. Proxy và cầu nối là chi phí thật ngoài process Dark, không bỏ khỏi báo cáo; không có chúng khi chạy native Linux. Native Linux PTY trên fixture nhỏ quan sát Dark 880.640 byte VmRSS. Các số PTY này không thay fixture 30 file của ngân sách. Bản bổ sung chưa đo lại riêng cây Git; dữ liệu cây Git ở phần báo cáo ban đầu là lịch sử.

Log ở `.dark/verification/m1/auto-refresh/`, gồm `final-{windows,musl}/`, `pty-{ext4,drive-linux,drive-windows,kill-owner}.txt` và `macos-arm-check.txt`. macOS ARM64 chỉ qua `cargo check`, chưa chạy thực tế và chưa có backend notification; lần thử target Intel trong WSL không chạy được vì chưa cài target. Chưa stress overflow queue thật; mã có đường phục hồi theo thông báo overflow nhưng không ghi là đã kiểm chứng nhánh đó. Benchmark đầy đủ với tuicr/Hunk vẫn còn mở như phần ban đầu; không dùng probe [so sánh cơ chế](../tests/bench/WATCH_PROBE.md) thay cho benchmark ứng dụng.

## Báo cáo ban đầu 01/11/2026 (lịch sử)

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

Giới hạn ở bản kiểm chứng ban đầu: patch >512 KiB hoặc untracked >256 KiB mở bằng editor ngoài, chưa paging; nhị phân/không UTF-8 và đường dẫn đặc biệt không phục hồi an toàn được báo rõ/chặn ghi. Rename đang thể hiện thành hai phần thêm/xóa. Bản ban đầu chưa theo dõi thay đổi nền; phần bổ sung 02/11 phía trên đã thay thế giới hạn này trên Windows/Linux. Dấu đã xem thuộc phiên hiện tại, không persist qua lần mở app.

## Bổ sung 02/11/2026 — Đồng bộ nhánh bên ngoài

Đã đo RAM trước khi sửa theo yêu cầu chủ dự án. Lỗi là nhãn nhánh chỉ được lấy lúc mở Workbench; watcher đã nhận thay đổi HEAD và Review đã dùng HEAD mới. Bản sửa dùng lại cách đọc tên nhánh và ahead/behind khi refresh thành công. Không thêm dependency, luồng, bộ theo dõi hay cache; chỉ chạy thêm hai lệnh Git khi có refresh, không chạy Git định kỳ lúc rảnh. Bố cục và màu giao diện giữ nguyên.

Fixture release 30 file/3.000 dòng đổi, watcher đang hoạt động; byte thập phân:

| Phép đo | Trước sửa | Sau sửa |
|---|---:|---:|
| Windows private working set idle | 1.466.368 | 1.323.008 |
| Windows private working set đỉnh lấy mẫu | 1.564.672 | 1.462.272 |
| WSL Linux musl VmRSS idle | 901.120 | 933.888 |
| WSL Linux musl VmRSS đỉnh lấy mẫu | 901.120 | 933.888 |

Windows trước/sau dùng `cargo test --release --test ram release_review_fixture_meets_real_memory_limits -- --nocapture` và binary `target/release/dark.exe`. Linux dùng cùng bài test với target musl; số sau lấy từ runner musl. Đây là từng lượt, có dao động cấp phát/trang nhớ; không kết luận bản sửa giảm RAM. Cả hai vẫn dưới ngưỡng idle 2.000.000 byte và đỉnh 10.000.000 byte. Binary sau sửa: Windows 558.080 byte; Linux musl 848.760 byte.

Kiểm chứng: fmt, Clippy với `-D warnings`, test debug/release, build debug/release, help/version/self-test đạt trong hai runner Windows và musl. Test mới bao phủ đổi nhánh cùng cây, khác cây với diff mới, detached HEAD và ahead/behind. Test watcher xác nhận HEAD đổi dù nội dung file không đổi. Terminal Windows đổi nhánh bằng Git bên ngoài, không gửi phím cho Dark, chờ tiêu đề cửa sổ cập nhật; phép kiểm ban đầu tìm nhãn trên màn hình hẹp thất bại vì tên fixture dài che nhãn, đã sửa phép kiểm sang tiêu đề đầy đủ và chạy lại. Hành vi đang mở hộp xác nhận vẫn trì hoãn refresh để giữ an toàn.

Thử thêm WSL đọc ổ D, ghi từ Windows: tự cập nhật, thay file bằng rename, file mới, bảo vệ xác nhận và thoát cầu nối đều đạt. App VmRSS idle 851.968 byte; đo riêng proxy Linux 2.199.552 byte VmRSS và cầu nối Windows median 638.976 byte EX2 private working set (5 mẫu). Không cộng các chỉ số khác nhau để xét ngân sách app; chưa đo riêng đỉnh RAM của Git trong lần kiểm này. Log runner tại `.dark/verification/branch-refresh/after/{windows,musl}`; terminal Windows chạy lại tại `windows/terminal-retry.txt`.

Giới hạn: không thêm thao tác đổi nhánh trong Dark; chưa kiểm chứng macOS. Không mở rộng watcher cho `refs/remotes`, nên không khẳng định tự cập nhật ahead/behind chỉ do fetch khi không có sự kiện đang theo dõi.

## Bổ sung 02/11/2026 — Bấm chọn file trong Review

Đã đo bản cũ trước khi triển khai. Bấm chuột trái trên dòng file chọn file và nạp diff hiện có; không thêm thư viện, luồng, cache hay bộ đệm giữ lâu. Tính dòng từ danh sách đang vẽ, dùng lại danh sách Item tạm; bấm lại cùng file không đọc lại Git hoặc đặt lại vị trí cuộn. Danh sách vẫn nhận focus, kể cả khi diff đang ẩn. Chỉ thêm hành vi vào UI_SPEC trước khi sửa mã; bố cục, màu và ảnh tham chiếu không đổi.

Fixture release 30 file/3.000 dòng đổi, watcher đang chạy; bài đo sau sửa có thêm bấm qua các dòng đang hiển thị ở 160×44 và 80×24:

| Phép đo (byte) | Trước sửa | Sau sửa |
|---|---:|---:|
| Windows EX2 private working set idle | 1.413.120 | 1.458.176 |
| Windows EX2 private working set đỉnh lấy mẫu | 1.486.848 | 1.597.440 |
| Linux musl VmRSS idle | 946.176 | 950.272 |
| Linux musl VmRSS đỉnh lấy mẫu | 946.176 | 950.272 |

Số từng lượt, có dao động trang nhớ; không dùng chênh lệch này làm kích thước cấp phát chính xác của tính năng. Idle vẫn dưới 2.000.000 byte, đỉnh lấy mẫu dưới 10.000.000 byte. Binary release Windows 558.592 byte (tăng 512 byte), musl 848.760 byte (không đổi so với bản ngay trước). Không gộp RAM Git/terminal vào ngân sách app; lần này chưa đo riêng đỉnh tiến trình Git, không tạo thêm tiến trình nền.

Năm test hành vi mới bao phủ chọn/nạp/focus, danh sách cuộn, nhóm thu, dòng trùng đường dẫn Staged/Changes, giữ vị trí đọc khi bấm lại, vùng không phải file, chuột phải, hộp xác nhận và chế độ chỉ danh sách. Test kéo cột hiện có vẫn đạt. Terminal Windows gửi mouse events để chuyển từ sample.txt sang file mới rồi quay lại; Linux PTY gửi chuỗi SGR mouse thật để chuyển giữa hai file. Kiểm tra fmt, Clippy `-D warnings`, test debug/release và build release đạt trên Windows/WSL musl. Log runner: `.dark/verification/mouse-review/after/{windows,musl}`. Bài Linux PTY được chạy lại sau khi bổ sung thao tác bấm; cả 6 kịch bản đều đạt.

Giới hạn: chưa thử macOS; không thêm bấm thu/mở nhóm hoặc cuộn bằng bánh xe trong lần thay đổi này.

## Chủ dự án chạy thử

```powershell
cd D:\workspace\dark
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/try-m1.ps1
```

Script tạo clone độc lập dưới `.dark/verification/m1/manual-...`, mở baseline trước M1 rồi áp thay đổi M1 thành working tree để review, không sửa repo nguồn. Clone thử không có remote; giữ lại sau khi thoát để kiểm tra. `-PrepareOnly` chỉ chuẩn bị và in đường dẫn, không mở giao diện.

Đã chạy `-PrepareOnly` và `--self-test-review` trên clone chứa chính thay đổi M1 (31 file ở thời điểm kiểm): private working set idle 1.409.024 byte, đỉnh lấy mẫu 1.777.664 byte, exit 0. Đây là kiểm tra bổ sung trên code M1, không thay fixture 30 file/3.000 dòng của tiêu chí RAM.

Thử review chính thay đổi M1 ở cửa sổ 160×44 rồi 80×24; kiểm tab, nhóm file, phím a/x và thông báo hunk staged (dùng Git bên ngoài trên clone thử để stage một hunk). Nội dung hộp `u` phải rõ về mốc lúc mở phiên. Muốn mở repo đang làm việc trực tiếp thì chạy `target/verify-windows/x86_64-pc-windows-msvc/release/dark.exe --debug` tại repo đó. Sau phản hồi mới quyết định đóng M1; không push/tag tự động.
