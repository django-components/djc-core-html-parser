# Django Components Template Parser

A high-performance Rust-based template parser for [`django-components`](https://github.com/django-components/django-components), designed to parse Django template syntax into an Abstract Syntax Tree (AST) and compile it into callable Python functions.

## Overview

This package provides a fast, Rust-implemented parser for Django template syntax using the [Pest](https://pest.rs/) parsing library. This library has follow parts:

1. **tag_parser** - Turn `{% ... %}` or `<... />` into an AST using the grammar defined in `grammar.pest`
2. **tag_compiler** - Compile the AST into optimized, callable Python functions

The parser supports:

- Tag name (must be the first token, e.g. `{% my_tag ... %}`)
- Key-value pairs (e.g. `key=value`)
- Standalone values (e.g. `1`, `"my string"`, `val`)
- Spread operators (e.g. `...value`, `**value`, `*value`)
- Filters (e.g. `value|filter:arg`)
- Lists and dictionaries (e.g. `[1, 2, 3]`, `{"key": "value"}`)
- String literals (single/double quoted) (e.g. `"my string"`, `'my string'`)
- Numbers (e.g. `1`, `1.23`, `1e-10`)
- Variables (e.g. `val`, `key`)
- Translation strings (e.g. `_("text")`)
- Comments (e.g. `{# comment #}`)
- Self-closing tags (e.g. `{% my_tag / %}`)

## Development

### Prerequisites

- Rust (latest stable version)
- Python 3.8+
- [Maturin](https://github.com/PyO3/maturin) for building Python extensions

### Setup

1. Install Maturin:

   ```bash
   pip install maturin
   ```

2. Build and install the package in development mode:
   ```bash
   maturin develop
   ```

This will compile the Rust code and install the Python package in your current environment.

### Running tests

```bash
# Run Rust tests
cargo test

# Run specific Rust test
cargo test tag_parser::tests::test_list_spread_comments

# Run Python tests
python -m pytest tests/
```

## Developing django-components

If you're making changes to the parser, you should test that the updated parser still works with `django-components`.

To do that, you need to build the parser package and install it in your local fork of `django-components`.

1. Build the parser package:

   ```bash
   cd djc_core_template_parser
   maturin develop
   ```

2. Install `djc_core_template_parser` in django-components:
   ```bash
   cd ../django_components
   pip install -e ../djc_core_template_parser
   ```

## Publishing

There is a Github workflow to release the package. It runs when a new tag is pushed (AKA a new release is created).

The CI workflow compiles the package in many different environments, ensuring that this package can run across all major platforms.

Steps:

1. Bump version in `pyproject.toml`
2. Make a release on GitHub
3. The package will be automatically compiled and published to PyPI.

## Type definitions

### `djc_core_template_parser/__init__.pyi`

This file contains the **public interface** that will be used by other packages (like the main Django Components package) in VSCode and other IDEs. It defines:

- All public functions and classes
- Type hints for parameters and return values
- Documentation for the API

This is the interface that external consumers of the package will see.

### Root `__init__.pyi`

The root `__init__.pyi` file is for **local development** and should be a copy of `djc_core_template_parser/__init__.pyi`.

**Important**: Keep these files in sync. When updating the interface, update both files.

## Test compatibility

To ensure that the parser works correctly with `django-components`, we define tests in `test_tag_parser.py`.

This test file exists in two locations:

- `djc_core_template_parser/tests/test_tag_parser.py` - Tests for the parser package itself
- `django_components/tests/test_tag_parser.py` - Integration tests for Django Components (copy)

When updating the `test_tag_parser.py`, you should also update the copy in `django_components/tests/test_tag_parser.py`.

## Architecture

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   Django        │    │   Rust Parser    │    │   Python        │
│   Template      │───▶│   (grammar.pest) │───▶│   Function      │
│   Syntax        │    │                  │    │                 │
└─────────────────┘    └──────────────────┘    └─────────────────┘
```

The parser uses Pest's declarative grammar to define Django template syntax rules, then compiles the parsed AST into optimized Python functions that can be called directly from Django Components.

## Contributing

1. Make changes to the Rust code in `src/`
2. Update the grammar in `grammar.pest` if needed
3. Update both `__init__.pyi` files if the interface changes
4. Add tests to both test files if needed
5. Run `maturin develop` to test your changes
6. Ensure all tests pass before submitting a PR
