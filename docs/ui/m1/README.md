# M1 Review terminal recordings

These recordings were captured from the Linux x86_64 musl release binary in a real PTY. Run `python3 scripts/capture-review.py /absolute/path/to/dark` on Linux to reproduce them. The script creates a temporary Git repository with 30 tracked files, 3,000 changed lines, two staged files, and one untracked file. The repository and snapshot time are temporary; the time in the capture is the actual capture time.

The captured application was built from revision `7cbb0ff`. Its executable was 840,568 bytes, with SHA-256 `73deafd6f148df19a69a48465d53a20ba95cd2ebfcd1c1877fa5824649cddd5b`.

For each size, `.ansi` is the original PTY output before exit; `.txt` and `.html` are representations of that output. The HTML renders captured truecolor ANSI attributes and is not a screenshot. Child-process RSS is sampled separately from dark and is not added to or substituted for its RAM budget.

During this capture, the sampled peak RSS was 950,272 bytes for dark and 0 bytes for its child process tree at 160×44; at 80×24 it was 880,640 and 4,976,640 bytes respectively. Zero means no child was observed, not that no child ran. Sampling can miss short-lived processes and is not an idle measurement or an operating-system peak counter.

The captured 160×44 view uses split diff; 80×24 uses unified diff and abbreviated tabs. The layout follows UI_SPEC 1.1: pane titles sit on their own `panel` row, only the active pane (the file list at startup) is outlined in `accent`, and the shared middle column separates the panes. Code in the diff is syntax-highlighted with the GitHub Dark Default token colors of UI_SPEC §2 (here: `export`/`const` as keywords, quoted text as strings).

Intentional differences from the reference image:

- AI notes under hunks are absent until M5; the commit area is disabled until M3 (dimmed label, `Có ở M3` input and hint without key boxes). Below 32 rows the input is one line instead of a 3-row box.
- Groups have no blank line between them, to keep list rows on short terminals.
- The fixture's changes start at line 1, so hunk headers carry no function context; headers keep git's context when git provides it (covered by tests).
- The tab bar adds `· so với HEAD` after the snapshot time, and the status bar adds `a accept`/`x reject` and shows `u hoàn tác` (M1 restores to session start, not an AI turn).
- At 80×24, directories are dropped and file names are cut in the middle keeping the distinguishing tail (`m…01.ts`); the tab bar loses project information and the trailing footer hints yield to review progress.
