# Changelog: frost-interval

Changes to this package.

## Unreleased

- Created from the prototype `frost-domain-proto` 0.7.0 (design record
  revision 8, `docs/design/proposals/frozen-box.md`).
- Features `chrono`, `jiff` and `time` are now `chrono-0_4`, `jiff-0_2` and
  `time-0_3`.
- `no_std` with `alloc`; the `SystemTime` and `Instant` domains need `std`.
- The demonstration domains (`demo`) and the law checks (`laws`) moved from
  `src/` to `tests/support/`; they are no longer public.
- One test no longer asserts a time limit on `count()`.
