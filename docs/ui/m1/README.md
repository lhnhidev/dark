# M1 Review terminal recordings

These recordings were captured from the Linux x86_64 musl release binary in a real PTY. Run `python3 scripts/capture-review.py /absolute/path/to/dark` on Linux to reproduce them. The script creates a temporary Git repository with 30 tracked files, 3,000 changed lines, two staged files, and one untracked file. The repository and snapshot time are temporary; the time in the capture is the actual capture time.

For each size, `.ansi` is the original PTY output before exit; `.txt` and `.html` are representations of that output. The HTML renders captured truecolor ANSI attributes and is not a screenshot. Child-process RSS is sampled separately from dark and is not added to or substituted for its RAM budget.

During this capture, the sampled peak RSS was 815,104 bytes for dark and 724,992 bytes for its child process tree at 160×44; at 80×24 it was 806,912 and 4,087,808 bytes respectively. Sampling can miss short-lived processes and is not an idle measurement or an operating-system peak counter.

The captured 160×44 view uses split diff; 80×24 uses unified diff and abbreviated tabs. Compared with the reference image, the pane borders surround both panes, the commit box is disabled until M3, and AI notes are absent until M5. At 80×24, long file paths are abbreviated to their distinguishing suffix and the available footer hints are clipped to leave room for review progress.
