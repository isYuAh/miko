# Contributing to Miko

First off, thank you for considering contributing to Miko! It's people like you that make Miko such a great tool.

Following these guidelines helps to communicate that you respect the time of the developers managing and developing this open source project. In return, they should reciprocate that respect in addressing your issue, assessing changes, and helping you finalize your pull requests.

## Code of Conduct

This project and everyone participating in it is governed by the [Miko Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code. Please report unacceptable behavior.

## How Can I Contribute?

### Reporting Bugs

This is one of the most helpful ways to contribute. Before creating a bug report, please check a few things:

1.  **Search the existing issues** to see if the bug has already been reported.
2.  If you don't find an existing issue, **create a new one**.
3.  Please include as many details as possible in your bug report. A good bug report is not a "It doesn't work" message. It should include:
    *   A clear and descriptive title.
    *   The version of Miko you are using.
    *   A step-by-step description of how to reproduce the bug.
    *   Code snippets, error messages, and logs.

### Suggesting Enhancements

If you have an idea for a new feature or an improvement to an existing one, we'd love to hear about it. Please create an issue and:

*   Use a clear and descriptive title.
*   Provide a detailed description of the enhancement, why it's needed, and how it would work.
*   Explain the use case and why this enhancement would be valuable to Miko users.

### Your First Code Contribution

Unsure where to begin contributing to Miko? You can start by looking through `good first issue` and `help wanted` issues.

### Pull Requests

The process for submitting a pull request is as follows:

1.  **Fork the repository** and create your branch from `main`.
2.  **Clone your fork** to your local machine: `git clone https://github.com/your-username/miko.git`
3.  **Create a new branch** for your changes: `git checkout -b name-of-your-feature-or-fix`
4.  **Make your changes**.
5.  **Run tests** to ensure everything is still working correctly: `cargo test --all-features --all-targets`
6.  This project uses `pre-commit` for code formatting and linting. Please make sure to install and run it before committing.
    ```bash
    # Install pre-commit hooks
    pre-commit install
    ```
7.  **Commit your changes**. Use a clear and descriptive commit message.
8.  **Push your branch** to your fork on GitHub: `git push origin name-of-your-feature-or-fix`
9.  **Open a pull request** to the `main` branch of the original Miko repository.
10. **Link to any relevant issues** in your pull request description.

## Licensing

By contributing to Miko, you agree that your contributions will be licensed under the MIT license.
