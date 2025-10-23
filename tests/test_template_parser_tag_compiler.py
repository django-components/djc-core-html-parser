# ruff: noqa: ANN001,ANN201,ANN202,ARG001,ARG005,S101
from unittest.mock import Mock, call

import pytest
from djc_core import compile_tag, parse_tag


def test_full_compilation_flow():
    tag_content = (
        '{% my_tag "a string" var_one 123 '
        'key_one="a value" '
        "key_two=var_two "
        'key_three=_("a translation") '
        'key_four="{{ an_expression }}" '
        "...spread_var|dict_filter "
        'key_five=my_val|other_filter:"my_arg" '
        "key_five=123 %}"
    )

    ast = parse_tag(tag_content)
    compiled_func = compile_tag(ast)

    context = {
        "var_one": "resolved_var_one",
        "var_two": "resolved_var_two",
        "spread_var": {"a": 1, "b": 2},
        "my_val": "original_value",
    }

    mock_variable = Mock(side_effect=lambda ctx, var: ctx.get(var))
    mock_template_string = Mock(side_effect=lambda ctx, expr: f"TEMPLATE_RESOLVED:{expr}")
    mock_translation = Mock(side_effect=lambda ctx, text: f"TRANSLATION_RESOLVED:{text}")

    def dummy_filter_side_effect(context, name, value, arg=None):
        # This filter is used on the spread argument, so it must return a dict
        if name == "dict_filter":
            return {"a": 1, "b": 2}
        return f"{value}|{name}:{arg}"

    mock_filter = Mock(side_effect=dummy_filter_side_effect)

    # The generated function has dependencies as keyword-only args
    args, kwargs = compiled_func(
        context,
        variable=mock_variable,
        template_string=mock_template_string,
        translation=mock_translation,
        filter=mock_filter,
    )

    # Assert that our Python callbacks were called with the correct arguments
    mock_variable.assert_has_calls(
        [
            call(context, "var_one"),
            call(context, "var_two"),
            call(context, "spread_var"),
            call(context, "my_val"),
        ]
    )
    mock_template_string.assert_called_once_with(context, "{{ an_expression }}")
    mock_translation.assert_called_once_with(context, "a translation")
    mock_filter.assert_has_calls(
        [
            call(context, "dict_filter", {"a": 1, "b": 2}, None),
            call(context, "other_filter", "original_value", "my_arg"),
        ]
    )

    # Assert the final resolved values
    assert args == [
        "a string",
        "resolved_var_one",
        123,
    ]

    assert kwargs == [
        ("key_one", "a value"),
        ("key_two", "resolved_var_two"),
        ("key_three", "TRANSLATION_RESOLVED:a translation"),
        ("key_four", "TEMPLATE_RESOLVED:{{ an_expression }}"),
        # Spread variables from `...spread_var|my_filter` are expanded into tuples
        ("a", 1),
        ("b", 2),
        # Kwargs after the spread variable
        ("key_five", "original_value|other_filter:my_arg"),
        # Compiler doesn't omit repeated kwargs, this is to be handled in Python
        ("key_five", 123),
    ]


# Since flags are NOT treated as args, this should be OK
def test_flag_after_kwarg():
    tag_content = "{% my_tag key='value' my_flag %}"

    ast1 = parse_tag(tag_content, flags={"my_flag"})
    assert ast1.attrs[1].value.token.token == "my_flag"
    assert ast1.attrs[1].is_flag

    compiled_func1 = compile_tag(ast1)
    args1, kwargs1 = compiled_func1(
        context={},
        variable=lambda ctx, var: ctx[var],
        template_string=Mock(),
        translation=Mock(),
        filter=Mock(),
    )
    assert args1 == []
    assert kwargs1 == [("key", "value")]

    # Same as before, but with flags=None
    ast2 = parse_tag(tag_content, flags=None)
    assert ast2.attrs[1].value.token.token == "my_flag"
    assert not ast2.attrs[1].is_flag

    with pytest.raises(SyntaxError, match="positional argument follows keyword argument"):
        compile_tag(ast2)


class TestParamsOrder:
    def test_arg_after_kwarg_is_error(self):
        tag_content = "{% my_tag key='value' positional_arg %}"
        ast = parse_tag(input=tag_content)
        with pytest.raises(SyntaxError, match="positional argument follows keyword argument"):
            compile_tag(tag_or_attrs=ast)

    def test_arg_after_dict_spread_is_error(self):
        tag_content = "{% my_tag ...{'key': 'value'} positional_arg %}"
        ast = parse_tag(input=tag_content)
        tag_func = compile_tag(ast)

        with pytest.raises(SyntaxError, match="positional argument follows keyword argument"):
            tag_func(
                context={},
                variable=Mock(),
                template_string=Mock(),
                translation=Mock(),
                filter=Mock(),
            )

    def test_arg_after_list_spread_is_ok(self):
        tag_content = "{% my_tag ...[1, 2, 3] positional_arg %}"
        ast = parse_tag(input=tag_content)
        tag_func = compile_tag(ast)
        args, kwargs = tag_func(
            context={"positional_arg": 4},
            variable=lambda ctx, var: ctx[var],
            template_string=Mock(),
            translation=Mock(),
            filter=Mock(),
        )
        assert args == [1, 2, 3, 4]
        assert kwargs == []

    def test_dict_spread_after_arg_is_ok(self):
        tag_content = "{% my_tag positional_arg ...{'key': 'value'} %}"
        ast = parse_tag(input=tag_content)
        tag_func = compile_tag(ast)
        args, kwargs = tag_func(
            context={"positional_arg": 1},
            variable=lambda ctx, var: ctx[var],
            template_string=Mock(),
            translation=Mock(),
            filter=Mock(),
        )
        assert args == [1]
        assert kwargs == [("key", "value")]

    def test_dict_spread_after_kwarg_is_ok(self):
        tag_content = "{% my_tag key='value' ...{'key2': 'value2'} %}"
        ast = parse_tag(input=tag_content)
        tag_func = compile_tag(ast)
        args, kwargs = tag_func(
            context={},
            variable=Mock(),
            template_string=Mock(),
            translation=Mock(),
            filter=Mock(),
        )
        assert args == []
        assert kwargs == [("key", "value"), ("key2", "value2")]

    def test_list_spread_after_arg_is_ok(self):
        tag_content = "{% my_tag positional_arg ...[1, 2, 3] %}"
        ast = parse_tag(input=tag_content)
        tag_func = compile_tag(ast)
        args, kwargs = tag_func(
            context={"positional_arg": 4},
            variable=lambda ctx, var: ctx[var],
            template_string=Mock(),
            translation=Mock(),
            filter=Mock(),
        )
        assert args == [4, 1, 2, 3]
        assert kwargs == []

    def test_list_spread_after_kwarg_is_error(self):
        tag_content = "{% my_tag key='value' ...[1, 2, 3] %}"
        ast = parse_tag(input=tag_content)
        tag_func = compile_tag(ast)
        with pytest.raises(SyntaxError, match="positional argument follows keyword argument"):
            tag_func(
                context={},
                variable=Mock(),
                template_string=Mock(),
                translation=Mock(),
                filter=Mock(),
            )

    def test_list_spread_after_list_spread_is_ok(self):
        tag_content = "{% my_tag ...[1, 2, 3] ...[4, 5, 6] %}"
        ast = parse_tag(input=tag_content)
        tag_func = compile_tag(ast)
        args, kwargs = tag_func(
            context={},
            variable=Mock(),
            template_string=Mock(),
            translation=Mock(),
            filter=Mock(),
        )
        assert args == [1, 2, 3, 4, 5, 6]
        assert kwargs == []

    def test_dict_spread_after_dict_spread_is_ok(self):
        tag_content = "{% my_tag ...{'key': 'value'} ...{'key2': 'value2'} %}"
        ast = parse_tag(input=tag_content)
        tag_func = compile_tag(ast)
        args, kwargs = tag_func(
            context={},
            variable=Mock(),
            template_string=Mock(),
            translation=Mock(),
            filter=Mock(),
        )
        assert args == []
        assert kwargs == [("key", "value"), ("key2", "value2")]

    def test_list_spread_after_dict_spread_is_error(self):
        tag_content = "{% my_tag ...{'key': 'value'} ...[1, 2, 3] %}"
        ast = parse_tag(input=tag_content)
        tag_func = compile_tag(ast)
        with pytest.raises(SyntaxError, match="positional argument follows keyword argument"):
            tag_func(
                context={},
                variable=Mock(),
                template_string=Mock(),
                translation=Mock(),
                filter=Mock(),
            )

    def test_dict_spread_after_list_spread_is_ok(self):
        tag_content = "{% my_tag ...[1, 2, 3] ...{'key': 'value'} %}"
        ast = parse_tag(input=tag_content)
        tag_func = compile_tag(ast)
        args, kwargs = tag_func(
            context={},
            variable=Mock(),
            template_string=Mock(),
            translation=Mock(),
            filter=Mock(),
        )
        assert args == [1, 2, 3]
        assert kwargs == [("key", "value")]
