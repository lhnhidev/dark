# dark — Sơ đồ tuần tự

> Phiên bản: 0.2 (sau M0) · Cập nhật: 01/11/2026
> Liên quan: [ARCHITECTURE.md](ARCHITECTURE.md) · [FEATURES.md](FEATURES.md) · [PLANNING.md](PLANNING.md)

Quy ước: `git`, agent, editor, test runner là **process con** hoặc chương trình riêng. dark chỉ đọc output của chúng; RAM của chúng đo riêng.

## 1. Mở phiên review (M1)

```mermaid
sequenceDiagram
    actor U as Vibe coder
    participant UI as Màn Review
    participant RV as Review engine
    participant G as git

    U->>UI: Mở dark (hoặc darkw) trong thư mục project
    UI->>RV: Tải trạng thái review
    RV->>G: Tìm snapshot gần nhất (refs/dark/...)
    alt Chưa có snapshot
        RV->>G: Tạo snapshot từ trạng thái hiện tại
    end
    RV->>G: git diff <snapshot> --name-status
    G-->>RV: Danh sách file đã đổi
    RV->>G: git diff <snapshot> -- <file đầu tiên>
    G-->>RV: Patch
    RV->>RV: Tách hunk, chỉ giữ hunk của file đang xem
    RV-->>UI: Danh sách file + hunk hiện tại
    UI-->>U: Hiện danh sách file và diff
```

## 2. Giữ / bỏ hunk, hoàn tác cả lượt (M1)

```mermaid
sequenceDiagram
    actor U as Vibe coder
    participant UI as Màn Review
    participant RV as Review engine
    participant G as git

    alt Bấm "a" (giữ)
        U->>UI: a
        UI->>RV: keep(hunk)
        RV->>RV: Đánh dấu đã giữ, giữ nguyên working tree
    else Bấm "r" (bỏ)
        U->>UI: r
        UI->>RV: drop(hunk)
        RV->>G: git apply -R (áp ngược hunk vào working tree)
        G-->>RV: OK / lỗi
    else Bấm "u" (hoàn tác cả lượt)
        U->>UI: u
        UI-->>U: Xác nhận?
        U->>UI: Đồng ý
        UI->>RV: restore(snapshot)
        RV->>G: Khôi phục working tree về snapshot
        G-->>RV: OK / lỗi
    end
    RV-->>UI: Cập nhật trạng thái hoặc báo lỗi rõ ràng
    UI-->>U: Chuyển hunk kế / làm mới danh sách
```

## 3. Ghi chú và gửi cho agent (M2 xuất markdown, M4 qua ACP)

```mermaid
sequenceDiagram
    actor U as Vibe coder
    participant UI as Màn Review
    participant NT as Notes
    participant ST as .dark/notes
    participant AC as ACP client
    participant AG as Agent

    U->>UI: "/" tại dòng hoặc hunk, gõ ghi chú
    UI->>NT: add(file, dòng, nội dung)
    NT->>ST: Lưu ghi chú
    U->>UI: Gửi ghi chú cho agent
    alt Có agent ACP (M4)
        NT->>AC: Prompt gồm các ghi chú kèm vị trí file:dòng
        AC->>AG: session/prompt
        AG-->>AC: Cập nhật tin nhắn, file đang sửa
        AC-->>UI: Mở màn Vibe
    else Chế độ đi kèm (M2)
        NT-->>U: Markdown có vị trí file:dòng (clipboard hoặc file)
        U->>AG: Dán vào agent ở cửa sổ khác
    end
```

## 4. Vibe qua ACP (M4)

```mermaid
sequenceDiagram
    actor U as Vibe coder
    participant UI as Màn Vibe
    participant LM as Agent lifecycle
    participant AC as ACP client
    participant AG as Agent (process con)
    participant RV as Review engine
    participant G as git

    U->>UI: v, gõ task
    UI->>LM: Cần agent đang chọn
    alt Agent chưa chạy
        LM->>AG: Khởi động (ẩn cửa sổ trên Windows)
        AC->>AG: initialize, session/new hoặc tiếp tục phiên cũ
    end
    RV->>G: Tạo snapshot cho lượt này
    AC->>AG: session/prompt (task)
    loop Cho tới khi agent xong lượt
        AG-->>AC: Cập nhật: tin nhắn, kế hoạch, lệnh, file
        AC-->>UI: Vẽ lại dòng thay đổi, log ghi ra đĩa
        opt Agent xin quyền chạy lệnh hoặc sửa file
            AC-->>UI: Hỏi người dùng
            U->>UI: Cho phép / từ chối
            UI->>AC: Trả lời
            AC->>AG: Kết quả
        end
        opt Agent ghi file qua client
            AG->>AC: Ghi nội dung file
            AC-->>UI: Hiện hunk để duyệt
            U->>UI: Giữ / bỏ
            UI->>AC: Ghi hoặc từ chối
        end
    end
    AC-->>UI: Hết lượt
    UI->>RV: Mở màn Review so với snapshot của lượt
    UI-->>U: Review những gì agent vừa làm
```

## 5. Nhường màn hình cho agent CLI (M2, bản terminal)

```mermaid
sequenceDiagram
    actor U as Vibe coder
    participant D as dark (terminal)
    participant AG as Agent CLI (giao diện gốc)
    participant RV as Review engine
    participant G as git

    U->>D: v
    D->>RV: Tạo snapshot
    RV->>G: Ghi ref snapshot
    D->>D: Rời alternate screen, trả terminal về bình thường
    D->>AG: Chạy agent CLI (tiếp tục phiên cũ nếu có)
    U->>AG: Vibe như bình thường
    U->>AG: Thoát agent
    AG-->>D: Process kết thúc
    D->>D: Vào lại alternate screen
    D->>RV: Tải diff so với snapshot
    D-->>U: Màn Review với những gì agent vừa sửa
```

## 6. Khởi động lại agent khi phình RAM (M4)

```mermaid
sequenceDiagram
    participant LM as Agent lifecycle
    participant AG as Agent
    participant AC as ACP client
    participant UI as Giao diện

    loop Mỗi vài giây
        LM->>LM: Đo RAM cả cây process của agent
    end
    LM->>LM: Vượt ngưỡng restart_above_mb
    LM->>AC: Chờ agent hết lượt hiện tại
    AC-->>LM: Đã rảnh
    LM->>AG: Tắt agent
    LM->>AG: Khởi động lại
    AC->>AG: initialize, tiếp tục phiên cũ
    LM-->>UI: Thông báo ngắn: đã khởi động lại agent
```

## 7. Chạy test (M2)

```mermaid
sequenceDiagram
    actor U as Vibe coder
    participant UI as Giao diện
    participant T as Test runner

    U->>UI: !
    UI->>T: Chạy lệnh test trong cấu hình (ẩn cửa sổ trên Windows)
    loop Output
        T-->>UI: stdout/stderr (chỉ giữ phần cần hiển thị)
    end
    T-->>UI: Exit code
    alt Có lỗi
        UI-->>U: Lỗi đầu tiên và vị trí file:dòng
        opt Có agent
            U->>UI: Gửi lỗi cho agent
        end
    else Tất cả đạt
        UI-->>U: Ẩn panel lỗi
    end
```
