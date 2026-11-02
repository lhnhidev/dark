# dark — Đặc tả giao diện

> Phiên bản: 1.1 · Cập nhật: 01/11/2026
> Liên quan: [FEATURES.md](FEATURES.md) · [PLANNING.md](PLANNING.md) · [ARCHITECTURE.md](ARCHITECTURE.md)

Tài liệu này là **chuẩn bắt buộc** cho giao diện của dark. Ảnh trong [`docs/ui/`](ui/) là đích nhìn; khi ảnh và văn bản khác nhau, **văn bản thắng** (ảnh được vẽ bằng HTML nên khoảng cách pixel chỉ mang tính minh họa). Mọi thay đổi giao diện phải cập nhật tài liệu này trước.

**Thay đổi được duyệt 02/11/2026 — chỉnh cột và toàn bộ phím Alt:** mọi lệnh bàn phím đang có, kể cả điều hướng, xác nhận và thoát, phải giữ Alt. Phím chữ không kèm Alt không thực thi lệnh. `Alt+1…7` đổi tab; `Alt+f` đổi khung (không dùng Alt+Tab vì Windows giữ tổ hợp này); `Alt+↑/↓/←/→` và `Alt+j/k` điều hướng; `Alt+PageUp/PageDown/Home` cuộn; `Alt+Enter` mở; `Alt+a/x`, `Alt+n/N`, `Alt+l/r`, `Alt+v`, `Alt+u`, `Alt+e`, `Alt+?`, `Alt+/` giữ chức năng tương ứng. Hộp xác nhận dùng `Alt+y/n` xác nhận/hủy (không phụ thuộc Alt+Esc vì hệ điều hành có thể giữ tổ hợp này); `Alt+q` thoát và `Alt+Ctrl+c` là cách thoát dự phòng. Các mô tả thao tác ở milestone sau cũng tuân theo quy tắc thêm Alt; chưa triển khai tính năng của milestone sau. Stage file dùng `Alt+g`, không dùng Alt+Space vì xung đột menu cửa sổ Windows.

Cột danh sách Review có thể kéo tại đường phân cách bằng nút chuột trái, hoặc `Alt+-` thu / `Alt+=` giãn mỗi lần 4 cột, `Alt+0` về mặc định. Độ rộng tùy chỉnh chỉ nhớ trong phiên, không tự giãn theo tên file. Khi cả hai khung hiện, giữ danh sách ≥24 cột và diff ≥40 cột; độ rộng thực tế được kẹp theo cửa sổ, nhưng nhớ mức người dùng chọn khi cửa sổ mở rộng lại. Mặc định vẫn 20%, giới hạn 24–40 cột như ảnh. Diff <120 cột tự về unified. Không cho kéo khi chỉ còn một khung, tab khác, màn hình quá nhỏ hoặc có hộp xác nhận. Chỉ kéo vùng nội dung/tiêu đề, không nhận thanh tab hay trạng thái; bắt đầu ở cột phân cách và kết thúc khi thả chuột, đổi tab, resize cửa sổ hoặc mở hộp xác nhận. Kéo chỉ vẽ lại, không đọc lại Git/diff. Giữ vị trí cuộn bằng dòng code khi split/unified đổi. Mouse capture bật khi vào terminal, tắt khi thoát, panic hoặc nhường cho editor ngoài; terminal hỗ trợ có thể dùng Shift để chọn chữ như thông thường. Không yêu cầu giữ Alt khi kéo chuột.

## 1. Ảnh tham chiếu

**Ngoại lệ đổi tab được duyệt 02/11/2026:** `1…7` đổi tab trực tiếp, `Alt+1…7` vẫn dùng được. Nhãn tab hiển thị số đơn. Các lệnh khác vẫn cần Alt; hộp xác nhận không cho đổi tab. Khi có chế độ nhập nội dung ở milestone sau, số dùng để nhập và Alt+1…7 dùng đổi tab.

**Sửa tương thích WSL 02/11/2026:** bộ phím chỉnh cột chính là `Alt+-` thu, `Alt+=` giãn 4 cột và `Alt+0` về mặc định. `Alt+[` / `Alt+]` / `Alt+\` còn là alias khi terminal gửi được phím rõ ràng. Legacy terminal gửi Alt+[ dưới dạng `ESC [` trùng tiền tố CSI, nên không quảng bá nó làm phím chính và không dùng timeout để đoán (có thể phá chuỗi chuột/mũi tên đến chậm). Cột ở mức tối thiểu 24 sẽ không thu thêm.

| Tab | Ảnh | Làm ở milestone |
|---|---|---|
| 1 Review | [ui/1-review.png](ui/1-review.png) | M1 (ghi chú AI ở M5) |
| 2 Edit | [ui/2-edit.png](ui/2-edit.png) | M2 (dùng bộ tô màu cú pháp có từ M1) |
| 3 Vibe | [ui/3-vibe.png](ui/3-vibe.png) | M3 (chọn và gọi CLI), M5 (agent chạy bên trong), M7 (6 ô) |
| 4 Git commit | [ui/4-git-commit.png](ui/4-git-commit.png) | M3 |
| 5 Models | [ui/5-models.png](ui/5-models.png) | M6 |
| 6 View, 7 Help | [ui/6-7-coming-soon.png](ui/6-7-coming-soon.png) | M1 (chỉ "Coming soon") |

Ảnh có kích thước 1280×800 px, tương đương terminal **160 cột × 44 dòng** (1 cột ≈ 8 px, 1 dòng ≈ 18 px). Khi kiểm tra, chạy dark ở 160×44 rồi so với ảnh.

Bản gốc có thể chỉnh trên canvas "Vibe Review TUI", hàng "Thiết kế v2" (riêng tư, chủ dự án mở được).

## 2. Màu (theme)

Bảng màu lấy từ theme mặc định của **Hunk** (`github-dark-default`, Copyright (c) Modem Labs Inc., giấy phép MIT), là kết quả Hunk tự pha từ theme GitHub Dark Default.

**Quy tắc:** mọi màu nằm trong **một module duy nhất** (`src/ui/theme.rs`) dưới dạng hằng có tên. Không mã nào khác được viết mã màu trực tiếp. Có unit test kiểm tra đúng từng mã hex dưới đây.

| Token | Hex | Dùng cho |
|---|---|---|
| `background` | `#0d1117` | Nền chính, nền dòng không đổi |
| `panel` | `#1e2329` | Thanh tab, thanh trạng thái, tiêu đề khung, dòng tiêu đề hunk, ô trống trong diff 2 cột, nền ghi chú AI |
| `panel_alt` | `#272b31` | Nền phím trong gợi ý phím, nút |
| `border` | `#34393f` | Viền và đường phân cách khi không được chọn |
| `text` | `#e6edf3` | Chữ chính |
| `muted` | `#adaeb1` | Chữ phụ, nhãn |
| `line_number` | `#878c92` | Số dòng, chữ mờ hơn nữa (mục chưa dùng được) |
| `accent` | `#bb8009` | **Ô/khung đang active**, hunk đang chọn, viền ghi chú AI của hunk đang chọn, con trỏ |
| `accent_muted` | `#392d14` | Nền tab đang chọn, nền dòng đang chọn |
| `added_bg` | `#12251d` | Nền dòng thêm |
| `added_word_bg` | `#163923` | Nền phần chữ thay đổi trong dòng thêm |
| `added_sign` | `#2ea043` | Dấu `+`, vạch đánh dấu dòng thêm trong editor |
| `removed_bg` | `#3c1e21` | Nền dòng xóa |
| `removed_word_bg` | `#4f2325` | Nền phần chữ thay đổi trong dòng xóa |
| `removed_sign` | `#f85149` | Dấu `−` |
| `file_added` | `#77c185` | Chữ `A`, `?`, số `+N`, trạng thái "sẵn sàng", "đã kết nối" |
| `file_deleted` | `#fa8e89` | Chữ `D`, số `−N`, cảnh báo `⚠` |
| `file_modified` | `#d3ac5f` | Chữ `M`, `R`, chấm "có thay đổi", "● chưa lưu", số thứ tự tab đang chọn |

### Màu cú pháp (từ M1, trong diff; editor dùng lại từ M2)

Theo theme VS Code **GitHub Dark Default** (`primer/github-vscode-theme` v6.3.5, `src/theme.js` commit `f7a67d6`, 2024-10-03; mã hex đối chiếu qua bản build của Shiki `github-dark-default.json`). Chỉ đổi **màu chữ**; nền vẫn theo loại dòng diff và phần chữ đổi.

| Token | Hex | Dùng cho |
|---|---|---|
| `syntax_keyword` | `#ff7b72` | Từ khóa, từ khai báo (`const`, `fn`, `def`, `return`, `if`…), ký tự thoát trong chuỗi, `#include`, `@rule` CSS |
| `syntax_string` | `#a5d6ff` | Chuỗi |
| `syntax_comment` | `#8b949e` | Chú thích |
| `syntax_function` | `#d2a8ff` | Tên hàm khi khai báo và khi gọi (`tên(`) |
| `syntax_constant` | `#79c0ff` | Số, `true`/`false`/`null`/`None`…, `HẰNG_SỐ`, thuộc tính HTML, thuộc tính CSS, tiêu đề Markdown (đậm) |
| `syntax_type` | `#ffa657` | Tên kiểu và class (định danh viết hoa chữ đầu, kiểu có sẵn của ngôn ngữ), lifetime Rust `'a` |
| `syntax_tag` | `#7ee787` | Thẻ HTML/JSX, khóa JSON/YAML/TOML |

Biến, dấu câu và phần còn lại dùng `text`. Bộ tô màu tự viết nên khác VS Code ở vài chỗ: lời gọi hàm thư viện tô tím (VS Code tô xanh dương), tham số hàm không tô cam, toán tử để màu `text`, regex và template lồng nhau có thể sai; hunk bắt đầu giữa một chú thích hoặc chuỗi nhiều dòng sẽ tô sai đoạn đầu hunk vì trạng thái bắt đầu lại ở mỗi hunk.

**Ngôn ngữ (theo đuôi file):** JS/TS (`js jsx mjs cjs ts tsx`), Python, HTML, CSS, JSON, Rust, Go, Java, C/C++, C#, PHP, Shell (`sh bash zsh`), YAML, TOML, Markdown (chỉ tiêu đề, code trong dòng và khối code). Đuôi khác: không tô màu. `NO_COLOR`: không tô màu.

**Chọn màu 24-bit hay 256 màu:** người dùng không phải đặt biến nào. dark dùng màu 24-bit khi: chạy trên Windows (Windows 10 trở lên và Windows Terminal đều hỗ trợ); hoặc `COLORTERM` là `truecolor`/`24bit`; hoặc `TERM` chứa `truecolor`, `24bit` hay `direct`; hoặc có `WT_SESSION`; hoặc `TERM_PROGRAM` là `iTerm.app`, `WezTerm`, `vscode`, `ghostty`. Không thấy dấu hiệu nào thì dùng 256 màu.

**Khi dùng 256 màu:** màu có sắc (kênh lớn nhất trừ kênh nhỏ nhất ≥ 16) chỉ được quy về ô có sắc trong khối 6×6×6, không quy về dải xám; điểm chọn là khoảng cách RGB cộng phạt lệch hướng sắc, để nền thêm/xóa vẫn xanh/đỏ và nền đang chọn vẫn nâu/olive (không thành đỏ). Màu gần xám (chênh < 16: nền, panel, chữ) quy về màu 256 gần nhất như cũ. **Biến `NO_COLOR` được đặt:** không dùng màu, chỉ giữ chữ đậm/đảo màu cho ô đang chọn.

**Không bao giờ chỉ dùng màu để truyền nghĩa:** dòng thêm/xóa luôn có dấu `+`/`−`, trạng thái file luôn có chữ `M A D ? R`, ô đang chọn luôn có vạch `▌` ở đầu.

## 3. Bố cục chung

```
┌ dòng 0 ─ thanh tab ─────────────────────────────────────────────────────────┐
│ dark  1 Review  2 Edit  3 Vibe  4 Git commit  5 Models  6 View  7 Help   …  │
├ dòng 1 … H-2 ─ nội dung của tab ────────────────────────────────────────────┤
│                                                                              │
├ dòng H-1 ─ thanh trạng thái ────────────────────────────────────────────────┤
│ REVIEW  l danh sách file  r khung diff  …                            2/8 …  │
└──────────────────────────────────────────────────────────────────────────────┘
```

### 3.1 Thanh tab (dòng 0)
- Nền `panel`, dưới cùng là một đường `border` (vẽ bằng màu nền dòng 1 tách biệt, không tốn thêm dòng).
- Trái: chữ `dark` màu `accent`, đậm.
- Mỗi tab: ` N Nhãn ` với `N` màu `line_number`, nhãn màu `muted`.
- **Tab đang chọn:** nền `accent_muted`, nhãn `text` đậm, `N` màu `file_modified`.
- Phải: `tên-project · ⎇ nhánh ↑trước ↓sau · snapshot HH:MM`, màu `muted`, căn sát phải; `↑N`/`↓N` chỉ hiện khi khác 0. Thông tin này bị cắt trước tiên khi thiếu chỗ.
- Đổi tab: `1`…`7`, hoặc `Alt+1`…`Alt+7`. Khi có chế độ nhập nội dung ở milestone sau, số dùng để nhập và chỉ `Alt+1`…`Alt+7` đổi tab.

### 3.2 Thanh trạng thái (dòng cuối)
- Nền `panel`. Trái: tên chế độ viết hoa (`REVIEW`, `EDIT`, `VIBE`, `GIT`, `MODELS`, `VIEW`, `HELP`) màu `text`.
- Gợi ý phím: phím trong ô nền `panel_alt` chữ `text`, nhãn màu `muted`, cách nhau 2 cột. Cặp phím như `Alt+n/N` vẽ thành hai ô `n` và `N`, giữa là `Alt+/` màu `muted`.
- Phải: thông tin ngữ cảnh màu `muted` (tiến độ review, vị trí con trỏ...).
- `--debug`: thêm RAM của dark ở cuối bên phải.

### 3.3 Khung (pane)
- Tiêu đề khung: một dòng riêng nền `panel` (không nằm trên đường viền), tiêu đề màu `text`, thông tin phụ màu `muted`.
- Dưới dòng tiêu đề, mỗi khung luôn **giữ chỗ** cho viền: 1 cột trái, 1 cột phải, 1 dòng đáy. Hai khung cạnh nhau dùng chung một cột ở giữa.
- **Khung đang active** (đang nhận phím): vẽ viền `│ └ ─ ┘` màu `accent` vào các ô giữ chỗ. **Khung không active không có viền**: các ô giữ chỗ để trống nền `background`. Nhờ giữ chỗ, đổi khung active không làm nội dung xê dịch. Đổi khung active: `Alt+f`.
- Phân cách giữa hai khung: cột chung ở giữa. Ở dòng tiêu đề là `│` màu `border` trên nền `panel`; bên dưới là viền của khung active nên mang màu `accent`.
- Khi chỉ hiện một khung, khung đó luôn coi là active. Khi `NO_COLOR`, có/không có viền cho biết khung nào active.
- **Dòng đang chọn** trong danh sách: nền `accent_muted`, cột đầu tiên là `▌` màu `accent`.

### 3.4 Kích thước tối thiểu và màn hẹp

| Độ rộng terminal | Hành vi |
|---|---|
| < 80 cột hoặc < 24 dòng | Chỉ hiện thông báo "Cửa sổ quá nhỏ, cần ít nhất 80×24" |
| < 100 cột | Nhãn tab rút gọn thành số + chữ đầu (`1 Rev`, `2 Edt`...); bỏ thông tin bên phải thanh tab |
| Khung diff < 120 cột | Diff tự chuyển từ 2 cột sang 1 cột (inline) |
| Panel trái | Mặc định 20%, giới hạn 24–40 cột; kéo đường phân cách hoặc dùng Alt+- / Alt+= / Alt+0; giữ danh sách ≥24 và diff ≥40 cột |

## 4. Tab 1 · Review

Ảnh: [ui/1-review.png](ui/1-review.png)

### 4.1 Panel trái: danh sách thay đổi (kiểu git panel của Zed)
- `Alt+l` ẩn/hiện. Khi ẩn, khung diff chiếm toàn bộ chiều ngang.
- Tiêu đề: `Changes` (`text`) + `N file · X/N đã xem` (`muted`).
- Ba nhóm, mỗi nhóm thu/mở được (`▾`/`▸`): **Staged (n)**, **Changes (n)**, **Untracked (n)**. Giữa các nhóm không có dòng trống (ảnh có dòng trống; bỏ để dành chỗ cho danh sách ở màn thấp).
- Mỗi dòng file: `[x]`/`[ ]` (đã stage hay chưa) · chữ trạng thái có màu (`M` `file_modified`, `A` `file_added`, `D` `file_deleted`, `?` `file_added`, `R` `file_modified`) · thư mục màu `line_number` + tên file màu `text` (ví dụ `auth/` + `session.ts`) · bên phải `+N` (`file_added`) và `−N` (`file_deleted`); phần bằng 0 không hiện, cả hai bằng 0 thì không hiện số.
- Thiếu chỗ: rút gọn thư mục từ bên trái (`…/auth/`), rồi bỏ hẳn thư mục; **luôn giữ tên file**. Chỉ khi riêng tên file cũng không vừa mới cắt giữa, giữ khoảng 1/4 đầu (ít nhất 1 ký tự khi có từ 3 cột) và phần còn lại cho đuôi, luôn còn ít nhất 1 ký tự đuôi (2 cột: `…s`) (ví dụ `module_01.ts` trong 7 cột → `m…01.ts`), để phần đuôi phân biệt (`01`/`11`/`21`) và đuôi mở rộng còn thấy.
- File đã xem hết các hunk: thêm `✓` sau tên (M1); file đang mở: kiểu dòng đang chọn.
- Dưới cùng: nhãn `Commit message` (`muted`), ô nhập một dòng (nền `background`, viền `border`, viền `accent` khi đang gõ), gợi ý `Alt+g stage · Alt+c commit`.
- Phím: `Alt+↑`/`Alt+↓` hoặc `Alt+j`/`Alt+k` chọn file · `Alt+Enter` mở diff · `Alt+g` stage/unstage file · `Alt+s` stage tất cả · `Alt+c` tới ô commit message, `Alt+Enter` trong ô để commit · `Alt+d` bỏ toàn bộ thay đổi của file (**có hỏi xác nhận**).

**Trong M1:** vùng commit giữ nguyên vị trí nhưng vô hiệu hóa: đường kẻ `─` màu `border` ngăn với danh sách; nhãn, ô nhập (chữ `Có ở M3`) và dòng gợi ý đều màu `line_number`; gợi ý không có ô phím `panel_alt` để không trông như đang dùng được. Màn cao từ 32 dòng: ô nhập có viền 3 dòng như ảnh; thấp hơn: ô gọn 1 dòng `[ Có ở M3 ]` để giữ chỗ cho danh sách file. Các phím stage/commit/bỏ cả file chỉ báo chưa có ở M1, không thay đổi index. File có cả staged và unstaged có thể hiện trong hai nhóm nhưng tổng file và tiến độ chỉ đếm một lần.

### 4.2 Khung phải: diff (80%)
- `Alt+r` ẩn/hiện. Hai khung đều ẩn: hiện dòng giữa màn hình "Ấn l hoặc r để hiện lại".
- Tiêu đề: đường dẫn file (`text`) · `hunk i/n` (`muted`) · bên phải `+N` (`file_added`) `−N` (`file_deleted`) · chế độ `split`/`unified` (`muted`).
- **Dòng tiêu đề hunk:** nền `panel`, chữ `muted`: `@@ -a,b +c,d @@ ngữ cảnh`.
- **Diff 2 cột** (cũ trái, mới phải), giữa là `│` màu `border`. Mỗi cột: số dòng (5 cột, căn phải, `line_number`) · 1 khoảng trắng · dấu (1 cột) · 1 khoảng trắng · nội dung, ví dụ `   41 − const raw`.

| Loại dòng | Nền | Dấu |
|---|---|---|
| Không đổi | `background` | trống |
| Thêm | `added_bg`; phần chữ đổi `added_word_bg` | `+` màu `added_sign` |
| Xóa | `removed_bg`; phần chữ đổi `removed_word_bg` | `−` màu `removed_sign` |
| Ô trống (bên kia không có dòng tương ứng) | `panel` | trống |
| Đoạn không đổi bị thu gọn | `background`, chữ `muted`: `⋯ N dòng không đổi` | — |

- **Màu chữ của code** theo bảng màu cú pháp ở §2 cho mọi loại dòng (không đổi, thêm, xóa); nền theo bảng trên. Trạng thái chú thích/chuỗi nhiều dòng được theo dõi riêng cho cột cũ và cột mới, bắt đầu lại ở mỗi hunk.

- **Hunk đang chọn:** cột đầu tiên của dòng tiêu đề hunk và mọi dòng của hunk có `▌` màu `accent`.
- **Diff 1 cột** (khi hẹp hoặc chọn `unified`): dòng xóa ngay trên dòng thêm tương ứng, cùng bảng màu.

### 4.3 Ghi chú AI dưới mỗi hunk (từ M5)
- Khung ngay dưới hunk, thụt vào 1 cột hai bên, nền `panel`, viền `accent` nếu là hunk đang chọn, viền `border` nếu không.
- Dòng đầu: `◆ Agent` (`accent` nếu đang chọn, `muted` nếu không) + `tóm tắt hunk · không ghi vào code` (`muted`).
- Nội dung: 1–3 câu ngắn, màu `text`. Cảnh báo bắt đầu bằng `⚠`, màu `file_deleted`.
- Chỉ hunk đang chọn có dòng gợi ý phím: `a accept · x reject · Enter edit · ? hỏi về hunk này`.
- Hunk chưa có ghi chú: một dòng `muted` "◆ Agent: đang tóm tắt…" hoặc không hiện gì nếu chưa có agent.
- Ghi chú **không bao giờ** được ghi vào file mã nguồn; lưu trong `.dark/notes/`.

### 4.4 Phím của tab Review

| Phím | Tác dụng |
|---|---|
| `Alt+l` / `Alt+r` | Ẩn/hiện panel danh sách / khung diff |
| `Alt+f` | Đổi khung active |
| `Alt+n` / `Alt+Shift+n` | Hunk kế / trước (sang file kế khi hết hunk) |
| `Alt+a` / `Alt+x` | Accept / reject hunk |
| `Alt+Enter` | Sửa hunk ngay tại chỗ (mở editor của dark tại dòng đó) |
| `Alt+?` | Hỏi agent về hunk (từ M5) |
| `Alt+/` | Ghi chú của người dùng gửi cho agent (từ M5) |
| `Alt+u` | Hoàn tác cả lượt AI về snapshot (có xác nhận) |
| `Alt+e` | Mở file trong editor ngoài |
| `Alt+v` | Đổi split / unified |
| `Alt+-` / `Alt+=` / `Alt+0` | Thu / giãn danh sách 4 cột / về độ rộng mặc định |
| `Alt+y` / `Alt+n` | Xác nhận / hủy hộp đang mở |
| `Alt+q` | Thoát |

**Trong M1:** `Alt+Enter` ở danh sách mở diff; `Alt+Enter` ở diff báo sửa nội bộ có ở M2 và gợi ý `Alt+e`. `Alt+?` và `Alt+/` báo có ở M5 trên thanh trạng thái, không im lặng. Khi chưa có ghi chú AI, thanh trạng thái vẫn chỉ dẫn `Alt+a accept` và `Alt+x reject`; màn hẹp ưu tiên giữ hai phím này. `Alt+u` ghi rõ khôi phục về **lúc mở phiên dark**, liệt kê file bị ảnh hưởng và cảnh báo gồm cả thay đổi người dùng tự làm sau khi mở; file đổi tiếp trong lúc xác nhận thì hủy thao tác và yêu cầu xem lại.

**Tự cập nhật (đưa lên M1 theo duyệt ngày 02/11/2026):** thay đổi từ bên ngoài được nhận qua thông báo hệ điều hành, gộp trong khoảng 150 ms trước khi tải lại diff; không quét file định kỳ. Giữ file, hunk và vị trí cuộn đang xem khi còn xác định được. Trong lúc mở hộp xác nhận, ghi nhận thay đổi nhưng hoãn cập nhật màn hình tới khi đóng hộp; kiểm tra stale trước khi ghi vẫn bắt buộc. WSL đọc ổ Windows dùng `dark.exe` chạy ngầm làm cầu nối. Nếu không bật được theo dõi hoặc cầu nối mất kết nối, báo trên thanh trạng thái; người dùng vẫn đổi tab để tải lại được. Bố cục, màu và phím không đổi.

## 5. Tab 2 · Edit

Ảnh: [ui/2-edit.png](ui/2-edit.png)

### 5.1 Cây thư mục (trái, 20%)
- Tiêu đề `Explorer`. `Alt+Ctrl+B` ẩn/hiện.
- Thụt 2 cột mỗi cấp; thư mục có `▾`/`▸`; tải dần, chỉ đọc thư mục khi mở. Bỏ qua file theo `.gitignore`.
- Thư mục chứa file đã đổi: chấm `•` màu `file_modified`. File đã đổi: chữ trạng thái bên phải như panel Review.
- Phím: `Alt+↑`/`Alt+↓` chọn · `Alt+→`/`Alt+←` mở/thu thư mục · `Alt+Enter` mở file.

### 5.2 Editor (phải)
- Tiêu đề: đường dẫn file + `● chưa lưu` (`file_modified`) khi có thay đổi.
- Lề trái: số dòng (`line_number`; dòng hiện tại màu `text`) rồi **vạch thay đổi 1 cột** so với snapshot: `▎` màu `added_sign` (dòng mới), `▎` màu `file_modified` (dòng sửa), `▁` màu `removed_sign` (có dòng bị xóa ở đây).
- Dòng chứa con trỏ: nền `panel`. Con trỏ: ô nền `accent`, chữ `background`.
- Vùng chọn: nền `accent_muted`.
- Code tô màu cú pháp theo §2 (bộ tô màu có từ M1).
- Thanh trạng thái phải: `Ngôn ngữ · UTF-8 · LF|CRLF · Ln X, Col Y`.
- Phím: xem [FEATURES.md](FEATURES.md) mục Edit (`Alt+Ctrl+S`, `Alt+Ctrl+Z`/`Alt+Ctrl+Y`, `Alt+Shift`+mũi tên, `Alt+Ctrl+C`/`Alt+Ctrl+X`/`Alt+Ctrl+V`, `Alt+Ctrl+F`, `Alt+Ctrl+G`, `Alt+n`). Trong editor, `Alt+Ctrl+C` là copy, không thoát app.

## 6. Tab 3 · Vibe

Ảnh: [ui/3-vibe.png](ui/3-vibe.png)

- **Dải ô agent** (dòng ngay dưới thanh tab): 6 ô `1 · tên-agent` … `6 · +`, ô đang chọn kiểu tab đang chọn. Bên phải: `MVP: 1 agent · tối đa 6 ở bản sau` cho tới M7. Ô 2–6 màu `line_number` và không chọn được trước M7. Đổi ô: `Alt+Ctrl+1`…`Alt+Ctrl+6`.
- **Ô chưa có agent:** khoảng trống, giữa màn hình là hộp chọn agent (rộng ~65% hoặc tối đa 104 cột), viền `accent`, nền `panel`:
  - Tiêu đề `Chọn agent cho ô N` + mô tả `muted`.
  - Nhóm **CLI ĐÃ CÀI TRÊN MÁY** và **MODEL QUA API KEY (từ tab 5 Models)**, tiêu đề nhóm màu `muted`.
  - Mỗi dòng: dấu chọn `▸` · tên · chi tiết (`muted`) · trạng thái: `sẵn sàng` (`file_added`), `cần API key` (`file_modified`), `chưa cài` (`line_number`, cả dòng mờ).
  - Dòng chân: `Enter bắt đầu · ↑↓ chọn · 5 quản lý API key`.
- **M3:** chỉ nhóm CLI; `Alt+Enter` gọi agent CLI ở giao diện gốc (nhường màn hình hoặc chia màn hình), agent thoát thì về tab Review. Nhóm API hiện một dòng `muted` "Có ở v0.2".
- **Từ M5, ô có agent đang chạy** (chưa có ảnh, theo cùng bảng màu):
  - Luồng hội thoại: tin nhắn người dùng có tiền tố `›` màu `muted`; trả lời của agent màu `text`; kế hoạch dạng danh sách `☐`/`☑`; lệnh agent chạy trong khung nền `panel` có tiền tố `$`; file agent sửa hiện `M path +N −N`.
  - **Yêu cầu xin quyền:** khung viền `accent` như ghi chú AI, có `y cho phép · n từ chối · d xem diff`.
  - Ô nhập ở đáy, viền `accent` khi đang gõ; `Alt+Enter` gửi, `Shift-Enter` xuống dòng.
  - Thanh trạng thái phải: tên agent · RAM của agent · token/chi phí lượt này (từ M8).

## 7. Tab 4 · Git commit

Ảnh: [ui/4-git-commit.png](ui/4-git-commit.png)

- **Danh sách commit (trái, ~73%)**, tiêu đề cột `graph · hash · message · author · time` nền `panel`.
  - `graph`: lấy từ `git log --graph`, tối đa 12 cột; màu nhánh xoay vòng `file_modified`, `file_added`, `file_deleted`, `muted`.
  - `hash`: 7 ký tự, `line_number`. Nhãn nhánh: nền `file_modified` chữ `background`; tag: nền `file_added` chữ `background`.
  - `author`, `time` (tương đối, ví dụ "2 giờ trước"): `muted`.
  - Tải dần theo trang khoảng 200 commit khi cuộn.
- **Chi tiết commit (phải, ~27%, tối thiểu 36 cột):** message đậm; `hash · tác giả · ngày giờ`, `parent …` màu `muted`; tổng `N file · +a −b`; danh sách file với chữ trạng thái và `+/−`.
- Phím: `Alt+↑`/`Alt+↓` chọn · `Alt+Enter` mở diff commit bằng chính khung diff của tab Review (chỉ đọc, không accept/reject) · `Alt+/` tìm · `Alt+b` đổi nhánh (M3: chỉ xem, không checkout).

## 8. Tab 5 · Models (M6)

Ảnh: [ui/5-models.png](ui/5-models.png)

- **Danh sách nhà cung cấp (trái, ~23%)**, nhóm: nhà cung cấp đám mây (Anthropic, OpenAI, Google Gemini, OpenRouter, DeepSeek), **MÁY CỦA BẠN** (Ollama, LM Studio), **TÙY CHỈNH** (`+ OpenAI-compatible`). Trạng thái bên phải: `✓ đã kết nối` (`file_added`), `chưa có key` / `không chạy` (`line_number`).
- **Biểu mẫu (phải):** nhãn `muted` trên ô; ô nhập nền `background`, viền `border`, viền `accent` khi đang gõ; API key luôn hiển thị dạng `••••`; ô `Base URL` chỉ bật với OpenAI-compatible; chọn model mặc định.
- Nút vẽ dạng `[ Kiểm tra kết nối ]` `[ Lưu ]`; nút đang chọn nền `accent_muted`, viền/chữ `accent`. Kết quả: `✓ kết nối thành công · N ms` (`file_added`) hoặc `✗ lý do` (`file_deleted`).
- Dòng ghi chú `line_number`: key lưu trong kho mật khẩu của hệ điều hành, không ghi ra file hay log.
- Phím: `Alt+↑`/`Alt+↓` nhà cung cấp · `Alt+f` trường kế · `Alt+t` kiểm tra · `Alt+Ctrl+S` lưu · `Alt+Del` xóa key (có xác nhận).

## 9. Tab 6 · View và Tab 7 · Help

Ảnh: [ui/6-7-coming-soon.png](ui/6-7-coming-soon.png)

Giữa màn hình: tên tab đậm, dòng dưới `Coming soon` màu `muted`. Thanh trạng thái chỉ có `VIEW`/`HELP` và gợi ý quay về tab khác.

## 10. Kiểm tra giao diện

Mỗi milestone có thay đổi giao diện phải:
- Chạy dark ở 160×44 và ở 80×24, chụp màn hình (hoặc ghi lại bằng PTY trong test), so với ảnh tham chiếu: bố cục, màu, ký tự viền, gợi ý phím.
- Có test cho bảng màu (đúng mã hex), cho bố cục ở nhiều độ rộng (160, 120, 100, 80 cột) và cho quy tắc màn hẹp ở mục 3.4.
- Ghi rõ trong báo cáo những chỗ khác ảnh và lý do.
