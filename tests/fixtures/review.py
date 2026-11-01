"""Create a reproducible Git review fixture without touching the caller's repository."""
from pathlib import Path
import subprocess


def create(root: Path) -> None:
    root.mkdir(parents=True, exist_ok=True)
    def git(*args):
        subprocess.run(["git", "-C", str(root), *args], check=True, stdout=subprocess.DEVNULL)
    git("init", "-q", "-b", "main")
    git("config", "user.name", "Dark Test")
    git("config", "user.email", "dark-test@example.invalid")
    for index in range(30):
        path = root / ("src/auth/session.ts" if index == 0 else f"src/module_{index:02}.ts")
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text("".join(f"export const value_{n:03} = 'old value {index}:{n}';\n" for n in range(100)), encoding="utf-8")
    git("add", ".")
    git("commit", "-qm", "Fixture baseline")
    for index in range(30):
        path = root / ("src/auth/session.ts" if index == 0 else f"src/module_{index:02}.ts")
        path.write_text("".join(f"export const value_{n:03} = '{'new' if n < 50 else 'old'} value {index}:{n}';\n" for n in range(100)), encoding="utf-8")
    git("add", "src/auth/session.ts", "src/module_01.ts")
    (root / "notes.md").write_text("Untracked review fixture\n", encoding="utf-8")
