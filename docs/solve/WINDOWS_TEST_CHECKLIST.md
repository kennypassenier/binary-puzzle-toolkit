# Windows runtime checklist

`bpt` is **build-verified** on Windows: CI compiles it and runs the full
test suite on `windows-latest` on every push. This checklist is the
runtime half: the released binaries driven on a real Windows desktop.

Rewritten 2026-09-26 for the merged toolkit. The first version named the
pre-merge `binsolve.exe` and `binsolve-tui`, which no longer exist, and
expected `solve` to print a grid and a statistics line; `bpt solve`
prints only the solution line.

Run it against the release archive, not a local build, so the binary
under test is the one people download:

```powershell
gh release download v1.0.0 -R kennypassenier/binary-puzzle-toolkit -p '*windows*' -p SHA256SUMS
Expand-Archive bpt-v1.0.0-x86_64-pc-windows-msvc.zip .
cd bpt-v1.0.0-x86_64-pc-windows-msvc
$p = "1..0....00.1.00..1......00.1...1..00"
```

Windows PowerShell 5.1 note: `>` there writes UTF-16 and `2>` wraps each
stderr line in a PowerShell error record. To compare bytes, redirect
through `cmd /c "... > file"` or use PowerShell 7.

## 1 · Basic solve

```powershell
.\bpt.exe --version
.\bpt.exe solve $p
```

- [ ] Prints `bpt 1.0.0 (62f971b)`, then the solution line
      `101010010011100101011010001101110100`.

## 2 · Exit codes

```powershell
.\bpt.exe solve $p; $LASTEXITCODE
.\bpt.exe solve ("000" + "." * 33); $LASTEXITCODE
.\bpt.exe solve; $LASTEXITCODE
```

- [ ] `0`, then `1` with a `#contradiction:` line, then `2` with a
      message naming the remedy and an example.

## 3 · CRLF input

Save two copies of `$p` in Notepad (CRLF line endings) as `puzzles.txt`.

```powershell
.\bpt.exe solve --file .\puzzles.txt
```

- [ ] Two solution lines, no parse errors.

## 4 · Atomic output and a locked destination

```powershell
.\bpt.exe solve --file .\puzzles.txt --out .\out.txt
cmd /c ".\bpt.exe solve --file .\puzzles.txt > stdout.txt"
(Get-FileHash out.txt).Hash -eq (Get-FileHash stdout.txt).Hash
```

- [ ] `True`, and no `out.txt.tmp` in the directory.

Now hold `out.txt` open (Excel, or
`$fs = [IO.File]::Open("$PWD\out.txt", 'Open', 'Read', 'None')`) and run
the `--out` command again.

- [ ] It fails with exit 2 and a message that names the file and says
      what to do. `out.txt` keeps its old content. An `out.txt.tmp` is
      left behind by design, and the next successful run removes it.

## 5 · Paths with spaces

```powershell
mkdir "$env:TEMP\test map"
.\bpt.exe solve --file .\puzzles.txt --out "$env:TEMP\test map\out.txt"
```

- [ ] Writes to the quoted path without error.

## 6 · Explain output redirection

```powershell
cmd /c ".\bpt.exe solve --explain $p 2> trace.txt"
```

- [ ] `trace.txt` contains numbered `step N:` lines; standard output
      carries only the solution line.

## 7 · Console rendering and the TUI (needs a person at the screen)

In Windows Terminal, and if possible also in the old `conhost` console:

```powershell
.\bpt.exe solve --explain $p
.\bpt.exe watch $p
.\bpt-tui.exe $p
```

- [ ] The `—` dash in the explain trace renders as a dash, not `?` or
      mojibake. Note whether it needed `chcp 65001`.
- [ ] Box-drawing borders render as lines, not as garbage.
- [ ] Colours are visible (givens bold, deduced cells cyan, the current
      cell highlighted).
- [ ] Keys work: space pauses, `←`/`→` step, `+`/`-` change speed,
      `Home`/`End` jump, `q` quits.
- [ ] After quitting, the terminal is restored: no leftover alternate
      screen, cursor visible, typing works normally.

## Sign-off

Sections 1 to 6 were run on 2026-09-26 by Claude on Kenny's Windows 11
PC (build 26200, Windows PowerShell 5.1.26100), against the v1.0.0
release archive with its SHA256 checked, started from WSL through
Windows interop, so the processes ran as native Windows processes.

| Section | Result | Notes |
|---|---|---|
| 1 Basic solve | pass | |
| 2 Exit codes | pass | 0, 1, 2 as specified |
| 3 CRLF input | pass | file bytes checked: `0D 0A` between the lines |
| 4 Atomic write + lock | pass, with a finding | output identical; lock: exit 2, clear message, old content kept, `.tmp` removed by the next run. **Finding:** a locked destination reports `Access is denied (os error 5)`, never the sharing violation (32) the retry waits for, under every share mode tried (None, Read, ReadWrite). The bounded retry therefore never runs for a locked destination. |
| 5 Paths | pass | |
| 6 Explain redirection | pass | 22 `step N:` lines |
| 7 Console + TUI | open | needs Kenny at the screen; Windows stays beta by Kenny's decision of 2026-09-26 |

Section 4 rerun on 2026-09-26 against the v1.1.0 release: a lock
released after 50 ms or 100 ms no longer fails the write (fix-1, closed).

Signed off by: ______________  date: ____________
