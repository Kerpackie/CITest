Contributing and Releases

This project uses Conventional Commits to automate versioning and releases.

When you push to main, the CI pipeline analyzes your commit messages to determine the next version number.

How to format your commits

fix: ... -> Triggers a Patch release (0.1.0 -> 0.1.1)

Example: fix: correct modbus CRC calculation

feat: ... -> Triggers a Minor release (0.1.0 -> 0.2.0)

Example: feat: add support for writing multiple registers

feat!: ... -> Triggers a Major release (0.1.0 -> 1.0.0)

Example: feat!: change command line arguments structure

The Release Process

Push your changes to main.

A "Release PR" will be automatically created by the release-plz bot.

This PR updates Cargo.toml version.

This PR updates CHANGELOG.md.

Review the PR. When you Merge it:

A new GitHub Release is created.

Git Tag is pushed.

Binaries for Windows, Linux, and macOS are compiled and attached to the release.