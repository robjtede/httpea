# GitHub Actions parallel lint experiment

Measured on 2026-10-07. Keep the shared lint job and keep the Linux test job separate.

The chosen variant reduces median CI + Lint runner use from 171 to 125 seconds with cold caches (27%) and from 133 to 94 seconds with warm caches (29%). The required test check changes from 51 to 52 seconds with cold caches and from 34 to 37 seconds with warm caches. The test workflow is unchanged in the final diff. The shared lint job finishes before the test job in every measured sample.

The variant that also shares Linux test setup saves more runner time, but its warm required check takes 46 seconds (45–53), compared with 34 seconds (34–36) in the baseline. Do not retain that variant.

## Configuration and source controls

- Base revision: 815900767c5aea99049ee35a2c6ba01d83755e19.
- Controlled baseline: fcd32f8dd2bf06285382e2dea92db0832770c465.
- Shared lint setup: e74d7f5fff4141a3b7236ec895506b2c0d797bf3.
- Shared lint and Linux tests: d7acf9273d5147cadf6c47064cc2c15f7b589c22.
- The three benchmark revisions have identical crate source, Cargo manifests, Cargo.lock, Just recipes, and Nix inputs. Their differences are confined to workflow files.
- Cargo.lock Git blob: f45d2a0549a817b43c9e65d46de1c47e0287b7af. Crate tree: 1c8acdab57ea083cfbce927992164ae81ae0862b.
- All compatible jobs use the same ubuntu-latest Linux x64 runner class. Logs show runner 2.337.0 and Ubuntu 24.04 images 20260927.320.1 and 20261004.327.1. Both image versions occur across configurations. GitHub was rolling out the new image during the experiment; its exact image build is not pinned.
- All check jobs use nightly-2026-10-07: rustc 1.101.0-nightly (8d1a76430 2026-10-06), Cargo 1.101.0-nightly (f3865b2a4 2026-09-29). All action revisions and check flags remain fixed.
- Each configuration has three cold/warm pairs. Attempts 1/2, 3/4 and 5/6 use distinct cache namespaces. The namespace contains configuration, run ID, pair and job. Logs confirm every core job has a miss in each cold attempt and an exact hit in each warm attempt. Shared caches were not deleted.
- The cache action hashes all installed compilers. A temporary RUSTUP_HOME at /tmp/httpea-ci-rustup removes variation from the runner's preinstalled stable compiler. Only the pinned nightly is installed there. This setting, the nightly date, isolated cache keys, and temporary per-revision concurrency groups are removed from the final workflow.
- The original format, Clippy, docs and test commands are preserved. CI environment values from the setup action remain in effect, including CARGO_BUILD_WARNINGS=deny, CARGO_INCREMENTAL=0 and CARGO_PROFILE_DEV_DEBUG=0. Documentation retains RUSTDOCFLAGS=-D warnings.

The original source failed before useful benchmarking: nightly Cargo rejects deprecated hyphenated lint names and warns when the httpea package does not state its lint inheritance choice. The prerequisite commits use current lint names, assign lint groups a lower priority than individual lints, and add an empty package lint table to retain httpea's existing independent lint settings. These corrections are present in every compared revision. Warning checks are not disabled.

## Measurement definitions

All values are seconds. Summary cells show median (minimum–maximum), with n = 3 for each cache state.

Runner time is the sum of real jobs' completed_at minus started_at from the Actions jobs API. It includes runner setup, all check steps, post-job cache work, checkout cleanup and teardown. Reviewdog's extra check report has no runner steps and is excluded. The all total includes the CI and Lint workflows, including external-types and security. The core total includes format, Clippy, docs and Linux tests. The unchanged Benchmark and main-only Coverage workflows are outside this comparison.

Required test time is that job's runner duration, including post-job work. It is also the queue-free critical path to all four core checks: each configuration has independent jobs, and tests are the longest path. This assumes simultaneous runner assignment. It is a model of execution time, not the observed service wall clock.

Observed all-check time is the last core-job completion minus the earliest workflow run_started_at for the pair of CI/Lint runs. It includes admission, scheduling and queue delays. Job queue time is started_at minus created_at, reported separately. The JSON also retains admission time and all job timestamps. Queue differences do not count as runner-time savings.

Setup sum adds durations of runner setup, checkout, Rust installation/cache restoration, test-tool installation, Nix installation and Nix shell entry across core jobs. Concurrent setup and check steps can overlap in the shared-test variant. Post sum measures each core job from its first post step through job completion. The [timing data](ci-parallel-lint-experiment.json) contains every job's setup elapsed time, step durations, cache state, runner version and image version. API step times have one-second resolution, so zero means less than the reported resolution.

## Results

| Configuration               | Cache | Runner seconds: CI + Lint | Runner seconds: core | Required test / queue-free all checks | Lint elapsed     |
| --------------------------- | ----- | ------------------------- | -------------------- | ------------------------------------- | ---------------- |
| Separate jobs               | cold  | 171 (170–201)             | 130 (128–166)        | 51 (50–72)                            | 35 (32–47)       |
| Separate jobs               | warm  | 133 (126–138)             | 95 (92–95)           | 34 (34–36)                            | 23 (21–25)       |
| Shared lint setup           | cold  | 125 (119–128)             | 87 (83–94)           | 52 (49–53)                            | 38 (31–41)       |
| Shared lint setup           | warm  | 94 (92–99)                | 59 (56–65)           | 37 (35–37)                            | 22 (21–28)       |
| Shared lint and Linux tests | cold  | 93 (84–94)                | 55 (54–55)           | 55 (54–55)                            | included in test |
| Shared lint and Linux tests | warm  | 86 (84–89)                | 46 (45–53)           | 46 (45–53)                            | included in test |

| Configuration               | Cache | Setup sum  | Format  | Clippy     | Docs    | Tests      | Post sum   |
| --------------------------- | ----- | ---------- | ------- | ---------- | ------- | ---------- | ---------- |
| Separate jobs               | cold  | 71 (67–78) | 0 (0–0) | 14 (14–27) | 2 (2–2) | 14 (14–18) | 25 (23–42) |
| Separate jobs               | warm  | 74 (71–77) | 0 (0–1) | 2 (2–4)    | 1 (0–1) | 2 (2–3)    | 10 (9–12)  |
| Shared lint setup           | cold  | 43 (42–47) | 0 (0–0) | 14 (12–15) | 0 (0–1) | 17 (15–17) | 11 (11–13) |
| Shared lint setup           | warm  | 46 (45–48) | 0 (0–0) | 3 (2–3)    | 1 (1–2) | 3 (3–3)    | 5 (4–8)    |
| Shared lint and Linux tests | cold  | 39 (36–39) | 0 (0–1) | 17 (17–18) | 1 (1–1) | 10 (10–11) | 6 (5–7)    |
| Shared lint and Linux tests | warm  | 37 (37–39) | 0 (0–0) | 8 (8–8)    | 1 (1–2) | 5 (5–5)    | 3 (3–8)    |

## Measured runs

Every table row is one CI/Lint attempt pair. All listed core checks, external-types jobs and security jobs pass.

### Separate jobs

| Attempt / cache | Measured runs                                                                                                                                                | Setup sum | Fmt / Clippy / docs / tests | Post sum | Runner all / core | Required test | Observed all checks | Job queue range |
| --------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------: | --------------------------- | -------: | ----------------- | ------------: | ------------------: | --------------- |
| 1 / cold        | [CI](https://github.com/robjtede/httpea/actions/runs/37653300423/attempts/1), [Lint](https://github.com/robjtede/httpea/actions/runs/37653300412/attempts/1) |        78 | 0 / 27 / 2 / 14             |       42 | 201 / 166         |            72 |                 235 | 13–187          |
| 2 / warm        | [CI](https://github.com/robjtede/httpea/actions/runs/37653300423/attempts/2), [Lint](https://github.com/robjtede/httpea/actions/runs/37653300412/attempts/2) |        74 | 0 / 4 / 1 / 2               |       10 | 133 / 95          |            36 |                 142 | 55–122          |
| 3 / cold        | [CI](https://github.com/robjtede/httpea/actions/runs/37653300423/attempts/3), [Lint](https://github.com/robjtede/httpea/actions/runs/37653300412/attempts/3) |        71 | 0 / 14 / 2 / 14             |       25 | 170 / 130         |            51 |                  95 | 22–47           |
| 4 / warm        | [CI](https://github.com/robjtede/httpea/actions/runs/37653300423/attempts/4), [Lint](https://github.com/robjtede/httpea/actions/runs/37653300412/attempts/4) |        71 | 0 / 2 / 0 / 3               |       12 | 126 / 92          |            34 |                  40 | 3–4             |
| 5 / cold        | [CI](https://github.com/robjtede/httpea/actions/runs/37653300423/attempts/5), [Lint](https://github.com/robjtede/httpea/actions/runs/37653300412/attempts/5) |        67 | 0 / 14 / 2 / 18             |       23 | 171 / 128         |            50 |                  56 | 2–3             |
| 6 / warm        | [CI](https://github.com/robjtede/httpea/actions/runs/37653300423/attempts/6), [Lint](https://github.com/robjtede/httpea/actions/runs/37653300412/attempts/6) |        77 | 1 / 2 / 1 / 2               |        9 | 138 / 95          |            34 |                  41 | 3–4             |

### Shared lint setup

| Attempt / cache | Measured runs                                                                                                                                                | Setup sum | Fmt / Clippy / docs / tests | Post sum | Runner all / core | Required test | Observed all checks | Job queue range |
| --------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------: | --------------------------- | -------: | ----------------- | ------------: | ------------------: | --------------- |
| 1 / cold        | [CI](https://github.com/robjtede/httpea/actions/runs/37653942382/attempts/1), [Lint](https://github.com/robjtede/httpea/actions/runs/37653942344/attempts/1) |        42 | 0 / 14 / 1 / 17             |       11 | 125 / 87          |            49 |                 121 | 11–79           |
| 2 / warm        | [CI](https://github.com/robjtede/httpea/actions/runs/37653942382/attempts/2), [Lint](https://github.com/robjtede/httpea/actions/runs/37653942344/attempts/2) |        48 | 0 / 3 / 1 / 3               |        8 | 99 / 65           |            37 |                  61 | 18–29           |
| 3 / cold        | [CI](https://github.com/robjtede/httpea/actions/runs/37653942382/attempts/3), [Lint](https://github.com/robjtede/httpea/actions/runs/37653942344/attempts/3) |        47 | 0 / 15 / 0 / 17             |       13 | 128 / 94          |            53 |                  59 | 3–3             |
| 4 / warm        | [CI](https://github.com/robjtede/httpea/actions/runs/37653942382/attempts/4), [Lint](https://github.com/robjtede/httpea/actions/runs/37653942344/attempts/4) |        46 | 0 / 2 / 2 / 3               |        5 | 94 / 59           |            37 |                  45 | 3–4             |
| 5 / cold        | [CI](https://github.com/robjtede/httpea/actions/runs/37653942382/attempts/5), [Lint](https://github.com/robjtede/httpea/actions/runs/37653942344/attempts/5) |        43 | 0 / 12 / 0 / 15             |       11 | 119 / 83          |            52 |                  58 | 2–3             |
| 6 / warm        | [CI](https://github.com/robjtede/httpea/actions/runs/37653942382/attempts/6), [Lint](https://github.com/robjtede/httpea/actions/runs/37653942344/attempts/6) |        45 | 0 / 3 / 1 / 3               |        4 | 92 / 56           |            35 |                  42 | 3–4             |

### Shared lint and Linux tests

| Attempt / cache | Measured runs                                                                                                                                                | Setup sum | Fmt / Clippy / docs / tests | Post sum | Runner all / core | Required test | Observed all checks | Job queue range |
| --------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------: | --------------------------- | -------: | ----------------- | ------------: | ------------------: | --------------- |
| 1 / cold        | [CI](https://github.com/robjtede/httpea/actions/runs/37654126604/attempts/1), [Lint](https://github.com/robjtede/httpea/actions/runs/37654126387/attempts/1) |        39 | 0 / 17 / 1 / 10             |        6 | 93 / 55           |            55 |                  84 | 28–28           |
| 2 / warm        | [CI](https://github.com/robjtede/httpea/actions/runs/37654126604/attempts/2), [Lint](https://github.com/robjtede/httpea/actions/runs/37654126387/attempts/2) |        37 | 0 / 8 / 1 / 5               |        3 | 86 / 46           |            46 |                  60 | 11–11           |
| 3 / cold        | [CI](https://github.com/robjtede/httpea/actions/runs/37654126604/attempts/3), [Lint](https://github.com/robjtede/httpea/actions/runs/37654126387/attempts/3) |        39 | 0 / 18 / 1 / 11             |        5 | 94 / 55           |            55 |                  60 | 2–2             |
| 4 / warm        | [CI](https://github.com/robjtede/httpea/actions/runs/37654126604/attempts/4), [Lint](https://github.com/robjtede/httpea/actions/runs/37654126387/attempts/4) |        39 | 0 / 8 / 1 / 5               |        8 | 89 / 53           |            53 |                  60 | 3–3             |
| 5 / cold        | [CI](https://github.com/robjtede/httpea/actions/runs/37654126604/attempts/5), [Lint](https://github.com/robjtede/httpea/actions/runs/37654126387/attempts/5) |        36 | 1 / 17 / 1 / 10             |        7 | 84 / 54           |            54 |                  67 | 6–6             |
| 6 / warm        | [CI](https://github.com/robjtede/httpea/actions/runs/37654126604/attempts/6), [Lint](https://github.com/robjtede/httpea/actions/runs/37654126387/attempts/6) |        37 | 0 / 8 / 2 / 5               |        3 | 84 / 45           |            45 |                  54 | 3–3             |

## Tradeoff and regression checks

The chosen job uses one checkout, one nightly installation with rustfmt, clippy and rust-docs, and one cache restoration. Format runs as a background step beside Clippy. Docs runs after Clippy and uses the same Cargo target directory. Its median cold check duration drops from 2 seconds to less than one reported second. Concurrent Clippy/docs was not benchmarked: less than a second of remaining docs work gives little benefit, while one target directory adds Cargo lock contention and separate directories lose artifact reuse and duplicate compilation.

The Linux experiment starts Clippy and tool installation in the background during Nix setup, then waits before docs and tests. It has no Clippy/docs overlap. Its warm Rust installation has a median of 15 seconds, compared with 11 seconds for the separate test control. Warm tests take 5 seconds, compared with 2 seconds in the baseline, and the combined check also waits for docs. Nix shell entry and cache post-work vary between samples. These costs explain the longer required-check path; they do not justify retaining the Linux consolidation. Logs show a build-directory lock inside the existing parallel Just test recipe, which runs nextest and doctests together. That recipe remains unchanged.

The chosen variant's required-check medians differ by 1 second cold and 3 seconds warm. Its separate test job is a control with identical setup and commands; the final test workflow is restored byte for byte. Three samples do not establish statistical equivalence, and hosted-runner image/network variation remains. The merged lint job never becomes the longest execution path.

## Status checks and preserved behavior

The active main ruleset requires only the GitHub Actions test context. The final workflow retains that name. No ruleset or branch protection was changed. The new lint job replaces the fmt, clippy and lint-docs runner jobs. The Clippy action continues to emit its separate clippy annotation check with the same reporter, token input, flags and existing failure settings.

The merged job has the union of the original lint permissions: contents: read and checks: write. The write permission remains limited to that job. Workflow triggers, concurrency, checkout credential settings, toolchain request, documentation flags, external-types, security, Benchmark and Coverage are preserved. The current repository has one Linux test job and no platform or MSRV matrix; the experiment does not remove coverage or add repeated matrix lint checks.

Docs uses a failure-aware condition so a failed Clippy step does not skip it. The final wait always observes the format result. No continue-on-error is added. A [runner failure probe](https://github.com/robjtede/httpea/actions/runs/37652586202) forces format, Clippy and docs failures in separate matrix entries. Each job fails, while its other intended checks and test step still run.

## Validation and limits

- All six attempts in each controlled configuration pass, including the existing security and external-types jobs.
- Runner 2.337.0 executes background and wait. The existing security workflow uses zizmor 1.30.1 and accepts the new syntax with its existing settings.
- Actionlint 1.7.12 rejects background/wait. Its [upstream support issue](https://github.com/rhysd/actionlint/issues/693) remains open. Actionlint is not an existing repository gate. No validator, rule or check was suppressed to get a pass.
- Local Nix/Just tests pass: 90 nextest tests and 41 doctests. The Just Clippy and documentation recipes pass. The exact CI documentation command also passes with its warning policy.
- The full command nix develop -c just toolchain=+nightly check passes its formatting, Clippy, YAML and TOML checks, then fails at cargo shear for the existing unused http dev-dependency in crates/http-request-target/Cargo.toml. That dependency remains unchanged; the check is not suppressed.
- The unchanged CodSpeed Performance Analysis check is also red. Its benchmark runner job succeeds, and the main ruleset does not require the analysis check. It is outside this CI setup experiment.
- Three cold and three warm samples are a small dataset. Queue ranges are large in some attempts. Image builds vary within the same runner class. The controlled results do not claim an improvement in GitHub scheduling or a precise production speedup.

The syntax is described in GitHub's [parallel-step announcement](https://github.blog/changelog/2026-06-25-actions-steps-can-now-be-run-in-parallel/) and [workflow syntax documentation](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax#jobsjob_idstepsbackground).

## Earlier observations excluded from the comparison

The unchanged [main Lint rerun](https://github.com/robjtede/httpea/actions/runs/36935415803/attempts/2) and [main CI rerun](https://github.com/robjtede/httpea/actions/runs/36935415874/attempts/2) establish the pre-edit failures. They are excluded because intended checks did not finish successfully.

The first corrected-source series used the runner's default Rustup directory. It showed stable compiler 1.98.1 or 1.99.0 in the cache fingerprint. Some intended warm jobs therefore missed their caches. These observations are retained in the JSON and linked below, but are excluded from the reported comparison. The cache column shows actual states.

| Attempt | Measured runs                                                                                                                                                | CI + Lint runner seconds | Actual core cache states                                       |
| ------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ | -----------------------: | -------------------------------------------------------------- |
| 1       | [CI](https://github.com/robjtede/httpea/actions/runs/37651781432/attempts/1), [Lint](https://github.com/robjtede/httpea/actions/runs/37651781490/attempts/1) |                      168 | test: miss; fmt: miss; lint-docs: miss; clippy: miss           |
| 2       | [CI](https://github.com/robjtede/httpea/actions/runs/37651781432/attempts/2), [Lint](https://github.com/robjtede/httpea/actions/runs/37651781490/attempts/2) |                      147 | test: exact-hit; lint-docs: miss; fmt: miss; clippy: exact-hit |
| 3       | [CI](https://github.com/robjtede/httpea/actions/runs/37651781432/attempts/3), [Lint](https://github.com/robjtede/httpea/actions/runs/37651781490/attempts/3) |                      176 | test: miss; fmt: miss; lint-docs: miss; clippy: miss           |
| 4       | [CI](https://github.com/robjtede/httpea/actions/runs/37651781432/attempts/4), [Lint](https://github.com/robjtede/httpea/actions/runs/37651781490/attempts/4) |                      158 | test: miss; fmt: miss; lint-docs: miss; clippy: exact-hit      |
| 5       | [CI](https://github.com/robjtede/httpea/actions/runs/37651781432/attempts/5), [Lint](https://github.com/robjtede/httpea/actions/runs/37651781490/attempts/5) |                      153 | test: miss; fmt: miss; clippy: miss; lint-docs: miss           |
| 6       | [CI](https://github.com/robjtede/httpea/actions/runs/37651781432/attempts/6), [Lint](https://github.com/robjtede/httpea/actions/runs/37651781490/attempts/6) |                      172 | test: miss; lint-docs: exact-hit; clippy: exact-hit; fmt: miss |

Two setup probes ([Lint](https://github.com/robjtede/httpea/actions/runs/37653169234), [CI](https://github.com/robjtede/httpea/actions/runs/37653170683)) were rejected before allocating jobs because runner.temp is not available in job-level env expressions. The controlled configuration uses the fixed temporary path instead. They have no runner measurements and do not enter the comparison.
