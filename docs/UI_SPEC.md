# dark — Đặc tả giao diện

> Phiên bản: 1.1 · Cập nhật: 01/11/2026
> Liên quan: [FEATURES.md](FEATURES.md) · [PLANNING.md](PLANNING.md) · [ARCHITECTURE.md](ARCHITECTURE.md)

Tài liệu này là **chuẩn bắt buộc** cho giao diện của dark. Ảnh trong [`docs/ui/`](ui/) là đích nhìn; khi ảnh và văn bản khác nhau, **văn bản thắng** (ảnh được vẽ bằng HTML nên khoảng cách pixel chỉ mang tính minh họa). Mọi thay đổi giao diện phải cập nhật tài liệu này trước.

## 1. Ảnh tham chiếu

| Tab | Ảnh | Làm ở milestone |
|---|---|---|
| 1 Review | [ui/1-review.png](ui/1-review.png) | M1 (ghi chú AI ở M5) |
| 2 Edit | [ui/2-edit.png](ui/2-edit.png) | M2 (tô màu cú pháp ở M9) |
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

Màu cú pháp (chỉ dùng từ M9, theo GitHub Dark Default): từ khóa `#ff7b72`, hàm `#d2a8ff`, chuỗi `#a5d6ff`, chú thích `#8b949e`. Trước M9, code hiển thị bằng `text`.

**Chọn màu 24-bit hay 256 màu:** người dùng không phải đặt biến nào. dark dùng màu 24-bit khi: chạy trên Windows (Windows 10 trở lên và Windows Terminal đều hỗ trợ); hoặc `COLORTERM` là `truecolor`/`24bit`; hoặc `TERM` chứa `truecolor`, `24bit` hay `direct`; hoặc có `WT_SESSION`; hoặc `TERM_PROGRAM` là `iTerm.app`, `WezTerm`, `vscode`, `ghostty`. Không thấy dấu hiệu nào thì dùng 256 màu.

**Khi dùng 256 màu:** màu có sắc (không phải xám) chỉ được quy về ô màu có sắc gần nhất trong khối 6×6×6, không quy về dải xám, để nền thêm/xóa vẫn xanh/đỏ và nền đang chọn vẫn nâu. Màu gần xám (nền, panel, chữ) quy về màu 256 gần nhất như cũ. **Biến `NO_COLOR` được đặt:** không dùng màu, chỉ giữ chữ đậm/đảo màu cho ô đang chọn.

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
- Đổi tab: phím `1`…`7`; khi đang gõ chữ (editor, ô nhập) dùng `Alt-1`…`Alt-7`.

### 3.2 Thanh trạng thái (dòng cuối)
- Nền `panel`. Trái: tên chế độ viết hoa (`REVIEW`, `EDIT`, `VIBE`, `GIT`, `MODELS`, `VIEW`, `HELP`) màu `text`.
- Gợi ý phím: phím trong ô nền `panel_alt` chữ `text`, nhãn màu `muted`, cách nhau 2 cột. Cặp phím như `n/N` vẽ thành hai ô `n` và `N`, giữa là `/` màu `muted`.
- Phải: thông tin ngữ cảnh màu `muted` (tiến độ review, vị trí con trỏ...).
- `--debug`: thêm RAM của dark ở cuối bên phải.

### 3.3 Khung (pane)
- Tiêu đề khung: một dòng riêng nền `panel` (không nằm trên đường viền), tiêu đề màu `text`, thông tin phụ màu `muted`.
- Dưới dòng tiêu đề, mỗi khung luôn **giữ chỗ** cho viền: 1 cột trái, 1 cột phải, 1 dòng đáy. Hai khung cạnh nhau dùng chung một cột ở giữa.
- **Khung đang active** (đang nhận phím): vẽ viền `│ └ ─ ┘` màu `accent` vào các ô giữ chỗ. **Khung không active không có viền**: các ô giữ chỗ để trống nền `background`. Nhờ giữ chỗ, đổi khung active không làm nội dung xê dịch. Đổi khung active: `Tab` / `Shift-Tab`.
- Phân cách giữa hai khung: cột chung ở giữa. Ở dòng tiêu đề là `│` màu `border` trên nền `panel`; bên dưới là viền của khung active nên mang màu `accent`.
- Khi chỉ hiện một khung, khung đó luôn coi là active. Khi `NO_COLOR`, có/không có viền cho biết khung nào active.
- **Dòng đang chọn** trong danh sách: nền `accent_muted`, cột đầu tiên là `▌` màu `accent`.

### 3.4 Kích thước tối thiểu và màn hẹp

| Độ rộng terminal | Hành vi |
|---|---|
| < 80 cột hoặc < 24 dòng | Chỉ hiện thông báo "Cửa sổ quá nhỏ, cần ít nhất 80×24" |
| < 100 cột | Nhãn tab rút gọn thành số + chữ đầu (`1 Rev`, `2 Edt`...); bỏ thông tin bên phải thanh tab |
| Khung diff < 120 cột | Diff tự chuyển từ 2 cột sang 1 cột (inline) |
| Panel trái | 20% độ rộng, tối thiểu 24 cột, tối đa 40 cột; khung còn lại chiếm phần còn lại (~80%) |

## 4. Tab 1 · Review

Ảnh: [ui/1-review.png](ui/1-review.png)

### 4.1 Panel trái: danh sách thay đổi (kiểu git panel của Zed)
- `l` ẩn/hiện. Khi ẩn, khung diff chiếm toàn bộ chiều ngang.
- Tiêu đề: `Changes` (`text`) + `N file · X/N đã xem` (`muted`).
- Ba nhóm, mỗi nhóm thu/mở được (`▾`/`▸`): **Staged (n)**, **Changes (n)**, **Untracked (n)**. Giữa các nhóm không có dòng trống (ảnh có dòng trống; bỏ để dành chỗ cho danh sách ở màn thấp).
- Mỗi dòng file: `[x]`/`[ ]` (đã stage hay chưa) · chữ trạng thái có màu (`M` `file_modified`, `A` `file_added`, `D` `file_deleted`, `?` `file_added`, `R` `file_modified`) · thư mục màu `line_number` + tên file màu `text` (ví dụ `auth/` + `session.ts`) · bên phải `+N` (`file_added`) và `−N` (`file_deleted`); phần bằng 0 không hiện, cả hai bằng 0 thì không hiện số.
- Thiếu chỗ: rút gọn thư mục từ bên trái (`…/auth/`), rồi bỏ hẳn thư mục; **luôn giữ tên file**. Chỉ khi riêng tên file cũng không vừa mới cắt giữa, giữ khoảng 1/4 đầu (ít nhất 1 ký tự) và phần còn lại cho đuôi (ví dụ `module_01.ts` trong 7 cột → `m…01.ts`), để phần đuôi phân biệt (`01`/`11`/`21`) và đuôi mở rộng còn thấy.
- File đã xem hết các hunk: thêm `✓` sau tên (M1); file đang mở: kiểu dòng đang chọn.
- Dưới cùng: nhãn `Commit message` (`muted`), ô nhập một dòng (nền `background`, viền `border`, viền `accent` khi đang gõ), gợi ý `Space stage · c commit`.
- Phím: `↑`/`↓` hoặc `j`/`k` chọn file · `Enter` mở diff · `Space` stage/unstage file · `s` stage tất cả · `c` tới ô commit message, `Enter` trong ô để commit · `d` bỏ toàn bộ thay đổi của file (**có hỏi xác nhận**).

**Trong M1:** vùng commit giữ nguyên vị trí nhưng vô hiệu hóa: đường kẻ `─` màu `border` ngăn với danh sách; nhãn, ô nhập (chữ `Có ở M3`) và dòng gợi ý đều màu `line_number`; gợi ý không có ô phím `panel_alt` để không trông như đang dùng được. Màn cao từ 32 dòng: ô nhập có viền 3 dòng như ảnh; thấp hơn: ô gọn 1 dòng `[ Có ở M3 ]` để giữ chỗ cho danh sách file. Các phím stage/commit/bỏ cả file chỉ báo chưa có ở M1, không thay đổi index. File có cả staged và unstaged có thể hiện trong hai nhóm nhưng tổng file và tiến độ chỉ đếm một lần.

### 4.2 Khung phải: diff (80%)
- `r` ẩn/hiện. Hai khung đều ẩn: hiện dòng giữa màn hình "Ấn l hoặc r để hiện lại".
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
| `l` / `r` | Ẩn/hiện panel danh sách / khung diff |
| `Tab` | Đổi khung active |
| `n` / `N` | Hunk kế / trước (sang file kế khi hết hunk) |
| `a` / `x` | Accept / reject hunk |
| `Enter` | Sửa hunk ngay tại chỗ (mở editor của dark tại dòng đó) |
| `?` | Hỏi agent về hunk (từ M5) |
| `/` | Ghi chú của người dùng gửi cho agent (từ M5) |
| `u` | Hoàn tác cả lượt AI về snapshot (có xác nhận) |
| `e` | Mở file trong editor ngoài |
| `v` | Đổi split / unified |
| `q` | Thoát |

**Trong M1:** `Enter` ở danh sách mở diff; `Enter` ở diff báo sửa nội bộ có ở M2 và gợi ý `e`. `?` và `/` báo có ở M5 trên thanh trạng thái, không im lặng. Khi chưa có ghi chú AI, thanh trạng thái vẫn chỉ dẫn `a accept` và `x reject`; màn hẹp ưu tiên giữ hai phím này. `u` ghi rõ khôi phục về **lúc mở phiên dark**, liệt kê file bị ảnh hưởng và cảnh báo gồm cả thay đổi người dùng tự làm sau khi mở; file đổi tiếp trong lúc xác nhận thì hủy thao tác và yêu cầu xem lại.

## 5. Tab 2 · Edit

Ảnh: [ui/2-edit.png](ui/2-edit.png)

### 5.1 Cây thư mục (trái, 20%)
- Tiêu đề `Explorer`. `Ctrl-B` ẩn/hiện.
- Thụt 2 cột mỗi cấp; thư mục có `▾`/`▸`; tải dần, chỉ đọc thư mục khi mở. Bỏ qua file theo `.gitignore`.
- Thư mục chứa file đã đổi: chấm `•` màu `file_modified`. File đã đổi: chữ trạng thái bên phải như panel Review.
- Phím: `↑`/`↓` chọn · `→`/`←` mở/thu thư mục · `Enter` mở file.

### 5.2 Editor (phải)
- Tiêu đề: đường dẫn file + `● chưa lưu` (`file_modified`) khi có thay đổi.
- Lề trái: số dòng (`line_number`; dòng hiện tại màu `text`) rồi **vạch thay đổi 1 cột** so với snapshot: `▎` màu `added_sign` (dòng mới), `▎` màu `file_modified` (dòng sửa), `▁` màu `removed_sign` (có dòng bị xóa ở đây).
- Dòng chứa con trỏ: nền `panel`. Con trỏ: ô nền `accent`, chữ `background`.
- Vùng chọn: nền `accent_muted`.
- Trước M9 code màu `text`; ảnh tham chiếu có tô màu cú pháp để minh họa đích M9.
- Thanh trạng thái phải: `Ngôn ngữ · UTF-8 · LF|CRLF · Ln X, Col Y`.
- Phím: xem [FEATURES.md](FEATURES.md) mục Edit (`Ctrl-S`, `Ctrl-Z`/`Ctrl-Y`, `Shift`+mũi tên, `Ctrl-C`/`Ctrl-X`/`Ctrl-V`, `Ctrl-F`, `Ctrl-G`, `Esc`). Trong editor, `Ctrl-C` là copy, không thoát app.

## 6. Tab 3 · Vibe

Ảnh: [ui/3-vibe.png](ui/3-vibe.png)

- **Dải ô agent** (dòng ngay dưới thanh tab): 6 ô `1 · tên-agent` … `6 · +`, ô đang chọn kiểu tab đang chọn. Bên phải: `MVP: 1 agent · tối đa 6 ở bản sau` cho tới M7. Ô 2–6 màu `line_number` và không chọn được trước M7. Đổi ô: `Ctrl-1`…`Ctrl-6`.
- **Ô chưa có agent:** khoảng trống, giữa màn hình là hộp chọn agent (rộng ~65% hoặc tối đa 104 cột), viền `accent`, nền `panel`:
  - Tiêu đề `Chọn agent cho ô N` + mô tả `muted`.
  - Nhóm **CLI ĐÃ CÀI TRÊN MÁY** và **MODEL QUA API KEY (từ tab 5 Models)**, tiêu đề nhóm màu `muted`.
  - Mỗi dòng: dấu chọn `▸` · tên · chi tiết (`muted`) · trạng thái: `sẵn sàng` (`file_added`), `cần API key` (`file_modified`), `chưa cài` (`line_number`, cả dòng mờ).
  - Dòng chân: `Enter bắt đầu · ↑↓ chọn · 5 quản lý API key`.
- **M3:** chỉ nhóm CLI; `Enter` gọi agent CLI ở giao diện gốc (nhường màn hình hoặc chia màn hình), agent thoát thì về tab Review. Nhóm API hiện một dòng `muted` "Có ở v0.2".
- **Từ M5, ô có agent đang chạy** (chưa có ảnh, theo cùng bảng màu):
  - Luồng hội thoại: tin nhắn người dùng có tiền tố `›` màu `muted`; trả lời của agent màu `text`; kế hoạch dạng danh sách `☐`/`☑`; lệnh agent chạy trong khung nền `panel` có tiền tố `$`; file agent sửa hiện `M path +N −N`.
  - **Yêu cầu xin quyền:** khung viền `accent` như ghi chú AI, có `y cho phép · n từ chối · d xem diff`.
  - Ô nhập ở đáy, viền `accent` khi đang gõ; `Enter` gửi, `Shift-Enter` xuống dòng.
  - Thanh trạng thái phải: tên agent · RAM của agent · token/chi phí lượt này (từ M8).

## 7. Tab 4 · Git commit

Ảnh: [ui/4-git-commit.png](ui/4-git-commit.png)

- **Danh sách commit (trái, ~73%)**, tiêu đề cột `graph · hash · message · author · time` nền `panel`.
  - `graph`: lấy từ `git log --graph`, tối đa 12 cột; màu nhánh xoay vòng `file_modified`, `file_added`, `file_deleted`, `muted`.
  - `hash`: 7 ký tự, `line_number`. Nhãn nhánh: nền `file_modified` chữ `background`; tag: nền `file_added` chữ `background`.
  - `author`, `time` (tương đối, ví dụ "2 giờ trước"): `muted`.
  - Tải dần theo trang khoảng 200 commit khi cuộn.
- **Chi tiết commit (phải, ~27%, tối thiểu 36 cột):** message đậm; `hash · tác giả · ngày giờ`, `parent …` màu `muted`; tổng `N file · +a −b`; danh sách file với chữ trạng thái và `+/−`.
- Phím: `↑`/`↓` chọn · `Enter` mở diff commit bằng chính khung diff của tab Review (chỉ đọc, không accept/reject) · `/` tìm · `b` đổi nhánh (M3: chỉ xem, không checkout).

## 8. Tab 5 · Models (M6)

Ảnh: [ui/5-models.png](ui/5-models.png)

- **Danh sách nhà cung cấp (trái, ~23%)**, nhóm: nhà cung cấp đám mây (Anthropic, OpenAI, Google Gemini, OpenRouter, DeepSeek), **MÁY CỦA BẠN** (Ollama, LM Studio), **TÙY CHỈNH** (`+ OpenAI-compatible`). Trạng thái bên phải: `✓ đã kết nối` (`file_added`), `chưa có key` / `không chạy` (`line_number`).
- **Biểu mẫu (phải):** nhãn `muted` trên ô; ô nhập nền `background`, viền `border`, viền `accent` khi đang gõ; API key luôn hiển thị dạng `••••`; ô `Base URL` chỉ bật với OpenAI-compatible; chọn model mặc định.
- Nút vẽ dạng `[ Kiểm tra kết nối ]` `[ Lưu ]`; nút đang chọn nền `accent_muted`, viền/chữ `accent`. Kết quả: `✓ kết nối thành công · N ms` (`file_added`) hoặc `✗ lý do` (`file_deleted`).
- Dòng ghi chú `line_number`: key lưu trong kho mật khẩu của hệ điều hành, không ghi ra file hay log.
- Phím: `↑`/`↓` nhà cung cấp · `Tab` trường kế · `t` kiểm tra · `Ctrl-S` lưu · `Del` xóa key (có xác nhận).

## 9. Tab 6 · View và Tab 7 · Help

Ảnh: [ui/6-7-coming-soon.png](ui/6-7-coming-soon.png)

Giữa màn hình: tên tab đậm, dòng dưới `Coming soon` màu `muted`. Thanh trạng thái chỉ có `VIEW`/`HELP` và gợi ý quay về tab khác.

## 10. Kiểm tra giao diện

Mỗi milestone có thay đổi giao diện phải:
- Chạy dark ở 160×44 và ở 80×24, chụp màn hình (hoặc ghi lại bằng PTY trong test), so với ảnh tham chiếu: bố cục, màu, ký tự viền, gợi ý phím.
- Có test cho bảng màu (đúng mã hex), cho bố cục ở nhiều độ rộng (160, 120, 100, 80 cột) và cho quy tắc màn hẹp ở mục 3.4.
- Ghi rõ trong báo cáo những chỗ khác ảnh và lý do.
