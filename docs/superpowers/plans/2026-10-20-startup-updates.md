# Startup updates — Windows installed application

User decision: check once at startup, then offer download and installation.

- [x] Keep checks off the UI thread and skip portable builds.
- [x] Restrict downloads to this repository and verify signatures bound to version.
- [x] Ask through a native dialog; declining leaves the installation untouched.
- [x] Preserve Inno identity, directory and profile; restart after silent installation.
- [x] Configure GitHub signing secret and versioned installer/feed publication.
- [x] Complete formatting, Clippy, release build and final source review.
- [ ] Merge to main and publish v0.2.1 through native packaging CI.

No unit/functional suites or UI automation are run. Installed upgrade and restart
acceptance require a user-controlled Windows check. Other platforms remain manual.
