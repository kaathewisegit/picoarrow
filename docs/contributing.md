# Contributing

See [`architecture.md`][./architecture.md] for an overview of how this project is structured.

Testing and linting is done via the `check.sh` script.  `picoarrow` has a number of features.  `sh check.sh` tests all of them individually.  `sh check.sh --full` tests all possible combinations of features and runs in CI, but takes more time.
