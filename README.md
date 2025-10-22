# djc-core

[![PyPI - Version](https://img.shields.io/pypi/v/djc-core)](https://pypi.org/project/djc-core/) [![PyPI - Python Version](https://img.shields.io/pypi/pyversions/djc-core)](https://pypi.org/project/djc-core/) [![PyPI - License](https://img.shields.io/pypi/l/djc-core)](https://github.com/django-components/djc-core/blob/master/LICENSE/) [![PyPI - Downloads](https://img.shields.io/pypi/dm/djc-core)](https://pypistats.org/packages/djc-core) [![GitHub Actions Workflow Status](https://img.shields.io/github/actions/workflow/status/django-components/djc-core/tests.yml)](https://github.com/django-components/djc-core/actions/workflows/tests.yml)

Rust-based parsers and toolings used by [django-components](https://github.com/django-components/django-components). Exposed as a Python package with [maturin](https://www.maturin.rs/).

## Installation

```sh
pip install djc-core
```

## Packages

### HTML transfomer

Transform HTML in a single pass. This is a simple implementation.

This implementation was found to be 40-50x faster than our Python implementation, taking ~90ms to parse 5 MB of HTML.

**Usage**

```python
from djc_core import set_html_attributes

html = '<div><p>Hello</p></div>'
result, _ = set_html_attributes(
  html,
  # Add attributes to the root elements
  root_attributes=['data-root-id'],
  # Add attributes to all elements
  all_attributes=['data-v-123'],
)
```

To save ourselves from re-parsing the HTML, `set_html_attributes` returns not just the transformed HTML, but also a dictionary as the second item.

This dictionary contains a record of which HTML attributes were written to which elemenents.

To populate this dictionary, you need set `watch_on_attribute` to an attribute name.

Then, during the HTML transformation, we check each element for this attribute. And if the element HAS this attribute, we:

1. Get the value of said attribute
2. Record the attributes that were added to the element, using the value of the watched attribute as the key.

```python
from djc_core import set_html_attributes

html = """
  <div data-watch-id="123">
    <p data-watch-id="456">
      Hello
    </p>
  </div>
"""

result, captured = set_html_attributes(
  html,
  # Add attributes to the root elements
  root_attributes=['data-root-id'],
  # Add attributes to all elements
  all_attributes=['data-djc-tag'],
  # Watch for this attribute on elements
  watch_on_attribute='data-watch-id',
)

print(captured)
# {
#   '123': ['data-root-id', 'data-djc-tag'],
#   '456': ['data-djc-tag'],
# }
```

## Development

1. Setup python env

   ```sh
   python -m venv .venv
   ```

2. Install dependencies

   ```sh
   pip install -r requirements-dev.txt
   ```

   The dev requirements also include `maturin` which is used packaging a Rust project
   as Python package.

3. Install Rust

   See https://www.rust-lang.org/tools/install

4. Run Rust tests

   ```sh
   cargo test
   ```

5. Build the Python package

   ```sh
   maturin develop
   ```

   To build the production-optimized package, use `maturin develop --release`.

6. Run Python tests

   ```sh
   pytest
   ```

   > NOTE: When running Python tests, you need to run `maturin develop` first.

## Deployment

Deployment is done automatically via GitHub Actions.

To publish a new version of the package, you need to:

1. Bump the version in `pyproject.toml` and `Cargo.toml`
2. Open a PR and merge it to `main`.
3. Create a new tag on the `main` branch with the new version number (e.g. `1.0.0`), or create a new release in the GitHub UI.
