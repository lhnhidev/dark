# dark — Sơ đồ tuần tự

> Phiên bản: 0.1 (pre-MVP) · Cập nhật: 01/11/2026
> Liên quan: [ARCHITECTURE.md](ARCHITECTURE.md) · [FEATURES.md](FEATURES.md)

Quy ước: `curl`, `git`, `rg`, test runner và LSP đều là **process con**. App chỉ đọc output của chúng qua pipe trong một vòng lặp sự kiện duy nhất.

## 1. Mở phiên review sau khi vibe code

```mermaid
sequenceDiagram
    actor U as Vibe coder
    participant UI as UI (Review)
    participant RV as Review engine
    participant G as git

    U->>UI: Mở dark trong thư mục project
    UI->>RV: Tải trạng thái review
    RV->>G: Tìm snapshot gần nhất (.dark ref)
    G-->>RV: commit/ref snapshot
    RV->>G: git diff <snapshot> --name-status
    G-->>RV: Danh sách file đã đổi
    RV->>G: git diff <snapshot> -- <file đầu tiên>
    G-->>RV: Patch
    RV->>RV: Tách hunk, chỉ giữ hunk trong RAM
    RV-->>UI: Danh sách file + hunk hiện tại
    UI-->>U: Hiển thị file list + diff
```

## 2. Giữ hoặc bỏ một hunk

```mermaid
sequenceDiagram
    actor U as Vibe coder
    participant UI as UI (Review)
    participant RV as Review engine
    participant G as git

    U->>UI: Bấm "a" (giữ) hoặc "r" (bỏ)
    alt Giữ
        UI->>RV: accept(hunk)
        RV->>G: git apply --cached (stage hunk)
    else Bỏ
        UI->>RV: revert(hunk)
        RV->>G: git apply -R (áp ngược hunk vào working tree)
    end
    G-->>RV: OK / lỗi
    RV-->>UI: Cập nhật trạng thái, chuyển hunk kế
    UI-->>U: Đánh dấu ✓, hiện hunk tiếp theo
```

## 3. Hỏi AI về một hunk (stream câu trả lời)

```mermaid
sequenceDiagram
    actor U as Vibe coder
    participant UI as UI
    participant AG as Agent / Context
    participant ST as Session store (đĩa)
    participant C as curl
    participant M as Model AI

    U->>UI: Bấm "?" rồi gõ câu hỏi
    UI->>AG: ask(câu hỏi, hunk, file)
    AG->>ST: Ghi tin nhắn người dùng
    AG->>AG: Ghép context: AGENTS.md + hunk + file liên quan
    AG->>C: Spawn curl, stream request body vào stdin
    C->>M: HTTPS POST (stream=true, prompt caching)
    loop Mỗi sự kiện SSE
        M-->>C: delta text
        C-->>AG: dòng SSE qua stdout
        AG-->>UI: Nối text, vẽ lại dòng đang đổi
    end
    M-->>C: Kết thúc stream
    C-->>AG: EOF, curl thoát
    AG->>ST: Ghi câu trả lời, giải phóng bộ đệm
    UI-->>U: Hiện nhận xét ngay dưới dòng liên quan
```

## 4. Agent thực hiện task (vòng lặp công cụ + kiểm tra)

```mermaid
sequenceDiagram
    actor U as Vibe coder
    participant AG as Agent loop
    participant CX as Context builder
    participant G as git
    participant C as curl → Model
    participant T as Tools (read/grep/edit/run)
    participant V as Verify (lint/test/LSP)

    U->>AG: Giao task
    AG->>G: Tạo checkpoint (snapshot)
    AG->>CX: Lấy repo map + AGENTS.md
    CX-->>AG: Context (repo map đọc từ cache đĩa)
    loop Cho tới khi model trả lời xong
        AG->>C: Gửi hội thoại đầy đủ + định nghĩa tool
        C-->>AG: Stream: text hoặc tool_call
        alt tool_call = read / grep / glob
            AG->>T: Thực thi
            T-->>AG: Kết quả
        else tool_call = edit_file
            AG->>T: Áp dụng sửa (search/replace, khớp gần đúng)
            alt Khớp được
                T-->>AG: OK + diff
                AG->>V: Chạy lint/typecheck/test liên quan
                V-->>AG: Kết quả (lỗi nếu có)
            else Không khớp / khớp nhiều chỗ
                T-->>AG: Lỗi cụ thể + đoạn gần giống nhất
            end
        else tool_call = run_command
            AG->>T: Chạy lệnh (cần duyệt nếu nguy hiểm)
            T-->>AG: stdout/stderr
        end
        AG->>AG: Nếu gần hết context → compaction
    end
    AG-->>U: Kết thúc lượt → mở màn Review với diff so với checkpoint
```

## 5. Áp dụng patch AI đề xuất (có duyệt)

```mermaid
sequenceDiagram
    actor U as Vibe coder
    participant UI as UI
    participant ED as Edit engine
    participant G as git
    participant FS as File

    U->>UI: Bấm "p" tại nhận xét của AI
    UI->>ED: preview(patch)
    ED-->>UI: Diff xem trước
    UI-->>U: Hiện diff, chờ xác nhận
    alt Đồng ý
        U->>UI: Xác nhận
        UI->>G: Tạo checkpoint
        UI->>ED: apply(patch)
        ED->>FS: Ghi file
        ED-->>UI: OK
        UI-->>U: Cập nhật diff, đánh dấu đã sửa
    else Từ chối
        U->>UI: Esc
        UI-->>U: Giữ nguyên file
    end
```

## 6. Chạy test và nhờ AI sửa lỗi

```mermaid
sequenceDiagram
    actor U as Vibe coder
    participant UI as UI (Edit + Run)
    participant R as Test runner
    participant AG as Agent loop

    U->>UI: Bấm "!"
    UI->>R: Spawn lệnh test (từ config hoặc tự phát hiện)
    loop Output
        R-->>UI: stdout/stderr
    end
    R-->>UI: Exit code
    alt Có test lỗi
        UI-->>U: Hiện lỗi đầu tiên + vị trí file:dòng
        U->>UI: Bấm "?" (nhờ AI sửa)
        UI->>AG: Task "sửa lỗi test" + output lỗi
        Note over AG: Chạy như sơ đồ 4 (có checkpoint, verify)
        AG-->>UI: Patch đã kiểm tra
        UI-->>U: Mở màn Review để duyệt
    else Tất cả pass
        UI-->>U: Ẩn panel lỗi
    end
```

## 7. Đổi model

```mermaid
sequenceDiagram
    actor U as Vibe coder
    participant UI as UI (chọn model)
    participant CFG as providers.toml
    participant ENV as Biến môi trường

    U->>UI: Bấm "m"
    UI->>CFG: Đọc danh sách provider
    UI->>ENV: Kiểm tra key_env của từng provider
    UI-->>U: Danh sách model (thiếu key → làm mờ)
    U->>UI: Chọn model, Enter
    UI->>UI: Đặt model cho phiên hiện tại
    Note over UI: Định dạng sửa file đổi theo họ model mới
    UI-->>U: Đóng danh sách
```
