# Windows runtime checklist

binsolve is **build-verified** on Windows: CI compiles it and runs the
full test suite on `windows-latest` on every push. It is **not
runtime-verified** — no one has yet driven the built binaries by hand on
a real Windows desktop. Until this checklist is signed off, the honest
status of the Windows target is "compiles and tests green", not "done".

Run this on a Windows machine, from a normal PowerShell prompt.

## 1 · Build and basic solve

```powershell
cargo build --release
.\target\release\binsolve.exe "1..0....00.1.00..1......00.1...1..00"
```

- [ ] Prints a 6×6 grid, a statistics line, and the solution line
      `101010010011100101011010001101110100`.
- [ ] The grid, the `—` dash and the `·` characters render correctly —
      no `?`, no mojibake. (The console must be UTF-8; note whether it
      worked out of the box or needed `chcp 65001`.)

## 2 · Exit codes

```powershell
.\target\release\binsolve.exe "1..0....00.1.00..1......00.1...1..00"; $LASTEXITCODE
.\target\release\binsolve.exe ("000" + "." * 33); $LASTEXITCODE
.\target\release\binsolve.exe; $LASTEXITCODE
```

- [ ] Prints `0`, then `1` (contradictory puzzle), then `2` (usage
      error with a message naming the remedy).

## 3 · CRLF input

Create a batch file in Notepad (which writes CRLF line endings), with
two puzzle lines, then:

```powershell
.\target\release\binsolve.exe --file .\puzzles.txt
```

- [ ] Two solution lines, no parse errors. This is the case the
      `\r`-tolerance in the parser exists for.

## 4 · Atomic output and the sharing-violation retry

```powershell
.\target\release\binsolve.exe --file .\puzzles.txt --out .\out.txt
```

- [ ] `out.txt` matches what the same command prints without `--out`.
- [ ] No `.out.txt.tmp` left behind in the directory.

Now the interesting one — the retry path that cannot be exercised on
Linux at all. Open `out.txt` in an application that holds an exclusive
lock (Excel is the classic; Notepad usually does not lock), leave it
open, and run the same command again.

- [ ] Either it succeeds after a brief pause (the bounded retry did its
      job), or it fails with an error message that names the file and
      says what to do. It must NOT leave a truncated `out.txt` or a
      stray `.tmp` file.

## 5 · Paths with spaces and backslashes

```powershell
mkdir "C:\Users\$env:USERNAME\test map"
.\target\release\binsolve.exe --file .\puzzles.txt --out "C:\Users\$env:USERNAME\test map\out.txt"
```

- [ ] Writes to the quoted path without error.

## 6 · Explain output redirection

```powershell
.\target\release\binsolve.exe --explain "1..0....00.1.00..1......00.1...1..00" 2> trace.txt
```

- [ ] `trace.txt` contains numbered `step N:` lines; standard output
      still carries only the single solution line.

## 7 · The TUI

```powershell
cargo run --release -p binsolve-tui -- "1..0....00.1.00..1......00.1...1..00"
```

In Windows Terminal, and if possible also in the old `conhost` console:

- [ ] Box-drawing borders render as lines, not as garbage.
- [ ] Colours are visible (givens bold, deduced cells cyan, the current
      cell highlighted).
- [ ] Keys work: space pauses, `←`/`→` step, `+`/`-` change speed,
      `Home`/`End` jump, `q` quits.
- [ ] After quitting, the terminal is restored — no leftover alternate
      screen, cursor visible again, typing works normally.

## Sign-off

| Section | Result | Notes |
|---|---|---|
| 1 Build and solve | | |
| 2 Exit codes | | |
| 3 CRLF input | | |
| 4 Atomic write + retry | | |
| 5 Paths | | |
| 6 Explain redirection | | |
| 7 TUI | | |

Signed off by: ______________  date: ____________
