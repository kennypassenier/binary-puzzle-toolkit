# Corrections

Live-found faults, one record each, per the correction form in
`~/Projects/dev-procedure/FORM_PROTOCOL.md` §8.

## fix-1 · rename-retry: the Windows rename retry never runs for a locked file

Status: approved by Kenny 2026-09-26 ("Klopt"). Test first: commit
2cb74ca, red on the Windows CI job with `Access is denied. (os error 5)`
(run 36252070265). Fixed in the following commit.

**Closed 2026-09-26 by measurement** (field 7), on Kenny's Windows 11 PC
with the v1.1.0 release zip (SHA256 checked, `bpt 1.1.0 (ca0f918)`):
a lock released after 50 ms and after 100 ms both let the write
through with the new content, where 1.0.0 had left `stale`; a lock
held throughout fails with exit 2 after 250 ms instead of 81 ms, which
is the back-off running; no `.tmp` survives the next clean run.

1. **What went wrong.** `bpt solve --out out.txt` with `out.txt` held
   open by another process fails with `Access is denied. (os error 5)`
   under every share mode tried (None, Read, ReadWrite), measured on
   Windows 11 build 26200 with the v1.0.0 release binary. The retry in
   `bpt/src/atomic.rs` only fires on error 32 (sharing violation), so it
   never runs; a lock released 100 ms later still fails the run.
2. **Which gate let it through.** The Windows checklist, written before
   the merge and never run; CI can only reach the sharing-violation path
   through a test that asserts which error code Windows returns, and no
   test holds a real lock on the destination.
3. **Where else.** Searched every project for the same property, a
   retry keyed on one raw Windows error code:
   `rg -n --type rust -g '!target' 'SHARING_VIOLATION|raw_os_error\(\) == Some\(32\)|RENAME_ATTEMPTS|rename_with_retry' ~/Projects`.
   One other hit: `newsflash/newsflash-win/src/installer.rs:75`, a copy
   (not a rename) over a running exe, where code 32 was seen live on
   2026-09-24. Different operation, observed rather than assumed, so not
   the same fault as far as measured.
4. **Prevention.** Treat error 5 as transient when the destination is
   not a directory (the directory case is why 5 was excluded; see the
   comment on `is_transient`). A Windows-only test holds the destination
   with `share_mode(0)`, releases it after 50 ms, and asserts success.
5. **Cost.** A genuinely unwritable destination now waits 200 ms (four
   back-offs of 20, 40, 60, 80 ms) before failing. No new dependency:
   `std::os::windows::fs::OpenOptionsExt` is in std.
6. **Enforcement.** Code: the test runs on the Windows CI job on every
   push.
7. **Measurement.** Section 4 of `docs/solve/WINDOWS_TEST_CHECKLIST.md`
   rerun against the first release that carries the fix, with a lock
   released after 100 ms: expect exit 0.
8. **Fallback.** If a released lock still fails, drop the retry
   entirely and keep the error message, which already names the file and
   the remedy.
9. **Review.** At the next Windows checklist run.
