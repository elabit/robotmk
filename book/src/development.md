# Development

These are internal notes.

## Release

The release is triggered via the workflow `.github/workflows/release.yaml`. Before executing the
workflow the binaries
* the main branch needs to be in a sane state and
* the binaries should be manually tested.

TODO: Replace manual tests by integration tests.

The workflow can be triggered via the Action menu. Once the workflow is complete, a draft release is
available. Some basic sanity checks:
* Is the draft body correct?
* Is the title correct?
* Does `executables.zip` contain the correct binaries?

In order to use the new release with `Checkmk`, the `omd` package `robotmk` needs to be updated. The
earliest release of `Checkmk`, which contains this `omd` package is the daily `2023.11.14`. Note,
the bakery plugin is enterprise-only.
