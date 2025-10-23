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

## On template tag parser

The template syntax parsing was implemented using [Pest](https://pest.rs/). Pest works in 3 parts:

1.  "grammar rules" - definition of patterns that are supported in the.. language? I'm not sure about the correct terminology.

    Pest defines it's own language for defining these rules, see `djc-template-parser/src/grammar.pest`.

    This is similar to [Backus–Naur Form](https://en.wikipedia.org/wiki/Backus%E2%80%93Naur_form), e.g.
   
    ```
    <postal-address> ::= <name-part> <street-address> <zip-part>
    <name-part> ::= <personal-part> <last-name> <opt-suffix-part> <EOL> | <name-part>
    <street-address> ::= <house-num> <street-name> <opt-apt-num> <EOL>
    <zip-part> ::= <town-name> "," <state-code> <ZIP-code> <EOL>
    ```

    Or the MDN's formal syntax, e.g. [here](https://developer.mozilla.org/en-US/docs/Web/CSS/border-left-width#formal_syntax):
    ```
	border-left-width = 
	  <line-width>  
	
	<line-width> = 
	  [<length [0,∞]>](https://developer.mozilla.org/en-US/docs/Web/CSS/length)  [|](https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_values_and_units/Value_definition_syntax#single_bar)
	  thin            [|](https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_values_and_units/Value_definition_syntax#single_bar)
	  medium          [|](https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_values_and_units/Value_definition_syntax#single_bar)
	  thick
    ```

    Well and this Pest grammar is where all the permissible patterns are defined. E.g. here's a high-level example for a `{% ... %}` template tag (NOTE: outdated version):

      ```
      // The full tag is a sequence of attributes
      // E.g. `{% slot key=val key2=val2 %}`
      tag_wrapper = { SOI ~ django_tag ~ EOI }
      
      django_tag = { "{%" ~ tag_content ~ "%}" }
      
      // The contents of a tag, without the delimiters
      tag_content = ${
         spacing*                             // Optional leading whitespace/comments
         ~ tag_name                           // The tag name must come first, MAY be preceded by whitespace
         ~ (spacing+ ~ attribute)*            // Then zero or more attributes, MUST be separated by whitespace/comments
         ~ spacing*                           // Optional trailing whitespace/comments
         ~ self_closing_slash?                // Optional self-closing slash
         ~ spacing*                           // More optional trailing whitespace
      }
      ```

2. Parsing and handling of the matched grammar rules.

   So each defined rule has its own name, e.g. `django_tag`.

   When a text is parsed with Pest in Rust, we get a list of parsed rules (or a single rule?).

   Since the grammar definition specifies the entire `{% .. %}` template tag, and we pass in a string starting and ending in `{% ... %}`, we should match exactly the top-level `tag_wrapper` rule.

   If we match anything else in its place, we raise an error.

   Once we have `tag_wrapper`, we walk down it, rule by rule, constructing the AST from the patterns we come across.

3. Constructing the AST.

    The AST consists of these nodes - Tag, TagAttr, TagToken, TagValue, TagValueFilter

    - `Tag` - the entire `{% ... %}`, e.g `{% my_tag x ...[1, 2, 3] key=val / %}`

    - The first word inside a `Tag` is the `tag_name`, e.g. `my_tag`.
    - After the tag name, there are zero or more `TagAttrs`. This is ALL inputs, both positional and keyword
      - Tag attrs are `x`,  `...[1, 2, 3]`, `key=val`
      - If a tag attribute has a key, that's stored on `TagAttrs`.
      - But ALL `TagAttrs` MUST have a value.
    - TagValue holds a single value, may have a filter, e.g. `"cool"|upper`
      - TagValue may be of different kinds, e.g. string, int, float, literal list, literal dict, variable, translation  `_('mystr')`, etc. The specific kind is identified by what rules we parse, and the resulting TagValue nodes are distinguished by the `ValueKind`, an enum with values like `"string"`, `"float"`, etc.
      - Since TagValue can be also e.g. literal lists, TagValues may contain other TagValues. This implies that:
         1. Lists and dicts themselves can have filters applied to them, e.g. `[1, 2, 3]|append:4`
         2. items inside lists and dicts can too have filters applied to them.   e.g. `[1|add:1, 2|add:2]`
   - Any TagValue can have 0 or more filters applied to it. Filters have a name and an optional argument, e.g. `3|add:2` - filter name `add`, arg `2`. These filters are held by `TagValueFilter`.
     - While the filter name is a plain identifier, the argument can be yet another TagValue. so even using literal lists and dicts at the position of filter argument is permitted, e.g. `[1]|extend:[2, 3]`

    - Lastly, `TagToken` is a secondary object used by the nodes above. It contains info about the original raw string, and the line / col where the string was found.

The final AST can look like this:

INPUT:
```django
{% my_tag value|lower %}
```

AST:
```rs
Tag {
    name: TagToken {
        token: "my_tag".to_string(),
        start_index: 3,
        end_index: 9,
        line_col: (1, 4),
    },
    attrs: vec![TagAttr {
        key: None,
        value: TagValue {
            token: TagToken {
                token: "value".to_string(),
                start_index: 10,
                end_index: 15,
                line_col: (1, 11),
            },
            children: vec![],
            spread: None,
            filters: vec![TagValueFilter {
                arg: None,
                token: TagToken {
                    token: "lower".to_string(),
                    start_index: 16,
                    end_index: 21,
                    line_col: (1, 17),
                },
                start_index: 15,
                end_index: 21,
                line_col: (1, 16),
            }],
            kind: ValueKind::Variable,
            start_index: 10,
            end_index: 21,
            line_col: (1, 11),
        },
        is_flag: false,
        start_index: 10,
        end_index: 21,
        line_col: (1, 11),
    }],
    is_self_closing: false,
    syntax: TagSyntax::Django,
    start_index: 0,
    end_index: 24,
    line_col: (1, 4),
}
```


## On template tag compilation

Another important part is the "tag compiler". This turns the parsed AST into an executable Python function. When this function is called with the `Context` object, it resolves the inputs to a tag into Python args and kwargs.

```py
from djc_core import parse_tag, compile_tag

ast = parse_tag('{% my_tag var1 ...[2, 3] key=val ...{"other": "x"} / %}')
tag_fn = compile_tag(ast)

args, kwargs = tag_fn({"var1": "hello", "val": "abc"})

assert args == ["hello", 2, 3]
assert kwargs == {"key": "abc", "other": "x"}
```

How it works is:

1. We start with the AST of the template tag.
2. TagAttrs with keys become function's kwargs, and TagAttrs without keys are functions args.
3. For each TagAttr, we walk down it's value, and handle each ValueKind differently
   - Literals - 1, 1.5, "abc", etc - These are compiled as literal Python values
   - Variables - e.g. `my_var` - we replace that with function call `variable(context, "my_var")`
   - Filters - `my_var|add:"txt"` - replaced with function call `filter(context, "add", my_var, "txt")`
   - Translation `_("abc")` - function call `translation(context, "abc")`
   - String with nested template tags, e.g. `"Hello {{ first_name }}"` - function call `template_string(context, "Hello {{ first_name }}")`
   - Literal lists and dicts - structure preserved, and we walk down and convert each item, key, value.
		
	Input:
		
	```django
	{% component my_var|add:"txt" / %}
	```
		
	Generated function:
		
	```py
	def compiled_func(context, *, template_string, translation, variable, filter):
	    args = []
	    kwargs = []
	    args.append(filter(context, 'add', variable(context, 'my_var'), "txt"))
	    return args, kwargs
	```

4. Apply Django-specific logic

   As you can see, the generated function accepts the definitions for the functions `variable()`, `filter()`, etc.

   This means that the implementation for these is defined in Python. So we can still easily change how individual features are handled. These definitions of `variable()`, etc are NOT exposed to the users of django-components.

    The implementation is defined in django-components, and it looks something like below.

    There you can see e.g. that when the Rust compiler came across a variable `my_var`, it generated `variable(..)` call. And the implementation for `variable(...)` calls Django's `Variable(var).resolve(ctx)`. 

    So at the end of the day we're still using the same Django logic to actually resolve variables into actual values. 

	```py
    def resolve_template_string(ctx: Context, expr: str) -> Any:
        return TemplateStringExpression(
            expr_str=expr,
            filters=filters,
            tags=tags,
        ).resolve(ctx)

    def resolve_filter(_ctx: Context, name: str, value: Any, arg: Any) -> Any:
        if name not in filters:
            raise TemplateSyntaxError(f"Invalid filter: '{name}'")

        filter_func = filters[name]
        if arg is None:
            return filter_func(value)
        else:
            return filter_func(value, arg)

    def resolve_variable(ctx: Context, var: str) -> Any:
        try:
            return Variable(var).resolve(ctx)
        except VariableDoesNotExist:
            return ""

    def resolve_translation(ctx: Context, var: str) -> Any:
        # The compiler gives us the variable stripped of `_(")` and `"),
        # so we put it back for Django's Variable class to interpret it as a translation.
        translation_var = "_('" + var + "')"
        return Variable(translation_var).resolve(ctx)

    args, kwargs = compiled_tag(
        context=context,
        template_string=template_string,
        variable=resolve_variable,
        translation=resolve_translation,
        filter=resolve_filter,
    )
	```

5. Call the component with the args and kwargs

    The compiled function returned a list of args and a dict of kwargs. We then simply pass these further to the implementation of the `{% component %}` node.

	So a template tag like this:

	```django
    {% component "my_table" var1 ...[2, 3] key=val ...{"other": "x"} / %}
    ```

    Eventually gets resolved to something like so:

    ```py
    ComponentNode.render("my_table", var1, 2, 3, key=val, other="x")
    ```

**Validation**

The template tag inputs respect Python's convetion of not allowing args after kwargs.

When compiling AST into a Python function, we're able to detect obvious cases and raise an error early, like:

```django
{% component key=val my_var / %}  {# Error! #}
```
However, some cases can be figured out only at render time. Becasue the spread syntax `...my_var` can be used with both a list of args or a dict of kwargs.

So we need to wait for the Context object to figure out whether this:
```django
{% component ...items my_var  / %}
```
Resolves to lists (OK):
```django
{% component ...[1, 2, 3] my_var  / %}
```
Or to dict (Error):
```django
{% component ...{"key": "x"} my_var  / %}
```

So when we detect that there is a spread within the template tag, we add a render-time function that checks whether the spread resolves to list or a dict, and raises if it's not permitted:

INPUT:
```django
{% component ...options1 key1="value1" ...options2 key1="value1" / %}
```

Generated function:
```py
def compiled_func(context, *, template_string, translation, variable, filter):
    def _handle_spread(value, raw_token_str, args, kwargs, kwarg_seen):
        if hasattr(value, "keys"):
            kwargs.extend(value.items())
            return True
        else:
            if kwarg_seen:
                raise SyntaxError("positional argument follows keyword argument")
            try:
                args.extend(value)
            except TypeError:
                raise TypeError(
                    f"Value of '...{raw_token_str}' must be a mapping or an iterable, "
                    f"not {type(value).__name__}."
                )
            return False

    args = []
    kwargs = []
    kwargs.append(('key1', "value1"))
    kwarg_seen = True
    kwarg_seen = _handle_spread(variable(context, 'options1'), """options1""", args, kwargs, kwarg_seen)
    kwargs.append(('key2', "value2"))
    kwarg_seen = _handle_spread(variable(context, 'options2'), """options2""", args, kwargs, kwarg_seen)
    return args, kwargs
```
