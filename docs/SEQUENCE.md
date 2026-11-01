# dark — Sơ đồ tuần tự

> Phiên bản: 0.3 · Cập nhật: 01/11/2026
> Liên quan: [ARCHITECTURE.md](ARCHITECTURE.md) · [FEATURES.md](FEATURES.md) · [UI_SPEC.md](UI_SPEC.md) · [PLANNING.md](PLANNING.md)

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
    else Bấm "x" (bỏ)
        U->>UI: x
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

## 3. Sửa code ngay trong dark (M2)

```mermaid
sequenceDiagram
    actor U as Vibe coder
    participant UI as Màn Review
    participant ED as Editor gọn
    participant FS as File
    participant RV as Review engine
    participant G as git

    U->>UI: Enter tại một hunk
    UI->>ED: open(file, dòng của hunk)
    ED->>FS: Đọc file, ghi nhận thời điểm sửa đổi
    alt File nhị phân, không phải UTF-8 hoặc quá lớn
        ED-->>U: Chỉ xem, gợi ý bấm e để mở editor ngoài
    else File bình thường
        ED-->>U: Chế độ sửa, con trỏ đúng dòng
        U->>ED: Gõ, xóa, chọn chữ, copy/paste, Ctrl-Z
        U->>ED: Ctrl-S
        ED->>FS: Kiểm tra file có bị đổi từ bên ngoài không
        alt Đã bị agent hoặc editor khác đổi
            ED-->>U: Hỏi giữ bản nào, không ghi đè âm thầm
        else Không đổi
            ED->>FS: Ghi file tạm rồi đổi tên (giữ LF/CRLF, BOM)
            ED->>RV: Đọc lại diff của file
            RV->>G: git diff <snapshot> -- <file>
            G-->>RV: Patch mới
            RV-->>UI: Cập nhật hunk
        end
        U->>ED: Esc
        ED-->>UI: Quay lại màn Review (hỏi nếu chưa lưu)
    end
```

## 4. Ghi chú của người dùng gửi cho agent (M5)

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
    alt Có agent ACP
        NT->>AC: Prompt gồm các ghi chú kèm vị trí file:dòng
        AC->>AG: session/prompt
        AG-->>AC: Cập nhật tin nhắn, file đang sửa
        AC-->>UI: Mở màn Vibe
    else Chế độ đi kèm, chưa có ACP
        NT-->>U: Markdown có vị trí file:dòng (clipboard hoặc file)
        U->>AG: Dán vào agent ở cửa sổ khác
    end
```

## 5. Vibe qua ACP (M5)

```mermaid
sequenceDiagram
    actor U as Vibe coder
    participant UI as Màn Vibe
    participant LM as Agent lifecycle
    participant AC as ACP client
    participant AG as Agent (process con)
    participant RV as Review engine
    participant G as git

    U->>UI: Tab Vibe, ô 1, gõ task
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

## 6. Gọi agent CLI từ tab Vibe (M3, bản terminal)

```mermaid
sequenceDiagram
    actor U as Vibe coder
    participant D as dark (terminal)
    participant AG as Agent CLI (giao diện gốc)
    participant RV as Review engine
    participant G as git

    U->>D: Tab Vibe, chọn agent CLI, Enter
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

## 7. Khởi động lại agent khi phình RAM (M5)

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

## 8. Chạy test (M3)

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

## 9. Stage và commit từ panel Review (M3)

```mermaid
sequenceDiagram
    actor U as Vibe coder
    participant P as Panel danh sách
    participant GT as Git
    participant G as git

    U->>P: Space trên một file (hoặc s: stage tất cả)
    P->>GT: stage(file)
    GT->>G: git add -- <file>
    G-->>GT: OK / lỗi
    GT->>G: git status --porcelain=v2
    G-->>GT: Trạng thái mới
    GT-->>P: File chuyển sang nhóm Staged
    U->>P: c, gõ commit message, Enter
    P->>GT: commit(message)
    GT->>G: git commit -m <message>
    alt Thành công
        G-->>GT: hash commit
        GT-->>P: Làm mới danh sách, cập nhật tab Git
    else Lỗi (hook, chưa có gì để commit)
        G-->>GT: Thông báo lỗi
        GT-->>U: Hiện lỗi rõ ràng, giữ nguyên message
    end
```

## 10. Ghi chú AI cho từng hunk qua skill dark-notes (M5)

```mermaid
sequenceDiagram
    actor U as Vibe coder
    participant SK as Skill installer
    participant AG as Agent
    participant NS as .dark/notes
    participant NT as Notes
    participant UI as Tab Review

    U->>SK: Đồng ý cài skill dark-notes (hỏi một lần)
    SK->>AG: Ghi SKILL.md vào thư mục skill của agent
    Note over AG: Trong một lượt vibe bình thường
    AG->>AG: Sửa code
    AG->>NS: Cuối lượt: ghi JSON tóm tắt từng hunk (file, dòng, tóm tắt, cảnh báo)
    UI->>NT: Mở Review cho lượt vừa xong
    NT->>NS: Đọc ghi chú của snapshot
    NT->>NT: Ghép với hunk theo hash nội dung
    NT-->>UI: Ghi chú dưới từng hunk
    UI-->>U: Hunk đang chọn có viền accent và phím a · x · Enter · ?
```

## 11. Lưu API key ở tab Models (M6)

```mermaid
sequenceDiagram
    actor U as Vibe coder
    participant UI as Tab Models
    participant KS as Credential store
    participant OS as Windows Credential Manager
    participant API as Nhà cung cấp model

    U->>UI: Chọn nhà cung cấp, nhập API key (hiển thị ••••)
    U->>UI: t (kiểm tra kết nối)
    UI->>API: Request nhỏ với key
    alt Hợp lệ
        API-->>UI: OK
        UI-->>U: ✓ kết nối thành công · N ms
    else Lỗi
        API-->>UI: Lỗi
        UI-->>U: ✗ lý do (không in key)
    end
    U->>UI: Ctrl-S
    UI->>KS: save(provider, key)
    KS->>OS: CredWriteW
    OS-->>KS: OK
    KS-->>UI: Đã lưu
    Note over UI,KS: Key không bao giờ ghi ra file cấu hình hay log
```
