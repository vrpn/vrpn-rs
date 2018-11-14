# Contribution Guidelines and Rules

This will be fleshed out further once I hear back on appropriate license
and attribution information for a very excellent template.
Until then, see it directly for general ideas and hints: https://gist.github.com/PurpleBooth/b24679402957c63ec426

## General Guidelines

There are plenty of valid reasons why someone might not be able
to follow all of the guidelines in this section, and that's OK,
especially for new contributors or those new to open source entirely.
Just let us know and we'll figure out a way to help you get involved successfully.

> Important note: Unlike the guidelines here, the Code of Conduct,
> which appears at the end of this document, is **not** optional,
> and applies in its entirety to anyone involved in the project,
> for the safety and comfort of all.

### Pull/Merge Requests

- If you're considering starting work on a large change that you'd like to contribute,
  it is recommended to first open an issue before you start,
  to begin a discussion and help smooth the acceptance of your contribution.

- Please make sure to run `cargo fmt` before each commit,
  so that you only commit things that are cleanly styled.
  Consistent, machine-performed formatting improves readability and makes it easier for others to contribute.
  It also makes it easier to review changes.

- Avoid including whitespace or other formatting changes to unrelated code when committing.
  The `git add -p` command or the "stage selected lines/hunks" feature of various Git GUIs are
  great ways of making sure you only stage and commit the changes that you mean to.
  Relatedly, `git commit -v` (if you commit from the command line) can be a great help
  in making sure you aren't committing things you don't mean to,
  by showing the diff you're committing in your commit message editor.
  (This can even be set system-wide in `git config --global commit.verbose true`
  if you find it as life-changing as many others have.
  Thanks to @emilyst and her community for this tip! [1][emilyst-1] [2][emilyst-2])

- If you can, before submitting a pull/merge request,
  try building and even running the tests (`cargo test`),
  and if you touched any code that has or should have documentation,
  run `cargo doc --open` and see if it looks OK.

- We work to try to keep the code free of warnings -
  please help by making sure your changes build cleanly (and pass all tests),
  or at least don't add new warnings.
  If a warning stumps you, just mention it in the request so we can figure it out together.

[emilyst-1]: https://twitter.com/emilyst/status/1038850409140346880
[emilyst-2]: https://twitter.com/emilyst/status/1039205453010362368

### Issues

Constructive issues are a valued form of contribution.
Please try to include any relevant information (whether it is a request for improvement or a bug report).
We'll try to respond promptly, but there is no guarantee or warranty (as noted in the license),
absent any externally-arranged consulting or support contract.

Since this is a library whose audience is software developers,
bug reports should include:

- details about your build enviroment
  - architecture
  - Rust version
  - dependency versions (your `Cargo.lock`)
- associated code
  - for build errors, the consuming code
  - for logic/execution errors, a new (failing) test case is ideal,
    otherwise a description of expected and actual behavior
  - if you cannot disclose your code, or even if you can,
    an "artificial", minimally-sized example can be very valuable.

# Contributor Covenant Code of Conduct

## Our Pledge

In the interest of fostering an open and welcoming environment, we as
contributors and maintainers pledge to making participation in our project and
our community a harassment-free experience for everyone, regardless of age, body
size, disability, ethnicity, sex characteristics, gender identity and expression,
level of experience, education, socio-economic status, nationality, personal
appearance, race, religion, or sexual identity and orientation.

## Our Standards

Examples of behavior that contributes to creating a positive environment
include:

* Using welcoming and inclusive language
* Being respectful of differing viewpoints and experiences
* Gracefully accepting constructive criticism
* Focusing on what is best for the community
* Showing empathy towards other community members

Examples of unacceptable behavior by participants include:

* The use of sexualized language or imagery and unwelcome sexual attention or
  advances
* Trolling, insulting/derogatory comments, and personal or political attacks
* Public or private harassment
* Publishing others' private information, such as a physical or electronic
  address, without explicit permission
* Other conduct which could reasonably be considered inappropriate in a
  professional setting

## Our Responsibilities

Project maintainers are responsible for clarifying the standards of acceptable
behavior and are expected to take appropriate and fair corrective action in
response to any instances of unacceptable behavior.

Project maintainers have the right and responsibility to remove, edit, or
reject comments, commits, code, wiki edits, issues, and other contributions
that are not aligned to this Code of Conduct, or to ban temporarily or
permanently any contributor for other behaviors that they deem inappropriate,
threatening, offensive, or harmful.

## Scope

This Code of Conduct applies both within project spaces and in public spaces
when an individual is representing the project or its community. Examples of
representing a project or community include using an official project e-mail
address, posting via an official social media account, or acting as an appointed
representative at an online or offline event. Representation of a project may be
further defined and clarified by project maintainers.

## Enforcement

Instances of abusive, harassing, or otherwise unacceptable behavior may be
reported by contacting the project team at ryan.pavlik at collabora dot com. All
complaints will be reviewed and investigated and will result in a response that
is deemed necessary and appropriate to the circumstances. The project team is
obligated to maintain confidentiality with regard to the reporter of an incident.
Further details of specific enforcement policies may be posted separately.

Project maintainers who do not follow or enforce the Code of Conduct in good
faith may face temporary or permanent repercussions as determined by other
members of the project's leadership.

## Attribution

This Code of Conduct is adapted from the [Contributor Covenant][homepage], version 1.4,
available at https://www.contributor-covenant.org/version/1/4/code-of-conduct.html

[homepage]: https://www.contributor-covenant.org

---

# Copyright and License for this CONTRIBUTING.md file

For this file only:

> General Guidelines section: Written by Ryan Pavlik. Copyright 2018 Collabora, Ltd.
>
> Code of Conduct: adapted (directly - no changes other than inserted email address)
> from the [Contributor Covenant][homepage], version 1.4,
> available at https://www.contributor-covenant.org/version/1/4/code-of-conduct.html
>
> For this entire file:
>
> SPDX-License-Identifier: CC-BY-4.0
