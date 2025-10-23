# ruff: noqa
from typing import Any, Callable, List, Literal, Optional, Protocol, Set, Tuple, TypeVar, Union

TContext = TypeVar("TContext")

class ValueKind:
    def __init__(
        self, kind: Literal["list", "dict", "int", "float", "variable", "template_string", "translation", "string"]
    ) -> None: ...

class TagSyntax:
    def __init__(self, syntax: Literal["django", "html"]) -> None: ...

class TagToken:
    def __init__(self, token: str, start_index: int, end_index: int, line_col: Tuple[int, int]) -> None: ...
    token: str
    start_index: int
    end_index: int
    line_col: Tuple[int, int]

class TagValueFilter:
    def __init__(
        self, token: TagToken, arg: Optional[TagValue], start_index: int, end_index: int, line_col: Tuple[int, int]
    ) -> None: ...
    token: TagToken
    arg: Optional[TagValue]
    start_index: int
    end_index: int
    line_col: Tuple[int, int]

class TagValue:
    def __init__(
        self,
        token: TagToken,
        children: List[TagValue],
        kind: ValueKind,
        spread: Optional[str],
        filters: List[TagValueFilter],
        start_index: int,
        end_index: int,
        line_col: Tuple[int, int],
    ) -> None: ...
    token: TagToken
    children: List[TagValue]
    kind: ValueKind
    spread: Optional[str]
    filters: List[TagValueFilter]
    start_index: int
    end_index: int
    line_col: Tuple[int, int]

class TagAttr:
    def __init__(
        self,
        key: Optional[TagToken],
        value: TagValue,
        is_flag: bool,
        start_index: int,
        end_index: int,
        line_col: Tuple[int, int],
    ) -> None: ...
    key: Optional[TagToken]
    value: TagValue
    is_flag: bool
    start_index: int
    end_index: int
    line_col: Tuple[int, int]

class Tag:
    def __init__(
        self,
        name: TagToken,
        attrs: List[TagAttr],
        is_self_closing: bool,
        syntax: TagSyntax,
        start_index: int,
        end_index: int,
        line_col: Tuple[int, int],
    ) -> None: ...
    name: TagToken
    attrs: List[TagAttr]
    is_self_closing: bool
    syntax: TagSyntax
    start_index: int
    end_index: int
    line_col: Tuple[int, int]

class CompiledFunc(Protocol[TContext]):
    def __call__(
        self,
        context: TContext,
        *,
        variable: Callable[[TContext, str], Any],
        template_string: Callable[[TContext, str], Any],
        translation: Callable[[TContext, str], Any],
        filter: Callable[[TContext, str, Any, Any], Any],
    ) -> Tuple[List[Any], List[Tuple[str, Any]]]: ...
    """
    The result of compiling the template tag AST into a Python function.

    This function accepts the context object, and function implementations,
    and returns a tuple of arguments and keyword arguments.

    Example:

    ```python
    tag_ast = parse_tag('...[val1] [1, 2, 3] a=b data={"key": "value"}')
    compiled_func = compile_tag(tag_ast)

    context = {"val1": "foo", "b": "bar"}
    variable = lambda ctx, var: ctx.get(var)
    template_string = lambda ctx, expr: f"TEMPLATE_RESOLVED:{expr}"
    translation = lambda ctx, text: f"TRANSLATION_RESOLVED:{text}"
    filter = lambda ctx, name, value, arg=None: f"{value}|{name}:{arg}"

    args, kwargs = compiled_func(
        context,
        variable=variable,
        template_string=template_string,
        translation=translation,
        filter=filter,
    )

    print(args) # ['foo', [1, 2, 3]]
    print(kwargs) # [('a', 'bar'), ('data', {'key': 'value'})]
    ```
    """

def parse_tag(input: str, flags: Optional[Set[str]] = None) -> Tag:
    """
    Parse a Django template tag string into a Tag object.

    If you have a template tag string like this:

    ```django
    {% my_tag ...[val1] a=b [1, 2, 3] data={"key": "value"} / %}
    ```

    Then this parser accepts the contents of the tag as a string, and returns their AST - a Tag object.

    ```python
    tag_ast = parse_tag('my_tag ...[val1] a=b [1, 2, 3] data={"key": "value"} /')
    print(tag_ast.name) # TagToken(token='my_tag', ...)
    print(tag_ast.attrs) # [TagAttr(...), TagAttr(...), ...]
    print(tag_ast.is_self_closing) # True
    ```

    The parser supports:
    - Tag name (must be the first token)
    - Key-value pairs (e.g. key=value)
    - Standalone values (e.g. 1, "my string", val)
    - Spread operators (e.g. ...value, **value, *value)
    - Filters (e.g. value|filter:arg)
    - Lists and dictionaries (e.g. [1, 2, 3], {"key": "value"})
    - String literals (single/double quoted) (e.g. "my string", 'my string')
    - Numbers (e.g. 1, 1.23, 1e-10)
    - Variables (e.g. val, key)
    - Translation strings (e.g. _("text"))
    - Comments (e.g. {# comment #})
    - Self-closing tags (e.g. {% my_tag / %})

    Args:
        input: The template tag string to parse, without the {% %} delimiters
        flags: An optional set of strings that should be treated as flags.

    Returns:
        A Tag object representing the parsed tag.

    Raises:
        ValueError: If the input cannot be parsed according to the grammar

    """

def compile_tag(tag_or_attrs: Union[Tag, List[TagAttr]]) -> CompiledFunc[Any]:
    """
    Compile the template tag AST (`Tag` object or list of `TagAttr` objects generated by `parse_tag`)
    into a Python function.

    The generated function takes a `context` object and returns a tuple of arguments and keyword arguments.

    Args:
        tag_or_attrs: A `Tag` object from `parse_tag` or a list of `TagAttr` objects.

    Returns:
        A callable function that matches the `CompiledFunc` protocol.

    """

__all__ = [
    "parse_tag",
    "compile_tag",
    "CompiledFunc",
    "Tag",
    "TagAttr",
    "TagSyntax",
    "TagToken",
    "TagValue",
    "TagValueFilter",
    "ValueKind",
]
