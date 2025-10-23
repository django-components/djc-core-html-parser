"""
This file is defined both in django-components and djc_core_template_parser to ensure compatibility.

Source of truth is djc_core_template_parser.
"""

# ruff: noqa: ANN201,ARG005,S101,S105,S106,E501
import re

import pytest
from djc_core import (
    Tag,
    TagAttr,
    TagSyntax,
    TagToken,
    TagValue,
    TagValueFilter,
    ValueKind,
    compile_tag,
    parse_tag,
)


class TestTagParser:
    def test_args_kwargs(self):
        tag = parse_tag("{% component 'my_comp' key=val key2='val2 two' %}")

        expected_tag = Tag(
            name=TagToken(
                token="component",
                start_index=3,
                end_index=12,
                line_col=(1, 4),
            ),
            attrs=[
                TagAttr(
                    key=None,
                    value=TagValue(
                        token=TagToken(
                            token="'my_comp'",
                            start_index=13,
                            end_index=22,
                            line_col=(1, 14),
                        ),
                        children=[],
                        kind=ValueKind("string"),
                        spread=None,
                        filters=[],
                        start_index=13,
                        end_index=22,
                        line_col=(1, 14),
                    ),
                    is_flag=False,
                    start_index=13,
                    end_index=22,
                    line_col=(1, 14),
                ),
                TagAttr(
                    key=TagToken(
                        token="key",
                        start_index=23,
                        end_index=26,
                        line_col=(1, 24),
                    ),
                    value=TagValue(
                        token=TagToken(
                            token="val",
                            start_index=27,
                            end_index=30,
                            line_col=(1, 28),
                        ),
                        children=[],
                        kind=ValueKind("variable"),
                        spread=None,
                        filters=[],
                        start_index=27,
                        end_index=30,
                        line_col=(1, 28),
                    ),
                    is_flag=False,
                    start_index=23,
                    end_index=30,
                    line_col=(1, 24),
                ),
                TagAttr(
                    key=TagToken(
                        token="key2",
                        start_index=31,
                        end_index=35,
                        line_col=(1, 32),
                    ),
                    value=TagValue(
                        token=TagToken(
                            token="'val2 two'",
                            start_index=36,
                            end_index=46,
                            line_col=(1, 37),
                        ),
                        children=[],
                        kind=ValueKind("string"),
                        spread=None,
                        filters=[],
                        start_index=36,
                        end_index=46,
                        line_col=(1, 37),
                    ),
                    is_flag=False,
                    start_index=31,
                    end_index=46,
                    line_col=(1, 32),
                ),
            ],
            is_self_closing=False,
            syntax=TagSyntax("django"),
            start_index=0,
            end_index=49,
            line_col=(1, 4),
        )

        assert tag == expected_tag

        tag_func = compile_tag(tag)
        args, kwargs = tag_func(
            context={"val": [1, 2, 3]},
            variable=lambda ctx, var: ctx[var],
            template_string=lambda ctx, expr: f"TEMPLATE_RESOLVED:{expr}",
            translation=lambda ctx, text: f"TRANSLATION_RESOLVED:{text}",
            filter=lambda ctx, name, value, arg=None: f"{name}({value}, {arg})",
        )

        assert args == ["my_comp"]
        assert kwargs == [("key", [1, 2, 3]), ("key2", "val2 two")]

    def test_nested_quotes(self):
        tag = parse_tag("{% component 'my_comp' key=val key2='val2 \"two\"' text=\"organisation's\" %}")

        expected_tag = Tag(
            name=TagToken(
                token="component",
                start_index=3,
                end_index=12,
                line_col=(1, 4),
            ),
            attrs=[
                TagAttr(
                    key=None,
                    value=TagValue(
                        token=TagToken(
                            token="'my_comp'",
                            start_index=13,
                            end_index=22,
                            line_col=(1, 14),
                        ),
                        children=[],
                        kind=ValueKind("string"),
                        spread=None,
                        filters=[],
                        start_index=13,
                        end_index=22,
                        line_col=(1, 14),
                    ),
                    is_flag=False,
                    start_index=13,
                    end_index=22,
                    line_col=(1, 14),
                ),
                TagAttr(
                    key=TagToken(
                        token="key",
                        start_index=23,
                        end_index=26,
                        line_col=(1, 24),
                    ),
                    value=TagValue(
                        token=TagToken(
                            token="val",
                            start_index=27,
                            end_index=30,
                            line_col=(1, 28),
                        ),
                        children=[],
                        kind=ValueKind("variable"),
                        spread=None,
                        filters=[],
                        start_index=27,
                        end_index=30,
                        line_col=(1, 28),
                    ),
                    is_flag=False,
                    start_index=23,
                    end_index=30,
                    line_col=(1, 24),
                ),
                TagAttr(
                    key=TagToken(
                        token="key2",
                        start_index=31,
                        end_index=35,
                        line_col=(1, 32),
                    ),
                    value=TagValue(
                        token=TagToken(
                            token="'val2 \"two\"'",
                            start_index=36,
                            end_index=48,
                            line_col=(1, 37),
                        ),
                        children=[],
                        kind=ValueKind("string"),
                        spread=None,
                        filters=[],
                        start_index=36,
                        end_index=48,
                        line_col=(1, 37),
                    ),
                    is_flag=False,
                    start_index=31,
                    end_index=48,
                    line_col=(1, 32),
                ),
                TagAttr(
                    key=TagToken(
                        token="text",
                        start_index=49,
                        end_index=53,
                        line_col=(1, 50),
                    ),
                    value=TagValue(
                        token=TagToken(
                            token='"organisation\'s"',
                            start_index=54,
                            end_index=70,
                            line_col=(1, 55),
                        ),
                        children=[],
                        kind=ValueKind("string"),
                        spread=None,
                        filters=[],
                        start_index=54,
                        end_index=70,
                        line_col=(1, 55),
                    ),
                    is_flag=False,
                    start_index=49,
                    end_index=70,
                    line_col=(1, 50),
                ),
            ],
            is_self_closing=False,
            syntax=TagSyntax("django"),
            start_index=0,
            end_index=73,
            line_col=(1, 4),
        )

        assert tag == expected_tag

        tag_func = compile_tag(tag)
        args, kwargs = tag_func(
            context={"val": "some_value"},
            variable=lambda ctx, var: ctx[var],
            template_string=lambda ctx, expr: f"TEMPLATE_RESOLVED:{expr}",
            translation=lambda ctx, text: f"TRANSLATION_RESOLVED:{text}",
            filter=lambda ctx, name, value, arg=None: f"{name}({value}, {arg})",
        )

        assert args == ["my_comp"]
        assert kwargs == [
            ("key", "some_value"),
            ("key2", 'val2 "two"'),
            ("text", "organisation's"),
        ]

    def test_trailing_quote_single(self):
        # Test that the Rust parser correctly identifies malformed input with unclosed quote
        with pytest.raises(SyntaxError, match="expected self_closing_slash, attribute, filter, or COMMENT"):
            parse_tag("{% component 'my_comp' key=val key2='val2 \"two\"' text=\"organisation's\" 'abc %}")

    def test_trailing_quote_double(self):
        # Test that the Rust parser correctly identifies malformed input with unclosed double quote
        with pytest.raises(SyntaxError, match="expected self_closing_slash, attribute, filter, or COMMENT"):
            parse_tag('{% component "my_comp" key=val key2="val2 \'two\'" text=\'organisation"s\' "abc %}')

    def test_trailing_quote_as_value_single(self):
        # Test that the Rust parser correctly identifies malformed input with unclosed quote in key=value pair
        with pytest.raises(SyntaxError, match="expected value"):
            parse_tag("{% component 'my_comp' key=val key2='val2 \"two\"' text=\"organisation's\" value='abc %}")

    def test_trailing_quote_as_value_double(self):
        # Test that the Rust parser correctly identifies malformed input with unclosed double quote in key=value pair
        with pytest.raises(SyntaxError, match="expected value"):
            parse_tag('{% component "my_comp" key=val key2="val2 \'two\'" text=\'organisation"s\' value="abc %}')

    def test_translation(self):
        tag = parse_tag('{% component "my_comp" _("one") key=_("two") %}')

        expected_tag = Tag(
            name=TagToken(token="component", start_index=3, end_index=12, line_col=(1, 4)),
            attrs=[
                TagAttr(
                    key=None,
                    value=TagValue(
                        token=TagToken(token='"my_comp"', start_index=13, end_index=22, line_col=(1, 14)),
                        children=[],
                        kind=ValueKind("string"),
                        spread=None,
                        filters=[],
                        start_index=13,
                        end_index=22,
                        line_col=(1, 14),
                    ),
                    is_flag=False,
                    start_index=13,
                    end_index=22,
                    line_col=(1, 14),
                ),
                TagAttr(
                    key=None,
                    value=TagValue(
                        token=TagToken(token='_("one")', start_index=23, end_index=31, line_col=(1, 24)),
                        children=[],
                        kind=ValueKind("translation"),
                        spread=None,
                        filters=[],
                        start_index=23,
                        end_index=31,
                        line_col=(1, 24),
                    ),
                    is_flag=False,
                    start_index=23,
                    end_index=31,
                    line_col=(1, 24),
                ),
                TagAttr(
                    key=TagToken(token="key", start_index=32, end_index=35, line_col=(1, 33)),
                    value=TagValue(
                        token=TagToken(token='_("two")', start_index=36, end_index=44, line_col=(1, 37)),
                        children=[],
                        kind=ValueKind("translation"),
                        spread=None,
                        filters=[],
                        start_index=36,
                        end_index=44,
                        line_col=(1, 37),
                    ),
                    is_flag=False,
                    start_index=32,
                    end_index=44,
                    line_col=(1, 33),
                ),
            ],
            is_self_closing=False,
            syntax=TagSyntax("django"),
            start_index=0,
            end_index=47,
            line_col=(1, 4),
        )

        assert tag == expected_tag

        tag_func = compile_tag(tag)
        args, kwargs = tag_func(
            context={},
            variable=lambda ctx, var: ctx[var],
            template_string=lambda ctx, expr: f"TEMPLATE_RESOLVED:{expr}",
            translation=lambda ctx, text: f"TRANSLATION_RESOLVED:{text}",
            filter=lambda ctx, name, value, arg=None: f"{name}({value}, {arg})",
        )

        assert args == ["my_comp", "TRANSLATION_RESOLVED:one"]
        assert kwargs == [("key", "TRANSLATION_RESOLVED:two")]

    def test_translation_whitespace(self):
        tag = parse_tag('{% component value=_(  "test"  ) %}')

        expected_tag = Tag(
            name=TagToken(token="component", start_index=3, end_index=12, line_col=(1, 4)),
            attrs=[
                TagAttr(
                    key=TagToken(token="value", start_index=13, end_index=18, line_col=(1, 14)),
                    value=TagValue(
                        token=TagToken(token='_("test")', start_index=19, end_index=32, line_col=(1, 20)),
                        children=[],
                        kind=ValueKind("translation"),
                        spread=None,
                        filters=[],
                        start_index=19,
                        end_index=32,
                        line_col=(1, 20),
                    ),
                    is_flag=False,
                    start_index=13,
                    end_index=32,
                    line_col=(1, 14),
                ),
            ],
            is_self_closing=False,
            syntax=TagSyntax("django"),
            start_index=0,
            end_index=35,
            line_col=(1, 4),
        )

        assert tag == expected_tag

        tag_func = compile_tag(tag)
        args, kwargs = tag_func(
            context={},
            variable=lambda ctx, var: ctx[var],
            template_string=lambda ctx, expr: f"TEMPLATE_RESOLVED:{expr}",
            translation=lambda ctx, text: f"TRANSLATION_RESOLVED:{text}",
            filter=lambda ctx, name, value, arg=None: f"{name}({value}, {arg})",
        )

        assert args == []
        assert kwargs == [("value", "TRANSLATION_RESOLVED:test")]


class TestFilter:
    def test_tag_parser_filters(self):
        tag = parse_tag('{% component "my_comp" value|lower key=val|yesno:"yes,no" key2=val2|default:"N/A"|upper %}')

        expected_tag = Tag(
            name=TagToken(token="component", start_index=3, end_index=12, line_col=(1, 4)),
            attrs=[
                TagAttr(
                    key=None,
                    value=TagValue(
                        token=TagToken(token='"my_comp"', start_index=13, end_index=22, line_col=(1, 14)),
                        children=[],
                        kind=ValueKind("string"),
                        spread=None,
                        filters=[],
                        start_index=13,
                        end_index=22,
                        line_col=(1, 14),
                    ),
                    is_flag=False,
                    start_index=13,
                    end_index=22,
                    line_col=(1, 14),
                ),
                TagAttr(
                    key=None,
                    value=TagValue(
                        token=TagToken(token="value", start_index=23, end_index=28, line_col=(1, 24)),
                        children=[],
                        kind=ValueKind("variable"),
                        spread=None,
                        filters=[
                            TagValueFilter(
                                token=TagToken(token="lower", start_index=29, end_index=34, line_col=(1, 30)),
                                arg=None,
                                start_index=28,
                                end_index=34,
                                line_col=(1, 29),
                            )
                        ],
                        start_index=23,
                        end_index=34,
                        line_col=(1, 24),
                    ),
                    is_flag=False,
                    start_index=23,
                    end_index=34,
                    line_col=(1, 24),
                ),
                TagAttr(
                    key=TagToken(token="key", start_index=35, end_index=38, line_col=(1, 36)),
                    value=TagValue(
                        token=TagToken(token="val", start_index=39, end_index=42, line_col=(1, 40)),
                        children=[],
                        kind=ValueKind("variable"),
                        spread=None,
                        filters=[
                            TagValueFilter(
                                token=TagToken(token="yesno", start_index=43, end_index=48, line_col=(1, 44)),
                                arg=TagValue(
                                    token=TagToken(token='"yes,no"', start_index=49, end_index=57, line_col=(1, 50)),
                                    children=[],
                                    kind=ValueKind("string"),
                                    spread=None,
                                    filters=[],
                                    start_index=48,
                                    end_index=57,
                                    line_col=(1, 49),
                                ),
                                start_index=42,
                                end_index=57,
                                line_col=(1, 43),
                            )
                        ],
                        start_index=39,
                        end_index=57,
                        line_col=(1, 40),
                    ),
                    is_flag=False,
                    start_index=35,
                    end_index=57,
                    line_col=(1, 36),
                ),
                TagAttr(
                    key=TagToken(token="key2", start_index=58, end_index=62, line_col=(1, 59)),
                    value=TagValue(
                        token=TagToken(token="val2", start_index=63, end_index=67, line_col=(1, 64)),
                        children=[],
                        kind=ValueKind("variable"),
                        spread=None,
                        filters=[
                            TagValueFilter(
                                token=TagToken(token="default", start_index=68, end_index=75, line_col=(1, 69)),
                                arg=TagValue(
                                    token=TagToken(token='"N/A"', start_index=76, end_index=81, line_col=(1, 77)),
                                    children=[],
                                    kind=ValueKind("string"),
                                    spread=None,
                                    filters=[],
                                    start_index=75,
                                    end_index=81,
                                    line_col=(1, 76),
                                ),
                                start_index=67,
                                end_index=81,
                                line_col=(1, 68),
                            ),
                            TagValueFilter(
                                token=TagToken(token="upper", start_index=82, end_index=87, line_col=(1, 83)),
                                arg=None,
                                start_index=81,
                                end_index=87,
                                line_col=(1, 82),
                            ),
                        ],
                        start_index=63,
                        end_index=87,
                        line_col=(1, 64),
                    ),
                    is_flag=False,
                    start_index=58,
                    end_index=87,
                    line_col=(1, 59),
                ),
            ],
            is_self_closing=False,
            syntax=TagSyntax("django"),
            start_index=0,
            end_index=90,
            line_col=(1, 4),
        )

        assert tag == expected_tag

        tag_func = compile_tag(tag)
        args, kwargs = tag_func(
            context={"value": "HELLO", "val": True, "val2": None},
            variable=lambda ctx, var: ctx[var],
            template_string=lambda ctx, expr: f"TEMPLATE_RESOLVED:{expr}",
            translation=lambda ctx, text: f"TRANSLATION_RESOLVED:{text}",
            filter=lambda ctx, name, value, arg=None: f"{name}({value}, {arg})",
        )

        assert args == ["my_comp", "lower(HELLO, None)"]
        assert kwargs == [
            ("key", "yesno(True, yes,no)"),
            ("key2", "upper(default(None, N/A), None)"),
        ]

    def test_filter_whitespace(self):
        tag = parse_tag("{% component value  |  lower    key=val  |  upper    key2=val2 %}")

        expected_tag = Tag(
            name=TagToken(token="component", start_index=3, end_index=12, line_col=(1, 4)),
            attrs=[
                TagAttr(
                    key=None,
                    value=TagValue(
                        token=TagToken(token="value", start_index=13, end_index=18, line_col=(1, 14)),
                        children=[],
                        kind=ValueKind("variable"),
                        spread=None,
                        filters=[
                            TagValueFilter(
                                token=TagToken(token="lower", start_index=23, end_index=28, line_col=(1, 24)),
                                arg=None,
                                start_index=20,
                                end_index=28,
                                line_col=(1, 21),
                            )
                        ],
                        start_index=13,
                        end_index=28,
                        line_col=(1, 14),
                    ),
                    is_flag=False,
                    start_index=13,
                    end_index=28,
                    line_col=(1, 14),
                ),
                TagAttr(
                    key=TagToken(token="key", start_index=32, end_index=35, line_col=(1, 33)),
                    value=TagValue(
                        token=TagToken(token="val", start_index=36, end_index=39, line_col=(1, 37)),
                        children=[],
                        kind=ValueKind("variable"),
                        spread=None,
                        filters=[
                            TagValueFilter(
                                token=TagToken(token="upper", start_index=44, end_index=49, line_col=(1, 45)),
                                arg=None,
                                start_index=41,
                                end_index=49,
                                line_col=(1, 42),
                            )
                        ],
                        start_index=36,
                        end_index=49,
                        line_col=(1, 37),
                    ),
                    is_flag=False,
                    start_index=32,
                    end_index=49,
                    line_col=(1, 33),
                ),
                TagAttr(
                    key=TagToken(token="key2", start_index=53, end_index=57, line_col=(1, 54)),
                    value=TagValue(
                        token=TagToken(token="val2", start_index=58, end_index=62, line_col=(1, 59)),
                        children=[],
                        kind=ValueKind("variable"),
                        spread=None,
                        filters=[],
                        start_index=58,
                        end_index=62,
                        line_col=(1, 59),
                    ),
                    is_flag=False,
                    start_index=53,
                    end_index=62,
                    line_col=(1, 54),
                ),
            ],
            is_self_closing=False,
            syntax=TagSyntax("django"),
            start_index=0,
            end_index=65,
            line_col=(1, 4),
        )

        assert tag == expected_tag

        tag_func = compile_tag(tag)
        args, kwargs = tag_func(
            context={"value": "HELLO", "val": "world", "val2": "test"},
            variable=lambda ctx, var: ctx[var],
            template_string=lambda ctx, expr: f"TEMPLATE_RESOLVED:{expr}",
            translation=lambda ctx, text: f"TRANSLATION_RESOLVED:{text}",
            filter=lambda ctx, name, value, arg=None: f"{name}({value}, {arg})",
        )

        assert args == ["lower(HELLO, None)"]
        assert kwargs == [
            ("key", "upper(world, None)"),
            ("key2", "test"),
        ]

    def test_filter_argument_must_follow_filter(self):
        with pytest.raises(
            SyntaxError,
            match=re.escape("expected filter or COMMENT"),
        ):
            parse_tag('{% component value=val|yesno:"yes,no":arg %}')


class TestDict:
    def test_dict_simple(self):
        tag = parse_tag('{% component data={ "key": "val" } %}')

        expected_tag = Tag(
            name=TagToken(token="component", start_index=3, end_index=12, line_col=(1, 4)),
            attrs=[
                TagAttr(
                    key=TagToken(token="data", start_index=13, end_index=17, line_col=(1, 14)),
                    value=TagValue(
                        token=TagToken(token='{ "key": "val" }', start_index=18, end_index=34, line_col=(1, 19)),
                        children=[
                            TagValue(
                                token=TagToken(token='"key"', start_index=20, end_index=25, line_col=(1, 21)),
                                children=[],
                                kind=ValueKind("string"),
                                spread=None,
                                filters=[],
                                start_index=20,
                                end_index=25,
                                line_col=(1, 21),
                            ),
                            TagValue(
                                token=TagToken(token='"val"', start_index=27, end_index=32, line_col=(1, 28)),
                                children=[],
                                kind=ValueKind("string"),
                                spread=None,
                                filters=[],
                                start_index=27,
                                end_index=32,
                                line_col=(1, 28),
                            ),
                        ],
                        kind=ValueKind("dict"),
                        spread=None,
                        filters=[],
                        start_index=18,
                        end_index=34,
                        line_col=(1, 19),
                    ),
                    is_flag=False,
                    start_index=13,
                    end_index=34,
                    line_col=(1, 14),
                ),
            ],
            is_self_closing=False,
            syntax=TagSyntax("django"),
            start_index=0,
            end_index=37,
            line_col=(1, 4),
        )

        assert tag == expected_tag

        tag_func = compile_tag(tag)
        args, kwargs = tag_func(
            context={},
            variable=lambda ctx, var: ctx[var],
            template_string=lambda ctx, expr: f"TEMPLATE_RESOLVED:{expr}",
            translation=lambda ctx, text: f"TRANSLATION_RESOLVED:{text}",
            filter=lambda ctx, name, value, arg=None: f"{name}({value}, {arg})",
        )

        assert args == []
        assert kwargs == [("data", {"key": "val"})]

    def test_dict_trailing_comma(self):
        tag = parse_tag('{% component data={ "key": "val", } %}')

        expected_tag = Tag(
            name=TagToken(token="component", start_index=3, end_index=12, line_col=(1, 4)),
            attrs=[
                TagAttr(
                    key=TagToken(token="data", start_index=13, end_index=17, line_col=(1, 14)),
                    value=TagValue(
                        token=TagToken(token='{ "key": "val", }', start_index=18, end_index=35, line_col=(1, 19)),
                        children=[
                            TagValue(
                                token=TagToken(token='"key"', start_index=20, end_index=25, line_col=(1, 21)),
                                children=[],
                                kind=ValueKind("string"),
                                spread=None,
                                filters=[],
                                start_index=20,
                                end_index=25,
                                line_col=(1, 21),
                            ),
                            TagValue(
                                token=TagToken(token='"val"', start_index=27, end_index=32, line_col=(1, 28)),
                                children=[],
                                kind=ValueKind("string"),
                                spread=None,
                                filters=[],
                                start_index=27,
                                end_index=32,
                                line_col=(1, 28),
                            ),
                        ],
                        kind=ValueKind("dict"),
                        spread=None,
                        filters=[],
                        start_index=18,
                        end_index=35,
                        line_col=(1, 19),
                    ),
                    is_flag=False,
                    start_index=13,
                    end_index=35,
                    line_col=(1, 14),
                ),
            ],
            is_self_closing=False,
            syntax=TagSyntax("django"),
            start_index=0,
            end_index=38,
            line_col=(1, 4),
        )

        assert tag == expected_tag

        tag_func = compile_tag(tag)
        args, kwargs = tag_func(
            context={},
            variable=lambda ctx, var: ctx[var],
            template_string=lambda ctx, expr: f"TEMPLATE_RESOLVED:{expr}",
            translation=lambda ctx, text: f"TRANSLATION_RESOLVED:{text}",
            filter=lambda ctx, name, value, arg=None: f"{name}({value}, {arg})",
        )

        assert args == []
        assert kwargs == [("data", {"key": "val"})]

    def test_dict_missing_colon(self):
        with pytest.raises(
            SyntaxError,
            match=re.escape("expected filter_noarg or COMMENT"),
        ):
            parse_tag('{% component data={ "key" } %}')

    def test_dict_missing_colon_2(self):
        with pytest.raises(
            SyntaxError,
            match=re.escape("expected filter_chain_noarg or COMMENT"),
        ):
            parse_tag('{% component data={ "key", "val" } %}')

    def test_dict_extra_colon(self):
        with pytest.raises(
            SyntaxError,
            match=re.escape("expected value or COMMENT"),
        ):
            parse_tag("{% component data={ key:: key } %}")

    def test_dict_spread(self):
        tag = parse_tag("{% component data={ **spread } %}")

        expected_tag = Tag(
            name=TagToken(token="component", start_index=3, end_index=12, line_col=(1, 4)),
            attrs=[
                TagAttr(
                    key=TagToken(token="data", start_index=13, end_index=17, line_col=(1, 14)),
                    value=TagValue(
                        token=TagToken(token="{ **spread }", start_index=18, end_index=30, line_col=(1, 19)),
                        children=[
                            TagValue(
                                token=TagToken(token="spread", start_index=22, end_index=28, line_col=(1, 23)),
                                children=[],
                                kind=ValueKind("variable"),
                                spread="**",
                                filters=[],
                                start_index=20,
                                end_index=28,
                                line_col=(1, 21),
                            ),
                        ],
                        kind=ValueKind("dict"),
                        spread=None,
                        filters=[],
                        start_index=18,
                        end_index=30,
                        line_col=(1, 19),
                    ),
                    is_flag=False,
                    start_index=13,
                    end_index=30,
                    line_col=(1, 14),
                ),
            ],
            is_self_closing=False,
            syntax=TagSyntax("django"),
            start_index=0,
            end_index=33,
            line_col=(1, 4),
        )

        assert tag == expected_tag

        tag_func = compile_tag(tag)
        args1, kwargs1 = tag_func(
            context={"spread": {"key": "val"}},
            variable=lambda ctx, var: ctx[var],
            template_string=lambda ctx, expr: f"TEMPLATE_RESOLVED:{expr}",
            translation=lambda ctx, text: f"TRANSLATION_RESOLVED:{text}",
            filter=lambda ctx, name, value, arg=None: f"{name}({value}, {arg})",
        )
        assert args1 == []
        assert kwargs1 == [("data", {"key": "val"})]

        args2, kwargs2 = tag_func(
            context={"spread": {}},
            variable=lambda ctx, var: ctx[var],
            template_string=lambda ctx, expr: f"TEMPLATE_RESOLVED:{expr}",
            translation=lambda ctx, text: f"TRANSLATION_RESOLVED:{text}",
            filter=lambda ctx, name, value, arg=None: f"{name}({value}, {arg})",
        )
        assert args2 == []
        assert kwargs2 == [("data", {})]

        with pytest.raises(
            TypeError,
            match=re.escape("'list' object is not a mapping"),
        ):
            tag_func(
                context={"spread": [1, 2, 3]},
                variable=lambda ctx, var: ctx[var],
                template_string=lambda ctx, expr: f"TEMPLATE_RESOLVED:{expr}",
                translation=lambda ctx, text: f"TRANSLATION_RESOLVED:{text}",
                filter=lambda ctx, name, value, arg=None: f"{name}({value}, {arg})",
            )

        with pytest.raises(
            TypeError,
            match=re.escape("'int' object is not a mapping"),
        ):
            tag_func(
                context={"spread": 3},
                variable=lambda ctx, var: ctx[var],
                template_string=lambda ctx, expr: f"TEMPLATE_RESOLVED:{expr}",
                translation=lambda ctx, text: f"TRANSLATION_RESOLVED:{text}",
                filter=lambda ctx, name, value, arg=None: f"{name}({value}, {arg})",
            )

        with pytest.raises(
            TypeError,
            match=re.escape("'NoneType' object is not a mapping"),
        ):
            tag_func(
                context={"spread": None},
                variable=lambda ctx, var: ctx[var],
                template_string=lambda ctx, expr: f"TEMPLATE_RESOLVED:{expr}",
                translation=lambda ctx, text: f"TRANSLATION_RESOLVED:{text}",
                filter=lambda ctx, name, value, arg=None: f"{name}({value}, {arg})",
            )

    def test_dict_spread_between_key_value_pairs(self):
        tag = parse_tag('{% component data={ "key": val, **spread, "key2": val2 } %}')

        expected_tag = Tag(
            name=TagToken(token="component", start_index=3, end_index=12, line_col=(1, 4)),
            attrs=[
                TagAttr(
                    key=TagToken(token="data", start_index=13, end_index=17, line_col=(1, 14)),
                    value=TagValue(
                        token=TagToken(
                            token='{ "key": val, **spread, "key2": val2 }',
                            start_index=18,
                            end_index=56,
                            line_col=(1, 19),
                        ),
                        children=[
                            TagValue(
                                token=TagToken(token='"key"', start_index=20, end_index=25, line_col=(1, 21)),
                                children=[],
                                kind=ValueKind("string"),
                                spread=None,
                                filters=[],
                                start_index=20,
                                end_index=25,
                                line_col=(1, 21),
                            ),
                            TagValue(
                                token=TagToken(token="val", start_index=27, end_index=30, line_col=(1, 28)),
                                children=[],
                                kind=ValueKind("variable"),
                                spread=None,
                                filters=[],
                                start_index=27,
                                end_index=30,
                                line_col=(1, 28),
                            ),
                            TagValue(
                                token=TagToken(token="spread", start_index=34, end_index=40, line_col=(1, 35)),
                                children=[],
                                kind=ValueKind("variable"),
                                spread="**",
                                filters=[],
                                start_index=32,
                                end_index=40,
                                line_col=(1, 33),
                            ),
                            TagValue(
                                token=TagToken(token='"key2"', start_index=42, end_index=48, line_col=(1, 43)),
                                children=[],
                                kind=ValueKind("string"),
                                spread=None,
                                filters=[],
                                start_index=42,
                                end_index=48,
                                line_col=(1, 43),
                            ),
                            TagValue(
                                token=TagToken(token="val2", start_index=50, end_index=54, line_col=(1, 51)),
                                children=[],
                                kind=ValueKind("variable"),
                                spread=None,
                                filters=[],
                                start_index=50,
                                end_index=54,
                                line_col=(1, 51),
                            ),
                        ],
                        kind=ValueKind("dict"),
                        spread=None,
                        filters=[],
                        start_index=18,
                        end_index=56,
                        line_col=(1, 19),
                    ),
                    is_flag=False,
                    start_index=13,
                    end_index=56,
                    line_col=(1, 14),
                ),
            ],
            is_self_closing=False,
            syntax=TagSyntax("django"),
            start_index=0,
            end_index=59,
            line_col=(1, 4),
        )

        assert tag == expected_tag

        tag_func = compile_tag(tag)
        args1, kwargs1 = tag_func(
            context={"spread": {"a": 1}, "val": "HELLO", "val2": "WORLD"},
            variable=lambda ctx, var: ctx[var],
            template_string=lambda ctx, expr: f"TEMPLATE_RESOLVED:{expr}",
            translation=lambda ctx, text: f"TRANSLATION_RESOLVED:{text}",
            filter=lambda ctx, name, value, arg=None: f"{name}({value}, {arg})",
        )
        assert args1 == []
        assert kwargs1 == [("data", {"key": "HELLO", "a": 1, "key2": "WORLD"})]

    # Test that dictionary keys cannot have filter arguments - The `:` is parsed as dictionary key separator
    # So instead, the content below will be parsed as key `"key"|filter`, and value `"arg":"value"'
    # And the latter is invalid because it's missing the `|` separator.
    def test_colon_in_dictionary_keys(self):
        with pytest.raises(
            SyntaxError,
            match=re.escape("expected filter_chain or COMMENT"),
        ):
            parse_tag('{% component data={"key"|filter:"arg": "value"} %}')

    def test_dicts_complex(self):
        # NOTE: In this example, it looks like e.g. `"e"` should be a filter argument
        # to `c|default`. BUT! variables like `c|default` are inside a dictionary,
        # so the `:` is preferentially interpreted as dictionary key separator (`{key: val}`).
        # So e.g. line `{c|default: "e"|yesno:"yes,no"}`
        # actually means `{<key>: <val>}`,
        # where `<key>` is `c|default` and `val` is `"e"|yesno:"yes,no"`.
        tag = parse_tag(
            """
            {% component
            simple={
                "a": 1|add:2
            }
            nested={
                "key"|upper: val|lower,
                **spread,
                "obj": {"x": 1|add:2}
            }
            filters={
                "a"|lower: "b"|upper,
                c|default: "e"|yesno:"yes,no"
            }
            %}"""
        )

        expected_tag = Tag(
            name=TagToken(token="component", start_index=16, end_index=25, line_col=(2, 16)),
            attrs=[
                TagAttr(
                    key=TagToken(token="simple", start_index=38, end_index=44, line_col=(3, 13)),
                    value=TagValue(
                        token=TagToken(
                            token='{\n                "a": 1|add:2\n            }',
                            start_index=45,
                            end_index=89,
                            line_col=(3, 20),
                        ),
                        children=[
                            TagValue(
                                token=TagToken(token='"a"', start_index=63, end_index=66, line_col=(4, 17)),
                                children=[],
                                kind=ValueKind("string"),
                                spread=None,
                                filters=[],
                                start_index=63,
                                end_index=66,
                                line_col=(4, 17),
                            ),
                            TagValue(
                                token=TagToken(token="1", start_index=68, end_index=69, line_col=(4, 22)),
                                children=[],
                                kind=ValueKind("int"),
                                spread=None,
                                filters=[
                                    TagValueFilter(
                                        token=TagToken(token="add", start_index=70, end_index=73, line_col=(4, 24)),
                                        arg=TagValue(
                                            token=TagToken(token="2", start_index=74, end_index=75, line_col=(4, 28)),
                                            children=[],
                                            kind=ValueKind("int"),
                                            spread=None,
                                            filters=[],
                                            start_index=73,
                                            end_index=75,
                                            line_col=(4, 27),
                                        ),
                                        start_index=69,
                                        end_index=75,
                                        line_col=(4, 23),
                                    )
                                ],
                                start_index=68,
                                end_index=75,
                                line_col=(4, 22),
                            ),
                        ],
                        kind=ValueKind("dict"),
                        spread=None,
                        filters=[],
                        start_index=45,
                        end_index=89,
                        line_col=(3, 20),
                    ),
                    is_flag=False,
                    start_index=38,
                    end_index=89,
                    line_col=(3, 13),
                ),
                TagAttr(
                    key=TagToken(token="nested", start_index=102, end_index=108, line_col=(6, 13)),
                    value=TagValue(
                        token=TagToken(
                            token='{\n                "key"|upper: val|lower,\n                **spread,\n                "obj": {"x": 1|add:2}\n            }',
                            start_index=109,
                            end_index=228,
                            line_col=(6, 20),
                        ),
                        children=[
                            TagValue(
                                token=TagToken(token='"key"', start_index=127, end_index=132, line_col=(7, 17)),
                                children=[],
                                kind=ValueKind("string"),
                                spread=None,
                                filters=[
                                    TagValueFilter(
                                        token=TagToken(
                                            token="upper", start_index=133, end_index=138, line_col=(7, 23)
                                        ),
                                        arg=None,
                                        start_index=132,
                                        end_index=138,
                                        line_col=(7, 22),
                                    )
                                ],
                                start_index=127,
                                end_index=138,
                                line_col=(7, 17),
                            ),
                            TagValue(
                                token=TagToken(token="val", start_index=140, end_index=143, line_col=(7, 30)),
                                children=[],
                                kind=ValueKind("variable"),
                                spread=None,
                                filters=[
                                    TagValueFilter(
                                        token=TagToken(
                                            token="lower", start_index=144, end_index=149, line_col=(7, 34)
                                        ),
                                        arg=None,
                                        start_index=143,
                                        end_index=149,
                                        line_col=(7, 33),
                                    )
                                ],
                                start_index=140,
                                end_index=149,
                                line_col=(7, 30),
                            ),
                            TagValue(
                                token=TagToken(token="spread", start_index=169, end_index=175, line_col=(8, 19)),
                                children=[],
                                kind=ValueKind("variable"),
                                spread="**",
                                filters=[],
                                start_index=167,
                                end_index=175,
                                line_col=(8, 17),
                            ),
                            TagValue(
                                token=TagToken(token='"obj"', start_index=193, end_index=198, line_col=(9, 17)),
                                children=[],
                                kind=ValueKind("string"),
                                spread=None,
                                filters=[],
                                start_index=193,
                                end_index=198,
                                line_col=(9, 17),
                            ),
                            TagValue(
                                token=TagToken(
                                    token='{"x": 1|add:2}', start_index=200, end_index=214, line_col=(9, 24)
                                ),
                                children=[
                                    TagValue(
                                        token=TagToken(token='"x"', start_index=201, end_index=204, line_col=(9, 25)),
                                        children=[],
                                        kind=ValueKind("string"),
                                        spread=None,
                                        filters=[],
                                        start_index=201,
                                        end_index=204,
                                        line_col=(9, 25),
                                    ),
                                    TagValue(
                                        token=TagToken(token="1", start_index=206, end_index=207, line_col=(9, 30)),
                                        children=[],
                                        kind=ValueKind("int"),
                                        spread=None,
                                        filters=[
                                            TagValueFilter(
                                                token=TagToken(
                                                    token="add", start_index=208, end_index=211, line_col=(9, 32)
                                                ),
                                                arg=TagValue(
                                                    token=TagToken(
                                                        token="2", start_index=212, end_index=213, line_col=(9, 36)
                                                    ),
                                                    children=[],
                                                    kind=ValueKind("int"),
                                                    spread=None,
                                                    filters=[],
                                                    start_index=211,
                                                    end_index=213,
                                                    line_col=(9, 35),
                                                ),
                                                start_index=207,
                                                end_index=213,
                                                line_col=(9, 31),
                                            )
                                        ],
                                        start_index=206,
                                        end_index=213,
                                        line_col=(9, 30),
                                    ),
                                ],
                                kind=ValueKind("dict"),
                                spread=None,
                                filters=[],
                                start_index=200,
                                end_index=214,
                                line_col=(9, 24),
                            ),
                        ],
                        kind=ValueKind("dict"),
                        spread=None,
                        filters=[],
                        start_index=109,
                        end_index=228,
                        line_col=(6, 20),
                    ),
                    is_flag=False,
                    start_index=102,
                    end_index=228,
                    line_col=(6, 13),
                ),
                TagAttr(
                    key=TagToken(token="filters", start_index=241, end_index=248, line_col=(11, 13)),
                    value=TagValue(
                        token=TagToken(
                            token='{\n                "a"|lower: "b"|upper,\n                c|default: "e"|yesno:"yes,no"\n            }',
                            start_index=249,
                            end_index=348,
                            line_col=(11, 21),
                        ),
                        children=[
                            TagValue(
                                token=TagToken(token='"a"', start_index=267, end_index=270, line_col=(12, 17)),
                                children=[],
                                kind=ValueKind("string"),
                                spread=None,
                                filters=[
                                    TagValueFilter(
                                        token=TagToken(
                                            token="lower", start_index=271, end_index=276, line_col=(12, 21)
                                        ),
                                        arg=None,
                                        start_index=270,
                                        end_index=276,
                                        line_col=(12, 20),
                                    )
                                ],
                                start_index=267,
                                end_index=276,
                                line_col=(12, 17),
                            ),
                            TagValue(
                                token=TagToken(token='"b"', start_index=278, end_index=281, line_col=(12, 28)),
                                children=[],
                                kind=ValueKind("string"),
                                spread=None,
                                filters=[
                                    TagValueFilter(
                                        token=TagToken(
                                            token="upper", start_index=282, end_index=287, line_col=(12, 32)
                                        ),
                                        arg=None,
                                        start_index=281,
                                        end_index=287,
                                        line_col=(12, 31),
                                    )
                                ],
                                start_index=278,
                                end_index=287,
                                line_col=(12, 28),
                            ),
                            TagValue(
                                token=TagToken(token="c", start_index=305, end_index=306, line_col=(13, 17)),
                                children=[],
                                kind=ValueKind("variable"),
                                spread=None,
                                filters=[
                                    TagValueFilter(
                                        token=TagToken(
                                            token="default", start_index=307, end_index=314, line_col=(13, 19)
                                        ),
                                        arg=None,
                                        start_index=306,
                                        end_index=314,
                                        line_col=(13, 18),
                                    )
                                ],
                                start_index=305,
                                end_index=314,
                                line_col=(13, 17),
                            ),
                            TagValue(
                                token=TagToken(token='"e"', start_index=316, end_index=319, line_col=(13, 28)),
                                children=[],
                                kind=ValueKind("string"),
                                spread=None,
                                filters=[
                                    TagValueFilter(
                                        token=TagToken(
                                            token="yesno", start_index=320, end_index=325, line_col=(13, 32)
                                        ),
                                        arg=TagValue(
                                            token=TagToken(
                                                token='"yes,no"', start_index=326, end_index=334, line_col=(13, 38)
                                            ),
                                            children=[],
                                            kind=ValueKind("string"),
                                            spread=None,
                                            filters=[],
                                            start_index=325,
                                            end_index=334,
                                            line_col=(13, 37),
                                        ),
                                        start_index=319,
                                        end_index=334,
                                        line_col=(13, 31),
                                    )
                                ],
                                start_index=316,
                                end_index=334,
                                line_col=(13, 28),
                            ),
                        ],
                        kind=ValueKind("dict"),
                        spread=None,
                        filters=[],
                        start_index=249,
                        end_index=348,
                        line_col=(11, 21),
                    ),
                    is_flag=False,
                    start_index=241,
                    end_index=348,
                    line_col=(11, 13),
                ),
            ],
            is_self_closing=False,
            syntax=TagSyntax("django"),
            start_index=0,
            end_index=363,
            line_col=(2, 16),
        )

        assert tag == expected_tag

        tag_func = compile_tag(tag)
        args1, kwargs1 = tag_func(
            context={"spread": {6: 7}, "c": None, "val": "bar"},
            variable=lambda ctx, var: ctx[var],
            template_string=lambda ctx, expr: f"TEMPLATE_RESOLVED:{expr}",
            translation=lambda ctx, text: f"TRANSLATION_RESOLVED:{text}",
            filter=lambda ctx, name, value, arg=None: f"{name}({value}, {arg})",
        )
        assert args1 == []
        assert kwargs1 == [
            ("simple", {"a": "add(1, 2)"}),
            ("nested", {"upper(key, None)": "lower(bar, None)", 6: 7, "obj": {"x": "add(1, 2)"}}),
            ("filters", {"lower(a, None)": "upper(b, None)", "default(None, None)": "yesno(e, yes,no)"}),
        ]


class TestList:
    def test_list_simple(self):
        tag = parse_tag("{% component data=[1, 2, 3] %}")

        expected_tag = Tag(
            name=TagToken(token="component", start_index=3, end_index=12, line_col=(1, 4)),
            attrs=[
                TagAttr(
                    key=TagToken(token="data", start_index=13, end_index=17, line_col=(1, 14)),
                    value=TagValue(
                        token=TagToken(token="[1, 2, 3]", start_index=18, end_index=27, line_col=(1, 19)),
                        children=[
                            TagValue(
                                token=TagToken(token="1", start_index=19, end_index=20, line_col=(1, 20)),
                                children=[],
                                kind=ValueKind("int"),
                                spread=None,
                                filters=[],
                                start_index=19,
                                end_index=20,
                                line_col=(1, 20),
                            ),
                            TagValue(
                                token=TagToken(token="2", start_index=22, end_index=23, line_col=(1, 23)),
                                children=[],
                                kind=ValueKind("int"),
                                spread=None,
                                filters=[],
                                start_index=22,
                                end_index=23,
                                line_col=(1, 23),
                            ),
                            TagValue(
                                token=TagToken(token="3", start_index=25, end_index=26, line_col=(1, 26)),
                                children=[],
                                kind=ValueKind("int"),
                                spread=None,
                                filters=[],
                                start_index=25,
                                end_index=26,
                                line_col=(1, 26),
                            ),
                        ],
                        kind=ValueKind("list"),
                        spread=None,
                        filters=[],
                        start_index=18,
                        end_index=27,
                        line_col=(1, 19),
                    ),
                    is_flag=False,
                    start_index=13,
                    end_index=27,
                    line_col=(1, 14),
                ),
            ],
            is_self_closing=False,
            syntax=TagSyntax("django"),
            start_index=0,
            end_index=30,
            line_col=(1, 4),
        )

        assert tag == expected_tag

        tag_func = compile_tag(tag)
        args1, kwargs1 = tag_func(
            context={},
            variable=lambda ctx, var: ctx[var],
            template_string=lambda ctx, expr: f"TEMPLATE_RESOLVED:{expr}",
            translation=lambda ctx, text: f"TRANSLATION_RESOLVED:{text}",
            filter=lambda ctx, name, value, arg=None: f"{name}({value}, {arg})",
        )
        assert args1 == []
        assert kwargs1 == [("data", [1, 2, 3])]

    def test_list_trailing_comma(self):
        tag = parse_tag("{% component data=[1, 2, 3, ] %}")

        expected_tag = Tag(
            name=TagToken(token="component", start_index=3, end_index=12, line_col=(1, 4)),
            attrs=[
                TagAttr(
                    key=TagToken(token="data", start_index=13, end_index=17, line_col=(1, 14)),
                    value=TagValue(
                        token=TagToken(token="[1, 2, 3, ]", start_index=18, end_index=29, line_col=(1, 19)),
                        children=[
                            TagValue(
                                token=TagToken(token="1", start_index=19, end_index=20, line_col=(1, 20)),
                                children=[],
                                kind=ValueKind("int"),
                                spread=None,
                                filters=[],
                                start_index=19,
                                end_index=20,
                                line_col=(1, 20),
                            ),
                            TagValue(
                                token=TagToken(token="2", start_index=22, end_index=23, line_col=(1, 23)),
                                children=[],
                                kind=ValueKind("int"),
                                spread=None,
                                filters=[],
                                start_index=22,
                                end_index=23,
                                line_col=(1, 23),
                            ),
                            TagValue(
                                token=TagToken(token="3", start_index=25, end_index=26, line_col=(1, 26)),
                                children=[],
                                kind=ValueKind("int"),
                                spread=None,
                                filters=[],
                                start_index=25,
                                end_index=26,
                                line_col=(1, 26),
                            ),
                        ],
                        kind=ValueKind("list"),
                        spread=None,
                        filters=[],
                        start_index=18,
                        end_index=29,
                        line_col=(1, 19),
                    ),
                    is_flag=False,
                    start_index=13,
                    end_index=29,
                    line_col=(1, 14),
                ),
            ],
            is_self_closing=False,
            syntax=TagSyntax("django"),
            start_index=0,
            end_index=32,
            line_col=(1, 4),
        )

        assert tag == expected_tag

        tag_func = compile_tag(tag)
        args1, kwargs1 = tag_func(
            context={},
            variable=lambda ctx, var: ctx[var],
            template_string=lambda ctx, expr: f"TEMPLATE_RESOLVED:{expr}",
            translation=lambda ctx, text: f"TRANSLATION_RESOLVED:{text}",
            filter=lambda ctx, name, value, arg=None: f"{name}({value}, {arg})",
        )
        assert args1 == []
        assert kwargs1 == [("data", [1, 2, 3])]

    def test_lists_complex(self):
        tag = parse_tag(
            """
                {% component
                nums=[
                    1,
                    2|add:3,
                    *spread
                ]
                items=[
                    "a"|upper,
                    'b'|lower,
                    c|default:"d"
                ]
                mixed=[
                    1,
                    [*nested],
                    {"key": "val"}
                ]
            %}"""
        )

        expected_tag = Tag(
            name=TagToken(token="component", start_index=20, end_index=29, line_col=(2, 20)),
            attrs=[
                TagAttr(
                    key=TagToken(token="nums", start_index=46, end_index=50, line_col=(3, 17)),
                    value=TagValue(
                        token=TagToken(
                            token="[\n                    1,\n                    2|add:3,\n                    *spread\n                ]",
                            start_index=51,
                            end_index=150,
                            line_col=(3, 22),
                        ),
                        children=[
                            TagValue(
                                token=TagToken(token="1", start_index=73, end_index=74, line_col=(4, 21)),
                                children=[],
                                kind=ValueKind("int"),
                                spread=None,
                                filters=[],
                                start_index=73,
                                end_index=74,
                                line_col=(4, 21),
                            ),
                            TagValue(
                                token=TagToken(token="2", start_index=96, end_index=97, line_col=(5, 21)),
                                children=[],
                                kind=ValueKind("int"),
                                spread=None,
                                filters=[
                                    TagValueFilter(
                                        token=TagToken(token="add", start_index=98, end_index=101, line_col=(5, 23)),
                                        arg=TagValue(
                                            token=TagToken(
                                                token="3", start_index=102, end_index=103, line_col=(5, 27)
                                            ),
                                            children=[],
                                            kind=ValueKind("int"),
                                            spread=None,
                                            filters=[],
                                            start_index=101,
                                            end_index=103,
                                            line_col=(5, 26),
                                        ),
                                        start_index=97,
                                        end_index=103,
                                        line_col=(5, 22),
                                    )
                                ],
                                start_index=96,
                                end_index=103,
                                line_col=(5, 21),
                            ),
                            TagValue(
                                token=TagToken(token="spread", start_index=126, end_index=132, line_col=(6, 22)),
                                children=[],
                                kind=ValueKind("variable"),
                                spread="*",
                                filters=[],
                                start_index=125,
                                end_index=132,
                                line_col=(6, 21),
                            ),
                        ],
                        kind=ValueKind("list"),
                        spread=None,
                        filters=[],
                        start_index=51,
                        end_index=150,
                        line_col=(3, 22),
                    ),
                    is_flag=False,
                    start_index=46,
                    end_index=150,
                    line_col=(3, 17),
                ),
                TagAttr(
                    key=TagToken(token="items", start_index=167, end_index=172, line_col=(8, 17)),
                    value=TagValue(
                        token=TagToken(
                            token='[\n                    "a"|upper,\n                    \'b\'|lower,\n                    c|default:"d"\n                ]',
                            start_index=173,
                            end_index=288,
                            line_col=(8, 23),
                        ),
                        children=[
                            TagValue(
                                token=TagToken(token='"a"', start_index=195, end_index=198, line_col=(9, 21)),
                                children=[],
                                kind=ValueKind("string"),
                                spread=None,
                                filters=[
                                    TagValueFilter(
                                        token=TagToken(
                                            token="upper", start_index=199, end_index=204, line_col=(9, 25)
                                        ),
                                        arg=None,
                                        start_index=198,
                                        end_index=204,
                                        line_col=(9, 24),
                                    )
                                ],
                                start_index=195,
                                end_index=204,
                                line_col=(9, 21),
                            ),
                            TagValue(
                                token=TagToken(token="'b'", start_index=226, end_index=229, line_col=(10, 21)),
                                children=[],
                                kind=ValueKind("string"),
                                spread=None,
                                filters=[
                                    TagValueFilter(
                                        token=TagToken(
                                            token="lower", start_index=230, end_index=235, line_col=(10, 25)
                                        ),
                                        arg=None,
                                        start_index=229,
                                        end_index=235,
                                        line_col=(10, 24),
                                    )
                                ],
                                start_index=226,
                                end_index=235,
                                line_col=(10, 21),
                            ),
                            TagValue(
                                token=TagToken(token="c", start_index=257, end_index=258, line_col=(11, 21)),
                                children=[],
                                kind=ValueKind("variable"),
                                spread=None,
                                filters=[
                                    TagValueFilter(
                                        token=TagToken(
                                            token="default", start_index=259, end_index=266, line_col=(11, 23)
                                        ),
                                        arg=TagValue(
                                            token=TagToken(
                                                token='"d"', start_index=267, end_index=270, line_col=(11, 31)
                                            ),
                                            children=[],
                                            kind=ValueKind("string"),
                                            spread=None,
                                            filters=[],
                                            start_index=266,
                                            end_index=270,
                                            line_col=(11, 30),
                                        ),
                                        start_index=258,
                                        end_index=270,
                                        line_col=(11, 22),
                                    )
                                ],
                                start_index=257,
                                end_index=270,
                                line_col=(11, 21),
                            ),
                        ],
                        kind=ValueKind("list"),
                        spread=None,
                        filters=[],
                        start_index=173,
                        end_index=288,
                        line_col=(8, 23),
                    ),
                    is_flag=False,
                    start_index=167,
                    end_index=288,
                    line_col=(8, 17),
                ),
                TagAttr(
                    key=TagToken(token="mixed", start_index=305, end_index=310, line_col=(13, 17)),
                    value=TagValue(
                        token=TagToken(
                            token='[\n                    1,\n                    [*nested],\n                    {"key": "val"}\n                ]',
                            start_index=311,
                            end_index=419,
                            line_col=(13, 23),
                        ),
                        children=[
                            TagValue(
                                token=TagToken(token="1", start_index=333, end_index=334, line_col=(14, 21)),
                                children=[],
                                kind=ValueKind("int"),
                                spread=None,
                                filters=[],
                                start_index=333,
                                end_index=334,
                                line_col=(14, 21),
                            ),
                            TagValue(
                                token=TagToken(token="[*nested]", start_index=356, end_index=365, line_col=(15, 21)),
                                children=[
                                    TagValue(
                                        token=TagToken(
                                            token="nested", start_index=358, end_index=364, line_col=(15, 23)
                                        ),
                                        children=[],
                                        kind=ValueKind("variable"),
                                        spread="*",
                                        filters=[],
                                        start_index=357,
                                        end_index=364,
                                        line_col=(15, 22),
                                    )
                                ],
                                kind=ValueKind("list"),
                                spread=None,
                                filters=[],
                                start_index=356,
                                end_index=365,
                                line_col=(15, 21),
                            ),
                            TagValue(
                                token=TagToken(
                                    token='{"key": "val"}', start_index=387, end_index=401, line_col=(16, 21)
                                ),
                                children=[
                                    TagValue(
                                        token=TagToken(
                                            token='"key"', start_index=388, end_index=393, line_col=(16, 22)
                                        ),
                                        children=[],
                                        kind=ValueKind("string"),
                                        spread=None,
                                        filters=[],
                                        start_index=388,
                                        end_index=393,
                                        line_col=(16, 22),
                                    ),
                                    TagValue(
                                        token=TagToken(
                                            token='"val"', start_index=395, end_index=400, line_col=(16, 29)
                                        ),
                                        children=[],
                                        kind=ValueKind("string"),
                                        spread=None,
                                        filters=[],
                                        start_index=395,
                                        end_index=400,
                                        line_col=(16, 29),
                                    ),
                                ],
                                kind=ValueKind("dict"),
                                spread=None,
                                filters=[],
                                start_index=387,
                                end_index=401,
                                line_col=(16, 21),
                            ),
                        ],
                        kind=ValueKind("list"),
                        spread=None,
                        filters=[],
                        start_index=311,
                        end_index=419,
                        line_col=(13, 23),
                    ),
                    is_flag=False,
                    start_index=305,
                    end_index=419,
                    line_col=(13, 17),
                ),
            ],
            is_self_closing=False,
            syntax=TagSyntax("django"),
            start_index=0,
            end_index=434,
            line_col=(2, 20),
        )

        assert tag == expected_tag

        tag_func = compile_tag(tag)
        args1, kwargs1 = tag_func(
            context={"nested": [1, 2, 3], "spread": [5, 6], "c": None},
            variable=lambda ctx, var: ctx[var],
            template_string=lambda ctx, expr: f"TEMPLATE_RESOLVED:{expr}",
            translation=lambda ctx, text: f"TRANSLATION_RESOLVED:{text}",
            filter=lambda ctx, name, value, arg=None: f"{name}({value}, {arg})",
        )
        assert args1 == []
        assert kwargs1 == [
            ("nums", [1, "add(2, 3)", 5, 6]),
            ("items", ["upper(a, None)", "lower(b, None)", "default(None, d)"]),
            ("mixed", [1, [1, 2, 3], {"key": "val"}]),
        ]

    def test_mixed_complex(self):
        tag = parse_tag(
            """
            {% component
            data={
                "items": [
                    1|add:2,
                    {"x"|upper: 2|add:3},
                    *spread_items|default:""
                ],
                "nested": {
                    "a": [
                        1|add:2,
                        *nums|default:""
                    ],
                    "b": {
                        "x": [
                            *more|default:""
                        ]
                    }
                },
                **rest|injectd,
                "key": _('value')|upper
            }
            %}"""
        )

        expected_tag = Tag(
            name=TagToken(token="component", start_index=16, end_index=25, line_col=(2, 16)),
            attrs=[
                TagAttr(
                    key=TagToken(token="data", start_index=38, end_index=42, line_col=(3, 13)),
                    value=TagValue(
                        token=TagToken(
                            token='{\n                "items": [\n                    1|add:2,\n                    {"x"|upper: 2|add:3},\n                    *spread_items|default:""\n                ],\n                "nested": {\n                    "a": [\n                        1|add:2,\n                        *nums|default:""\n                    ],\n                    "b": {\n                        "x": [\n                            *more|default:""\n                        ]\n                    }\n                },\n                **rest|injectd,\n                "key": _(\'value\')|upper\n            }',
                            start_index=43,
                            end_index=614,
                            line_col=(3, 18),
                        ),
                        children=[
                            TagValue(
                                token=TagToken(token='"items"', start_index=61, end_index=68, line_col=(4, 17)),
                                children=[],
                                kind=ValueKind("string"),
                                spread=None,
                                filters=[],
                                start_index=61,
                                end_index=68,
                                line_col=(4, 17),
                            ),
                            TagValue(
                                token=TagToken(
                                    token='[\n                    1|add:2,\n                    {"x"|upper: 2|add:3},\n                    *spread_items|default:""\n                ]',
                                    start_index=70,
                                    end_index=205,
                                    line_col=(4, 26),
                                ),
                                children=[
                                    TagValue(
                                        token=TagToken(token="1", start_index=92, end_index=93, line_col=(5, 21)),
                                        children=[],
                                        kind=ValueKind("int"),
                                        spread=None,
                                        filters=[
                                            TagValueFilter(
                                                token=TagToken(
                                                    token="add", start_index=94, end_index=97, line_col=(5, 23)
                                                ),
                                                arg=TagValue(
                                                    token=TagToken(
                                                        token="2", start_index=98, end_index=99, line_col=(5, 27)
                                                    ),
                                                    children=[],
                                                    kind=ValueKind("int"),
                                                    spread=None,
                                                    filters=[],
                                                    start_index=97,
                                                    end_index=99,
                                                    line_col=(5, 26),
                                                ),
                                                start_index=93,
                                                end_index=99,
                                                line_col=(5, 22),
                                            )
                                        ],
                                        start_index=92,
                                        end_index=99,
                                        line_col=(5, 21),
                                    ),
                                    TagValue(
                                        token=TagToken(
                                            token='{"x"|upper: 2|add:3}',
                                            start_index=121,
                                            end_index=141,
                                            line_col=(6, 21),
                                        ),
                                        children=[
                                            TagValue(
                                                token=TagToken(
                                                    token='"x"', start_index=122, end_index=125, line_col=(6, 22)
                                                ),
                                                children=[],
                                                kind=ValueKind("string"),
                                                spread=None,
                                                filters=[
                                                    TagValueFilter(
                                                        token=TagToken(
                                                            token="upper",
                                                            start_index=126,
                                                            end_index=131,
                                                            line_col=(6, 26),
                                                        ),
                                                        arg=None,
                                                        start_index=125,
                                                        end_index=131,
                                                        line_col=(6, 25),
                                                    )
                                                ],
                                                start_index=122,
                                                end_index=131,
                                                line_col=(6, 22),
                                            ),
                                            TagValue(
                                                token=TagToken(
                                                    token="2", start_index=133, end_index=134, line_col=(6, 33)
                                                ),
                                                children=[],
                                                kind=ValueKind("int"),
                                                spread=None,
                                                filters=[
                                                    TagValueFilter(
                                                        token=TagToken(
                                                            token="add",
                                                            start_index=135,
                                                            end_index=138,
                                                            line_col=(6, 35),
                                                        ),
                                                        arg=TagValue(
                                                            token=TagToken(
                                                                token="3",
                                                                start_index=139,
                                                                end_index=140,
                                                                line_col=(6, 39),
                                                            ),
                                                            children=[],
                                                            kind=ValueKind("int"),
                                                            spread=None,
                                                            filters=[],
                                                            start_index=138,
                                                            end_index=140,
                                                            line_col=(6, 38),
                                                        ),
                                                        start_index=134,
                                                        end_index=140,
                                                        line_col=(6, 34),
                                                    )
                                                ],
                                                start_index=133,
                                                end_index=140,
                                                line_col=(6, 33),
                                            ),
                                        ],
                                        kind=ValueKind("dict"),
                                        spread=None,
                                        filters=[],
                                        start_index=121,
                                        end_index=141,
                                        line_col=(6, 21),
                                    ),
                                    TagValue(
                                        token=TagToken(
                                            token="spread_items", start_index=164, end_index=176, line_col=(7, 22)
                                        ),
                                        children=[],
                                        kind=ValueKind("variable"),
                                        spread="*",
                                        filters=[
                                            TagValueFilter(
                                                token=TagToken(
                                                    token="default", start_index=177, end_index=184, line_col=(7, 35)
                                                ),
                                                arg=TagValue(
                                                    token=TagToken(
                                                        token='""', start_index=185, end_index=187, line_col=(7, 43)
                                                    ),
                                                    children=[],
                                                    kind=ValueKind("string"),
                                                    spread=None,
                                                    filters=[],
                                                    start_index=184,
                                                    end_index=187,
                                                    line_col=(7, 42),
                                                ),
                                                start_index=176,
                                                end_index=187,
                                                line_col=(7, 34),
                                            )
                                        ],
                                        start_index=163,
                                        end_index=187,
                                        line_col=(7, 21),
                                    ),
                                ],
                                kind=ValueKind("list"),
                                spread=None,
                                filters=[],
                                start_index=70,
                                end_index=205,
                                line_col=(4, 26),
                            ),
                            TagValue(
                                token=TagToken(token='"nested"', start_index=223, end_index=231, line_col=(9, 17)),
                                children=[],
                                kind=ValueKind("string"),
                                spread=None,
                                filters=[],
                                start_index=223,
                                end_index=231,
                                line_col=(9, 17),
                            ),
                            TagValue(
                                token=TagToken(
                                    token='{\n                    "a": [\n                        1|add:2,\n                        *nums|default:""\n                    ],\n                    "b": {\n                        "x": [\n                            *more|default:""\n                        ]\n                    }\n                }',
                                    start_index=233,
                                    end_index=527,
                                    line_col=(9, 27),
                                ),
                                children=[
                                    TagValue(
                                        token=TagToken(token='"a"', start_index=255, end_index=258, line_col=(10, 21)),
                                        children=[],
                                        kind=ValueKind("string"),
                                        spread=None,
                                        filters=[],
                                        start_index=255,
                                        end_index=258,
                                        line_col=(10, 21),
                                    ),
                                    TagValue(
                                        token=TagToken(
                                            token='[\n                        1|add:2,\n                        *nums|default:""\n                    ]',
                                            start_index=260,
                                            end_index=357,
                                            line_col=(10, 26),
                                        ),
                                        children=[
                                            TagValue(
                                                token=TagToken(
                                                    token="1", start_index=286, end_index=287, line_col=(11, 25)
                                                ),
                                                children=[],
                                                kind=ValueKind("int"),
                                                spread=None,
                                                filters=[
                                                    TagValueFilter(
                                                        token=TagToken(
                                                            token="add",
                                                            start_index=288,
                                                            end_index=291,
                                                            line_col=(11, 27),
                                                        ),
                                                        arg=TagValue(
                                                            token=TagToken(
                                                                token="2",
                                                                start_index=292,
                                                                end_index=293,
                                                                line_col=(11, 31),
                                                            ),
                                                            children=[],
                                                            kind=ValueKind("int"),
                                                            spread=None,
                                                            filters=[],
                                                            start_index=291,
                                                            end_index=293,
                                                            line_col=(11, 30),
                                                        ),
                                                        start_index=287,
                                                        end_index=293,
                                                        line_col=(11, 26),
                                                    )
                                                ],
                                                start_index=286,
                                                end_index=293,
                                                line_col=(11, 25),
                                            ),
                                            TagValue(
                                                token=TagToken(
                                                    token="nums", start_index=320, end_index=324, line_col=(12, 26)
                                                ),
                                                children=[],
                                                kind=ValueKind("variable"),
                                                spread="*",
                                                filters=[
                                                    TagValueFilter(
                                                        token=TagToken(
                                                            token="default",
                                                            start_index=325,
                                                            end_index=332,
                                                            line_col=(12, 31),
                                                        ),
                                                        arg=TagValue(
                                                            token=TagToken(
                                                                token='""',
                                                                start_index=333,
                                                                end_index=335,
                                                                line_col=(12, 39),
                                                            ),
                                                            children=[],
                                                            kind=ValueKind("string"),
                                                            spread=None,
                                                            filters=[],
                                                            start_index=332,
                                                            end_index=335,
                                                            line_col=(12, 38),
                                                        ),
                                                        start_index=324,
                                                        end_index=335,
                                                        line_col=(12, 30),
                                                    )
                                                ],
                                                start_index=319,
                                                end_index=335,
                                                line_col=(12, 25),
                                            ),
                                        ],
                                        kind=ValueKind("list"),
                                        spread=None,
                                        filters=[],
                                        start_index=260,
                                        end_index=357,
                                        line_col=(10, 26),
                                    ),
                                    TagValue(
                                        token=TagToken(token='"b"', start_index=379, end_index=382, line_col=(14, 21)),
                                        children=[],
                                        kind=ValueKind("string"),
                                        spread=None,
                                        filters=[],
                                        start_index=379,
                                        end_index=382,
                                        line_col=(14, 21),
                                    ),
                                    TagValue(
                                        token=TagToken(
                                            token='{\n                        "x": [\n                            *more|default:""\n                        ]\n                    }',
                                            start_index=384,
                                            end_index=509,
                                            line_col=(14, 26),
                                        ),
                                        children=[
                                            TagValue(
                                                token=TagToken(
                                                    token='"x"', start_index=410, end_index=413, line_col=(15, 25)
                                                ),
                                                children=[],
                                                kind=ValueKind("string"),
                                                spread=None,
                                                filters=[],
                                                start_index=410,
                                                end_index=413,
                                                line_col=(15, 25),
                                            ),
                                            TagValue(
                                                token=TagToken(
                                                    token='[\n                            *more|default:""\n                        ]',
                                                    start_index=415,
                                                    end_index=487,
                                                    line_col=(15, 30),
                                                ),
                                                children=[
                                                    TagValue(
                                                        token=TagToken(
                                                            token="more",
                                                            start_index=446,
                                                            end_index=450,
                                                            line_col=(16, 30),
                                                        ),
                                                        children=[],
                                                        kind=ValueKind("variable"),
                                                        spread="*",
                                                        filters=[
                                                            TagValueFilter(
                                                                token=TagToken(
                                                                    token="default",
                                                                    start_index=451,
                                                                    end_index=458,
                                                                    line_col=(16, 35),
                                                                ),
                                                                arg=TagValue(
                                                                    token=TagToken(
                                                                        token='""',
                                                                        start_index=459,
                                                                        end_index=461,
                                                                        line_col=(16, 43),
                                                                    ),
                                                                    children=[],
                                                                    kind=ValueKind("string"),
                                                                    spread=None,
                                                                    filters=[],
                                                                    start_index=458,
                                                                    end_index=461,
                                                                    line_col=(16, 42),
                                                                ),
                                                                start_index=450,
                                                                end_index=461,
                                                                line_col=(16, 34),
                                                            )
                                                        ],
                                                        start_index=445,
                                                        end_index=461,
                                                        line_col=(16, 29),
                                                    ),
                                                ],
                                                kind=ValueKind("list"),
                                                spread=None,
                                                filters=[],
                                                start_index=415,
                                                end_index=487,
                                                line_col=(15, 30),
                                            ),
                                        ],
                                        kind=ValueKind("dict"),
                                        spread=None,
                                        filters=[],
                                        start_index=384,
                                        end_index=509,
                                        line_col=(14, 26),
                                    ),
                                ],
                                kind=ValueKind("dict"),
                                spread=None,
                                filters=[],
                                start_index=233,
                                end_index=527,
                                line_col=(9, 27),
                            ),
                            TagValue(
                                token=TagToken(token="rest", start_index=547, end_index=551, line_col=(20, 19)),
                                children=[],
                                kind=ValueKind("variable"),
                                spread="**",
                                filters=[
                                    TagValueFilter(
                                        token=TagToken(
                                            token="injectd", start_index=552, end_index=559, line_col=(20, 24)
                                        ),
                                        arg=None,
                                        start_index=551,
                                        end_index=559,
                                        line_col=(20, 23),
                                    )
                                ],
                                start_index=545,
                                end_index=559,
                                line_col=(20, 17),
                            ),
                            TagValue(
                                token=TagToken(token='"key"', start_index=577, end_index=582, line_col=(21, 17)),
                                children=[],
                                kind=ValueKind("string"),
                                spread=None,
                                filters=[],
                                start_index=577,
                                end_index=582,
                                line_col=(21, 17),
                            ),
                            TagValue(
                                token=TagToken(token="_('value')", start_index=584, end_index=594, line_col=(21, 24)),
                                children=[],
                                kind=ValueKind("translation"),
                                spread=None,
                                filters=[
                                    TagValueFilter(
                                        token=TagToken(
                                            token="upper", start_index=595, end_index=600, line_col=(21, 35)
                                        ),
                                        arg=None,
                                        start_index=594,
                                        end_index=600,
                                        line_col=(21, 34),
                                    )
                                ],
                                start_index=584,
                                end_index=600,
                                line_col=(21, 24),
                            ),
                        ],
                        kind=ValueKind("dict"),
                        spread=None,
                        filters=[],
                        start_index=43,
                        end_index=614,
                        line_col=(3, 18),
                    ),
                    is_flag=False,
                    start_index=38,
                    end_index=614,
                    line_col=(3, 13),
                ),
            ],
            is_self_closing=False,
            syntax=TagSyntax("django"),
            start_index=0,
            end_index=629,
            line_col=(2, 16),
        )

        assert tag == expected_tag

        tag_func = compile_tag(tag)
        args1, kwargs1 = tag_func(
            context={"spread_items": None, "nums": [1, 2, 3], "more": "x", "rest": {"a": "b"}},
            variable=lambda ctx, var: ctx[var],
            template_string=lambda ctx, expr: f"TEMPLATE_RESOLVED:{expr}",
            translation=lambda ctx, text: f"TRANSLATION_RESOLVED:{text}",
            filter=lambda ctx, name, value, arg=None: {**value, "injected": True}
            if name == "injectd"
            else f"{name}({value}, {arg})",
        )
        assert args1 == []
        assert kwargs1 == [
            (
                "data",
                {
                    "items": [
                        "add(1, 2)",
                        {"upper(x, None)": "add(2, 3)"},
                        *list("default(None, )"),
                    ],
                    "nested": {
                        "a": ["add(1, 2)", *list("default([1, 2, 3], )")],
                        "b": {"x": [*list("default(x, )")]},
                    },
                    "a": "b",
                    "injected": True,
                    "key": "upper(TRANSLATION_RESOLVED:value, None)",
                },
            ),
        ]


class TestSpread:
    # Test that spread operator cannot be used as dictionary value
    def test_spread_as_dictionary_value(self):
        with pytest.raises(
            SyntaxError,
            match=re.escape("expected value or COMMENT"),
        ):
            parse_tag('{% component data={"key": **spread} %}')

    # NOTE: The Rust parser actually parses this successfully,
    # treating `**spread|abc: 123` as a `spread` variable with a filter `abc`
    # that has an argument `123`.
    def test_spread_with_colon_interpreted_as_key(self):
        tag = parse_tag("{% component data={**spread|abc: 123 } %}")

        expected_tag = Tag(
            name=TagToken(token="component", start_index=3, end_index=12, line_col=(1, 4)),
            attrs=[
                TagAttr(
                    key=TagToken(token="data", start_index=13, end_index=17, line_col=(1, 14)),
                    value=TagValue(
                        token=TagToken(token="{**spread|abc: 123 }", start_index=18, end_index=38, line_col=(1, 19)),
                        children=[
                            TagValue(
                                token=TagToken(token="spread", start_index=21, end_index=27, line_col=(1, 22)),
                                children=[],
                                kind=ValueKind("variable"),
                                spread="**",
                                filters=[
                                    TagValueFilter(
                                        token=TagToken(token="abc", start_index=28, end_index=31, line_col=(1, 29)),
                                        arg=TagValue(
                                            token=TagToken(
                                                token="123", start_index=33, end_index=36, line_col=(1, 34)
                                            ),
                                            children=[],
                                            kind=ValueKind("int"),
                                            spread=None,
                                            filters=[],
                                            start_index=31,
                                            end_index=36,
                                            line_col=(1, 32),
                                        ),
                                        start_index=27,
                                        end_index=36,
                                        line_col=(1, 28),
                                    )
                                ],
                                start_index=19,
                                end_index=36,
                                line_col=(1, 20),
                            ),
                        ],
                        kind=ValueKind("dict"),
                        spread=None,
                        filters=[],
                        start_index=18,
                        end_index=38,
                        line_col=(1, 19),
                    ),
                    is_flag=False,
                    start_index=13,
                    end_index=38,
                    line_col=(1, 14),
                ),
            ],
            is_self_closing=False,
            syntax=TagSyntax("django"),
            start_index=0,
            end_index=41,
            line_col=(1, 4),
        )

        assert tag == expected_tag

        tag_func = compile_tag(tag)
        args1, kwargs1 = tag_func(
            context={"spread": {6: 7}},
            variable=lambda ctx, var: ctx[var],
            template_string=lambda ctx, expr: f"TEMPLATE_RESOLVED:{expr}",
            translation=lambda ctx, text: f"TRANSLATION_RESOLVED:{text}",
            filter=lambda ctx, name, value, arg=None: {**value, "ABC": arg}
            if name == "abc"
            else f"{name}({value}, {arg})",
        )
        assert args1 == []
        assert kwargs1 == [
            ("data", {6: 7, "ABC": 123}),
        ]

    def test_spread_in_filter_position(self):
        with pytest.raises(
            SyntaxError,
            match=re.escape("expected filter_name or COMMENT"),
        ):
            parse_tag("{% component data=val|...spread|abc } %}")

    def test_spread_whitespace_1(self):
        # NOTE: Separating `...` from its variable is NOT valid, and will result in error.
        with pytest.raises(
            SyntaxError,
            match=re.escape("expected value"),
        ):
            parse_tag("{% component ... attrs %}")

    # NOTE: But there CAN be whitespace between `*` / `**` and the value,
    #       because we're scoped inside `{ ... }` dict or `[ ... ]` list.
    def test_spread_whitespace_2(self):
        tag = parse_tag('{% component dict={"a": "b", ** my_attr} list=["a", * my_list] %}')

        expected_tag = Tag(
            name=TagToken(token="component", start_index=3, end_index=12, line_col=(1, 4)),
            attrs=[
                TagAttr(
                    key=TagToken(token="dict", start_index=13, end_index=17, line_col=(1, 14)),
                    value=TagValue(
                        token=TagToken(token='{"a": "b", ** my_attr}', start_index=18, end_index=40, line_col=(1, 19)),
                        children=[
                            TagValue(
                                token=TagToken(token='"a"', start_index=19, end_index=22, line_col=(1, 20)),
                                children=[],
                                kind=ValueKind("string"),
                                spread=None,
                                filters=[],
                                start_index=19,
                                end_index=22,
                                line_col=(1, 20),
                            ),
                            TagValue(
                                token=TagToken(token='"b"', start_index=24, end_index=27, line_col=(1, 25)),
                                children=[],
                                kind=ValueKind("string"),
                                spread=None,
                                filters=[],
                                start_index=24,
                                end_index=27,
                                line_col=(1, 25),
                            ),
                            TagValue(
                                token=TagToken(token="my_attr", start_index=32, end_index=39, line_col=(1, 33)),
                                children=[],
                                kind=ValueKind("variable"),
                                spread="**",
                                filters=[],
                                start_index=30,
                                end_index=39,
                                line_col=(1, 31),
                            ),
                        ],
                        kind=ValueKind("dict"),
                        spread=None,
                        filters=[],
                        start_index=18,
                        end_index=40,
                        line_col=(1, 19),
                    ),
                    is_flag=False,
                    start_index=13,
                    end_index=40,
                    line_col=(1, 14),
                ),
                TagAttr(
                    key=TagToken(token="list", start_index=41, end_index=45, line_col=(1, 42)),
                    value=TagValue(
                        token=TagToken(token='["a", * my_list]', start_index=46, end_index=62, line_col=(1, 47)),
                        children=[
                            TagValue(
                                token=TagToken(token='"a"', start_index=47, end_index=50, line_col=(1, 48)),
                                children=[],
                                kind=ValueKind("string"),
                                spread=None,
                                filters=[],
                                start_index=47,
                                end_index=50,
                                line_col=(1, 48),
                            ),
                            TagValue(
                                token=TagToken(token="my_list", start_index=54, end_index=61, line_col=(1, 55)),
                                children=[],
                                kind=ValueKind("variable"),
                                spread="*",
                                filters=[],
                                start_index=53,
                                end_index=61,
                                line_col=(1, 54),
                            ),
                        ],
                        kind=ValueKind("list"),
                        spread=None,
                        filters=[],
                        start_index=46,
                        end_index=62,
                        line_col=(1, 47),
                    ),
                    is_flag=False,
                    start_index=41,
                    end_index=62,
                    line_col=(1, 42),
                ),
            ],
            is_self_closing=False,
            syntax=TagSyntax("django"),
            start_index=0,
            end_index=65,
            line_col=(1, 4),
        )

        assert tag == expected_tag

        tag_func = compile_tag(tag)
        args1, kwargs1 = tag_func(
            context={"my_attr": {6: 7}, "my_list": [8, 9]},
            variable=lambda ctx, var: ctx[var],
            template_string=lambda ctx, expr: f"TEMPLATE_RESOLVED:{expr}",
            translation=lambda ctx, text: f"TRANSLATION_RESOLVED:{text}",
            filter=lambda ctx, name, value, arg=None: f"{name}({value}, {arg})",
        )
        assert args1 == []
        assert kwargs1 == [
            ("dict", {"a": "b", 6: 7}),
            ("list", ["a", 8, 9]),
        ]

        with pytest.raises(
            TypeError,
            match=re.escape("list' object is not a mapping"),
        ):
            tag_func(
                context={"my_attr": [6, 7], "my_list": [8, 9]},
                variable=lambda ctx, var: ctx[var],
                template_string=lambda ctx, expr: f"TEMPLATE_RESOLVED:{expr}",
                translation=lambda ctx, text: f"TRANSLATION_RESOLVED:{text}",
                filter=lambda ctx, name, value, arg=None: f"{name}({value}, {arg})",
            )

        # NOTE: This still works because even tho my_list is not a list,
        #       dictionaries are still iterable (same as dict.keys()).
        args2, kwargs2 = tag_func(
            context={"my_attr": {6: 7}, "my_list": {8: 9}},
            variable=lambda ctx, var: ctx[var],
            template_string=lambda ctx, expr: f"TEMPLATE_RESOLVED:{expr}",
            translation=lambda ctx, text: f"TRANSLATION_RESOLVED:{text}",
            filter=lambda ctx, name, value, arg=None: f"{name}({value}, {arg})",
        )
        assert args2 == []
        assert kwargs2 == [
            ("dict", {"a": "b", 6: 7}),
            ("list", ["a", 8]),
        ]

    # Test that one cannot use e.g. `...`, `**`, `*` in wrong places
    def test_spread_incorrect_syntax(self):
        with pytest.raises(
            SyntaxError,
            match=re.escape("expected dict_item_spread, dict_key, or COMMENT"),
        ):
            parse_tag('{% component dict={"a": "b", *my_attr} %}')

        with pytest.raises(
            SyntaxError,
            match=re.escape("expected dict_item_spread, dict_key, or COMMENT"),
        ):
            _ = parse_tag('{% component dict={"a": "b", ...my_attr} %}')

        with pytest.raises(
            SyntaxError,
            match=re.escape("expected value or COMMENT"),
        ):
            _ = parse_tag('{% component list=["a", "b", **my_list] %}')

        with pytest.raises(
            SyntaxError,
            match=re.escape("expected list_item or COMMENT"),
        ):
            _ = parse_tag('{% component list=["a", "b", ...my_list] %}')

        with pytest.raises(
            SyntaxError,
            match=re.escape("expected self_closing_slash, attribute, or COMMENT"),
        ):
            _ = parse_tag("{% component *attrs %}")

        with pytest.raises(
            SyntaxError,
            match=re.escape("expected self_closing_slash, attribute, or COMMENT"),
        ):
            _ = parse_tag("{% component **attrs %}")

        with pytest.raises(
            SyntaxError,
            match=re.escape("expected value"),
        ):
            _ = parse_tag("{% component key=*attrs %}")

        with pytest.raises(
            SyntaxError,
            match=re.escape("expected value"),
        ):
            _ = parse_tag("{% component key=**attrs %}")

    # Test that one cannot do `key=...{"a": "b"}`
    def test_spread_onto_key(self):
        with pytest.raises(
            SyntaxError,
            match=re.escape("expected value"),
        ):
            parse_tag('{% component key=...{"a": "b"} %}')

        with pytest.raises(
            SyntaxError,
            match=re.escape("expected value"),
        ):
            parse_tag('{% component key=...["a", "b"] %}')

        with pytest.raises(
            SyntaxError,
            match=re.escape("expected value"),
        ):
            parse_tag("{% component key=...attrs %}")

    def test_spread_dict_literal_nested(self):
        tag = parse_tag('{% component { **{"key": val2}, "key": val1 } %}')

        expected_tag = Tag(
            name=TagToken(token="component", start_index=3, end_index=12, line_col=(1, 4)),
            attrs=[
                TagAttr(
                    key=None,
                    value=TagValue(
                        token=TagToken(
                            token='{ **{"key": val2}, "key": val1 }', start_index=13, end_index=45, line_col=(1, 14)
                        ),
                        children=[
                            TagValue(
                                token=TagToken(token='{"key": val2}', start_index=17, end_index=30, line_col=(1, 18)),
                                children=[
                                    TagValue(
                                        token=TagToken(token='"key"', start_index=18, end_index=23, line_col=(1, 19)),
                                        children=[],
                                        kind=ValueKind("string"),
                                        spread=None,
                                        filters=[],
                                        start_index=18,
                                        end_index=23,
                                        line_col=(1, 19),
                                    ),
                                    TagValue(
                                        token=TagToken(token="val2", start_index=25, end_index=29, line_col=(1, 26)),
                                        children=[],
                                        kind=ValueKind("variable"),
                                        spread=None,
                                        filters=[],
                                        start_index=25,
                                        end_index=29,
                                        line_col=(1, 26),
                                    ),
                                ],
                                kind=ValueKind("dict"),
                                spread="**",
                                filters=[],
                                start_index=15,
                                end_index=30,
                                line_col=(1, 16),
                            ),
                            TagValue(
                                token=TagToken(token='"key"', start_index=32, end_index=37, line_col=(1, 33)),
                                children=[],
                                kind=ValueKind("string"),
                                spread=None,
                                filters=[],
                                start_index=32,
                                end_index=37,
                                line_col=(1, 33),
                            ),
                            TagValue(
                                token=TagToken(token="val1", start_index=39, end_index=43, line_col=(1, 40)),
                                children=[],
                                kind=ValueKind("variable"),
                                spread=None,
                                filters=[],
                                start_index=39,
                                end_index=43,
                                line_col=(1, 40),
                            ),
                        ],
                        kind=ValueKind("dict"),
                        spread=None,
                        filters=[],
                        start_index=13,
                        end_index=45,
                        line_col=(1, 14),
                    ),
                    is_flag=False,
                    start_index=13,
                    end_index=45,
                    line_col=(1, 14),
                ),
            ],
            is_self_closing=False,
            syntax=TagSyntax("django"),
            start_index=0,
            end_index=48,
            line_col=(1, 4),
        )

        assert tag == expected_tag

        tag_func = compile_tag(tag)
        args, kwargs = tag_func(
            context={"val1": 1, "val2": 2},
            variable=lambda ctx, var: ctx[var],
            template_string=lambda ctx, expr: f"TEMPLATE_RESOLVED:{expr}",
            translation=lambda ctx, text: f"TRANSLATION_RESOLVED:{text}",
            filter=lambda ctx, name, value, arg=None: f"{name}({value}, {arg})",
        )
        assert args == [{"key": 1}]
        assert kwargs == []

    def test_spread_dict_literal_as_attribute(self):
        tag = parse_tag('{% component ...{"key": val2} %}')

        expected_tag = Tag(
            name=TagToken(token="component", start_index=3, end_index=12, line_col=(1, 4)),
            attrs=[
                TagAttr(
                    key=None,
                    value=TagValue(
                        token=TagToken(token='{"key": val2}', start_index=16, end_index=29, line_col=(1, 17)),
                        children=[
                            TagValue(
                                token=TagToken(token='"key"', start_index=17, end_index=22, line_col=(1, 18)),
                                children=[],
                                kind=ValueKind("string"),
                                spread=None,
                                filters=[],
                                start_index=17,
                                end_index=22,
                                line_col=(1, 18),
                            ),
                            TagValue(
                                token=TagToken(token="val2", start_index=24, end_index=28, line_col=(1, 25)),
                                children=[],
                                kind=ValueKind("variable"),
                                spread=None,
                                filters=[],
                                start_index=24,
                                end_index=28,
                                line_col=(1, 25),
                            ),
                        ],
                        kind=ValueKind("dict"),
                        spread="...",
                        filters=[],
                        start_index=13,
                        end_index=29,
                        line_col=(1, 14),
                    ),
                    is_flag=False,
                    start_index=13,
                    end_index=29,
                    line_col=(1, 14),
                ),
            ],
            is_self_closing=False,
            syntax=TagSyntax("django"),
            start_index=0,
            end_index=32,
            line_col=(1, 4),
        )

        assert tag == expected_tag

        tag_func = compile_tag(tag)
        args, kwargs = tag_func(
            context={"val1": 1, "val2": 2},
            variable=lambda ctx, var: ctx[var],
            template_string=lambda ctx, expr: f"TEMPLATE_RESOLVED:{expr}",
            translation=lambda ctx, text: f"TRANSLATION_RESOLVED:{text}",
            filter=lambda ctx, name, value, arg=None: f"{name}({value}, {arg})",
        )
        assert args == []
        assert kwargs == [("key", 2)]

    def test_spread_list_literal_nested(self):
        tag = parse_tag("{% component [ *[val1], val2 ] %}")

        expected_tag = Tag(
            name=TagToken(token="component", start_index=3, end_index=12, line_col=(1, 4)),
            attrs=[
                TagAttr(
                    key=None,
                    value=TagValue(
                        token=TagToken(token="[ *[val1], val2 ]", start_index=13, end_index=30, line_col=(1, 14)),
                        children=[
                            TagValue(
                                token=TagToken(token="[val1]", start_index=16, end_index=22, line_col=(1, 17)),
                                children=[
                                    TagValue(
                                        token=TagToken(token="val1", start_index=17, end_index=21, line_col=(1, 18)),
                                        children=[],
                                        kind=ValueKind("variable"),
                                        spread=None,
                                        filters=[],
                                        start_index=17,
                                        end_index=21,
                                        line_col=(1, 18),
                                    ),
                                ],
                                kind=ValueKind("list"),
                                spread="*",
                                filters=[],
                                start_index=15,
                                end_index=22,
                                line_col=(1, 16),
                            ),
                            TagValue(
                                token=TagToken(token="val2", start_index=24, end_index=28, line_col=(1, 25)),
                                children=[],
                                kind=ValueKind("variable"),
                                spread=None,
                                filters=[],
                                start_index=24,
                                end_index=28,
                                line_col=(1, 25),
                            ),
                        ],
                        kind=ValueKind("list"),
                        spread=None,
                        filters=[],
                        start_index=13,
                        end_index=30,
                        line_col=(1, 14),
                    ),
                    is_flag=False,
                    start_index=13,
                    end_index=30,
                    line_col=(1, 14),
                ),
            ],
            is_self_closing=False,
            syntax=TagSyntax("django"),
            start_index=0,
            end_index=33,
            line_col=(1, 4),
        )

        assert tag == expected_tag

        tag_func = compile_tag(tag)
        args, kwargs = tag_func(
            context={"val1": 1, "val2": 2},
            variable=lambda ctx, var: ctx[var],
            template_string=lambda ctx, expr: f"TEMPLATE_RESOLVED:{expr}",
            translation=lambda ctx, text: f"TRANSLATION_RESOLVED:{text}",
            filter=lambda ctx, name, value, arg=None: f"{name}({value}, {arg})",
        )
        assert args == [[1, 2]]
        assert kwargs == []

    def test_spread_list_literal_as_attribute(self):
        tag = parse_tag("{% component ...[val1] %}")

        expected_tag = Tag(
            name=TagToken(token="component", start_index=3, end_index=12, line_col=(1, 4)),
            attrs=[
                TagAttr(
                    key=None,
                    value=TagValue(
                        token=TagToken(token="[val1]", start_index=16, end_index=22, line_col=(1, 17)),
                        children=[
                            TagValue(
                                token=TagToken(token="val1", start_index=17, end_index=21, line_col=(1, 18)),
                                children=[],
                                kind=ValueKind("variable"),
                                spread=None,
                                filters=[],
                                start_index=17,
                                end_index=21,
                                line_col=(1, 18),
                            ),
                        ],
                        kind=ValueKind("list"),
                        spread="...",
                        filters=[],
                        start_index=13,
                        end_index=22,
                        line_col=(1, 14),
                    ),
                    is_flag=False,
                    start_index=13,
                    end_index=22,
                    line_col=(1, 14),
                ),
            ],
            is_self_closing=False,
            syntax=TagSyntax("django"),
            start_index=0,
            end_index=25,
            line_col=(1, 4),
        )

        assert tag == expected_tag

        tag_func = compile_tag(tag)
        args, kwargs = tag_func(
            context={"val1": 1, "val2": 2},
            variable=lambda ctx, var: ctx[var],
            template_string=lambda ctx, expr: f"TEMPLATE_RESOLVED:{expr}",
            translation=lambda ctx, text: f"TRANSLATION_RESOLVED:{text}",
            filter=lambda ctx, name, value, arg=None: f"{name}({value}, {arg})",
        )
        assert args == [1]
        assert kwargs == []


class TestTemplateString:
    def test_template_string(self):
        tag = parse_tag("{% component '{% lorem w 4 %}' %}")

        expected_tag = Tag(
            name=TagToken(token="component", start_index=3, end_index=12, line_col=(1, 4)),
            attrs=[
                TagAttr(
                    key=None,
                    value=TagValue(
                        token=TagToken(token="'{% lorem w 4 %}'", start_index=13, end_index=30, line_col=(1, 14)),
                        children=[],
                        kind=ValueKind("template_string"),
                        spread=None,
                        filters=[],
                        start_index=13,
                        end_index=30,
                        line_col=(1, 14),
                    ),
                    is_flag=False,
                    start_index=13,
                    end_index=30,
                    line_col=(1, 14),
                ),
            ],
            is_self_closing=False,
            syntax=TagSyntax("django"),
            start_index=0,
            end_index=33,
            line_col=(1, 4),
        )

        assert tag == expected_tag

        tag_func = compile_tag(tag)
        args, kwargs = tag_func(
            context={"val1": 1, "val2": 2},
            variable=lambda ctx, var: ctx[var],
            template_string=lambda ctx, expr: f"TEMPLATE_RESOLVED:{expr}",
            translation=lambda ctx, text: f"TRANSLATION_RESOLVED:{text}",
            filter=lambda ctx, name, value, arg=None: f"{name}({value}, {arg})",
        )
        assert args == ["TEMPLATE_RESOLVED:{% lorem w 4 %}"]
        assert kwargs == []

    def test_template_string_in_dict(self):
        tag = parse_tag('{% component { "key": "{% lorem w 4 %}" } %}')

        expected_tag = Tag(
            name=TagToken(token="component", start_index=3, end_index=12, line_col=(1, 4)),
            attrs=[
                TagAttr(
                    key=None,
                    value=TagValue(
                        token=TagToken(
                            token='{ "key": "{% lorem w 4 %}" }', start_index=13, end_index=41, line_col=(1, 14)
                        ),
                        children=[
                            TagValue(
                                token=TagToken(token='"key"', start_index=15, end_index=20, line_col=(1, 16)),
                                children=[],
                                kind=ValueKind("string"),
                                spread=None,
                                filters=[],
                                start_index=15,
                                end_index=20,
                                line_col=(1, 16),
                            ),
                            TagValue(
                                token=TagToken(
                                    token='"{% lorem w 4 %}"', start_index=22, end_index=39, line_col=(1, 23)
                                ),
                                children=[],
                                kind=ValueKind("template_string"),
                                spread=None,
                                filters=[],
                                start_index=22,
                                end_index=39,
                                line_col=(1, 23),
                            ),
                        ],
                        kind=ValueKind("dict"),
                        spread=None,
                        filters=[],
                        start_index=13,
                        end_index=41,
                        line_col=(1, 14),
                    ),
                    is_flag=False,
                    start_index=13,
                    end_index=41,
                    line_col=(1, 14),
                ),
            ],
            is_self_closing=False,
            syntax=TagSyntax("django"),
            start_index=0,
            end_index=44,
            line_col=(1, 4),
        )

        assert tag == expected_tag

        tag_func = compile_tag(tag)
        args, kwargs = tag_func(
            context={"val1": 1, "val2": 2},
            variable=lambda ctx, var: ctx[var],
            template_string=lambda ctx, expr: f"TEMPLATE_RESOLVED:{expr}",
            translation=lambda ctx, text: f"TRANSLATION_RESOLVED:{text}",
            filter=lambda ctx, name, value, arg=None: f"{name}({value}, {arg})",
        )
        assert args == [{"key": "TEMPLATE_RESOLVED:{% lorem w 4 %}"}]
        assert kwargs == []

    def test_template_string_in_list(self):
        tag = parse_tag("{% component [ '{% lorem w 4 %}' ] %}")

        expected_tag = Tag(
            name=TagToken(token="component", start_index=3, end_index=12, line_col=(1, 4)),
            attrs=[
                TagAttr(
                    key=None,
                    value=TagValue(
                        token=TagToken(token="[ '{% lorem w 4 %}' ]", start_index=13, end_index=34, line_col=(1, 14)),
                        children=[
                            TagValue(
                                token=TagToken(
                                    token="'{% lorem w 4 %}'", start_index=15, end_index=32, line_col=(1, 16)
                                ),
                                children=[],
                                kind=ValueKind("template_string"),
                                spread=None,
                                filters=[],
                                start_index=15,
                                end_index=32,
                                line_col=(1, 16),
                            ),
                        ],
                        kind=ValueKind("list"),
                        spread=None,
                        filters=[],
                        start_index=13,
                        end_index=34,
                        line_col=(1, 14),
                    ),
                    is_flag=False,
                    start_index=13,
                    end_index=34,
                    line_col=(1, 14),
                ),
            ],
            is_self_closing=False,
            syntax=TagSyntax("django"),
            start_index=0,
            end_index=37,
            line_col=(1, 4),
        )

        assert tag == expected_tag

        tag_func = compile_tag(tag)
        args, kwargs = tag_func(
            context={"val1": 1, "val2": 2},
            variable=lambda ctx, var: ctx[var],
            template_string=lambda ctx, expr: f"TEMPLATE_RESOLVED:{expr}",
            translation=lambda ctx, text: f"TRANSLATION_RESOLVED:{text}",
            filter=lambda ctx, name, value, arg=None: f"{name}({value}, {arg})",
        )
        assert args == [["TEMPLATE_RESOLVED:{% lorem w 4 %}"]]
        assert kwargs == []


class TestComments:
    def test_comments(self):
        tag = parse_tag("{% component {# comment #} val %}")

        expected_tag = Tag(
            name=TagToken(token="component", start_index=3, end_index=12, line_col=(1, 4)),
            attrs=[
                TagAttr(
                    key=None,
                    value=TagValue(
                        token=TagToken(token="val", start_index=27, end_index=30, line_col=(1, 28)),
                        children=[],
                        kind=ValueKind("variable"),
                        spread=None,
                        filters=[],
                        start_index=27,
                        end_index=30,
                        line_col=(1, 28),
                    ),
                    is_flag=False,
                    start_index=27,
                    end_index=30,
                    line_col=(1, 28),
                ),
            ],
            is_self_closing=False,
            syntax=TagSyntax("django"),
            start_index=0,
            end_index=33,
            line_col=(1, 4),
        )

        assert tag == expected_tag

        tag_func = compile_tag(tag)
        args, kwargs = tag_func(
            context={"val": 1, "val2": 2},
            variable=lambda ctx, var: ctx[var],
            template_string=lambda ctx, expr: f"TEMPLATE_RESOLVED:{expr}",
            translation=lambda ctx, text: f"TRANSLATION_RESOLVED:{text}",
            filter=lambda ctx, name, value, arg=None: f"{name}({value}, {arg})",
        )
        assert args == [1]
        assert kwargs == []

    def test_comments_within_list(self):
        tag = parse_tag("{% component [ *[val1], {# comment #} val2 ] %}")

        expected_tag = Tag(
            name=TagToken(token="component", start_index=3, end_index=12, line_col=(1, 4)),
            attrs=[
                TagAttr(
                    key=None,
                    value=TagValue(
                        token=TagToken(
                            token="[ *[val1], {# comment #} val2 ]", start_index=13, end_index=44, line_col=(1, 14)
                        ),
                        children=[
                            TagValue(
                                token=TagToken(token="[val1]", start_index=16, end_index=22, line_col=(1, 17)),
                                children=[
                                    TagValue(
                                        token=TagToken(token="val1", start_index=17, end_index=21, line_col=(1, 18)),
                                        children=[],
                                        kind=ValueKind("variable"),
                                        spread=None,
                                        filters=[],
                                        start_index=17,
                                        end_index=21,
                                        line_col=(1, 18),
                                    ),
                                ],
                                kind=ValueKind("list"),
                                spread="*",
                                filters=[],
                                start_index=15,
                                end_index=22,
                                line_col=(1, 16),
                            ),
                            TagValue(
                                token=TagToken(token="val2", start_index=38, end_index=42, line_col=(1, 39)),
                                children=[],
                                kind=ValueKind("variable"),
                                spread=None,
                                filters=[],
                                start_index=38,
                                end_index=42,
                                line_col=(1, 39),
                            ),
                        ],
                        kind=ValueKind("list"),
                        spread=None,
                        filters=[],
                        start_index=13,
                        end_index=44,
                        line_col=(1, 14),
                    ),
                    is_flag=False,
                    start_index=13,
                    end_index=44,
                    line_col=(1, 14),
                ),
            ],
            is_self_closing=False,
            syntax=TagSyntax("django"),
            start_index=0,
            end_index=47,
            line_col=(1, 4),
        )

        assert tag == expected_tag

        tag_func = compile_tag(tag)
        args, kwargs = tag_func(
            context={"val1": 1, "val2": 2},
            variable=lambda ctx, var: ctx[var],
            template_string=lambda ctx, expr: f"TEMPLATE_RESOLVED:{expr}",
            translation=lambda ctx, text: f"TRANSLATION_RESOLVED:{text}",
            filter=lambda ctx, name, value, arg=None: f"{name}({value}, {arg})",
        )
        assert args == [[1, 2]]
        assert kwargs == []

    def test_comments_within_dict(self):
        tag = parse_tag('{% component { "key": "123" {# comment #} } %}')

        expected_tag = Tag(
            name=TagToken(token="component", start_index=3, end_index=12, line_col=(1, 4)),
            attrs=[
                TagAttr(
                    key=None,
                    value=TagValue(
                        token=TagToken(
                            token='{ "key": "123" {# comment #} }', start_index=13, end_index=43, line_col=(1, 14)
                        ),
                        children=[
                            TagValue(
                                token=TagToken(token='"key"', start_index=15, end_index=20, line_col=(1, 16)),
                                children=[],
                                kind=ValueKind("string"),
                                spread=None,
                                filters=[],
                                start_index=15,
                                end_index=20,
                                line_col=(1, 16),
                            ),
                            TagValue(
                                token=TagToken(token='"123"', start_index=22, end_index=27, line_col=(1, 23)),
                                children=[],
                                kind=ValueKind("string"),
                                spread=None,
                                filters=[],
                                start_index=22,
                                end_index=27,
                                line_col=(1, 23),
                            ),
                        ],
                        kind=ValueKind("dict"),
                        spread=None,
                        filters=[],
                        start_index=13,
                        end_index=43,
                        line_col=(1, 14),
                    ),
                    is_flag=False,
                    start_index=13,
                    end_index=43,
                    line_col=(1, 14),
                ),
            ],
            is_self_closing=False,
            syntax=TagSyntax("django"),
            start_index=0,
            end_index=46,
            line_col=(1, 4),
        )

        assert tag == expected_tag

        tag_func = compile_tag(tag)
        args, kwargs = tag_func(
            context={"val1": 1, "val2": 2},
            variable=lambda ctx, var: ctx[var],
            template_string=lambda ctx, expr: f"TEMPLATE_RESOLVED:{expr}",
            translation=lambda ctx, text: f"TRANSLATION_RESOLVED:{text}",
            filter=lambda ctx, name, value, arg=None: f"{name}({value}, {arg})",
        )
        assert args == [{"key": "123"}]
        assert kwargs == []


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
                variable=lambda ctx, var: ctx[var],
                template_string=lambda ctx, expr: f"TEMPLATE_RESOLVED:{expr}",
                translation=lambda ctx, text: f"TRANSLATION_RESOLVED:{text}",
                filter=lambda ctx, name, value, arg=None: f"{name}({value}, {arg})",
            )

    def test_arg_after_list_spread_is_ok(self):
        tag_content = "{% my_tag ...[1, 2, 3] positional_arg %}"
        ast = parse_tag(input=tag_content)
        tag_func = compile_tag(ast)
        args, kwargs = tag_func(
            context={"positional_arg": 4},
            variable=lambda ctx, var: ctx[var],
            template_string=lambda ctx, expr: f"TEMPLATE_RESOLVED:{expr}",
            translation=lambda ctx, text: f"TRANSLATION_RESOLVED:{text}",
            filter=lambda ctx, name, value, arg=None: f"{name}({value}, {arg})",
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
            template_string=lambda ctx, expr: f"TEMPLATE_RESOLVED:{expr}",
            translation=lambda ctx, text: f"TRANSLATION_RESOLVED:{text}",
            filter=lambda ctx, name, value, arg=None: f"{name}({value}, {arg})",
        )
        assert args == [1]
        assert kwargs == [("key", "value")]

    def test_dict_spread_after_kwarg_is_ok(self):
        tag_content = "{% my_tag key='value' ...{'key2': 'value2'} %}"
        ast = parse_tag(input=tag_content)
        tag_func = compile_tag(ast)
        args, kwargs = tag_func(
            context={},
            variable=lambda ctx, var: ctx[var],
            template_string=lambda ctx, expr: f"TEMPLATE_RESOLVED:{expr}",
            translation=lambda ctx, text: f"TRANSLATION_RESOLVED:{text}",
            filter=lambda ctx, name, value, arg=None: f"{name}({value}, {arg})",
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
            template_string=lambda ctx, expr: f"TEMPLATE_RESOLVED:{expr}",
            translation=lambda ctx, text: f"TRANSLATION_RESOLVED:{text}",
            filter=lambda ctx, name, value, arg=None: f"{name}({value}, {arg})",
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
                variable=lambda ctx, var: ctx[var],
                template_string=lambda ctx, expr: f"TEMPLATE_RESOLVED:{expr}",
                translation=lambda ctx, text: f"TRANSLATION_RESOLVED:{text}",
                filter=lambda ctx, name, value, arg=None: f"{name}({value}, {arg})",
            )

    def test_list_spread_after_list_spread_is_ok(self):
        tag_content = "{% my_tag ...[1, 2, 3] ...[4, 5, 6] %}"
        ast = parse_tag(input=tag_content)
        tag_func = compile_tag(ast)
        args, kwargs = tag_func(
            context={},
            variable=lambda ctx, var: ctx[var],
            template_string=lambda ctx, expr: f"TEMPLATE_RESOLVED:{expr}",
            translation=lambda ctx, text: f"TRANSLATION_RESOLVED:{text}",
            filter=lambda ctx, name, value, arg=None: f"{name}({value}, {arg})",
        )
        assert args == [1, 2, 3, 4, 5, 6]
        assert kwargs == []

    def test_dict_spread_after_dict_spread_is_ok(self):
        tag_content = "{% my_tag ...{'key': 'value'} ...{'key2': 'value2'} %}"
        ast = parse_tag(input=tag_content)
        tag_func = compile_tag(ast)
        args, kwargs = tag_func(
            context={},
            variable=lambda ctx, var: ctx[var],
            template_string=lambda ctx, expr: f"TEMPLATE_RESOLVED:{expr}",
            translation=lambda ctx, text: f"TRANSLATION_RESOLVED:{text}",
            filter=lambda ctx, name, value, arg=None: f"{name}({value}, {arg})",
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
                variable=lambda ctx, var: ctx[var],
                template_string=lambda ctx, expr: f"TEMPLATE_RESOLVED:{expr}",
                translation=lambda ctx, text: f"TRANSLATION_RESOLVED:{text}",
                filter=lambda ctx, name, value, arg=None: f"{name}({value}, {arg})",
            )

    def test_dict_spread_after_list_spread_is_ok(self):
        tag_content = "{% my_tag ...[1, 2, 3] ...{'key': 'value'} %}"
        ast = parse_tag(input=tag_content)
        tag_func = compile_tag(ast)
        args, kwargs = tag_func(
            context={},
            variable=lambda ctx, var: ctx[var],
            template_string=lambda ctx, expr: f"TEMPLATE_RESOLVED:{expr}",
            translation=lambda ctx, text: f"TRANSLATION_RESOLVED:{text}",
            filter=lambda ctx, name, value, arg=None: f"{name}({value}, {arg})",
        )
        assert args == [1, 2, 3]
        assert kwargs == [("key", "value")]


class TestFlags:
    def test_flag(self):
        tag_content = "{% my_tag 123 my_flag key='val' %}"

        ast = parse_tag(tag_content, flags={"my_flag"})
        assert ast.attrs[1].value.token.token == "my_flag"
        assert ast.attrs[1].is_flag

        # The compiled function should omit the flag
        compiled_func = compile_tag(ast)
        args, kwargs = compiled_func(
            context={},
            variable=lambda ctx, var: ctx[var],
            template_string=lambda ctx, expr: f"TEMPLATE_RESOLVED:{expr}",
            translation=lambda ctx, text: f"TRANSLATION_RESOLVED:{text}",
            filter=lambda ctx, name, value, arg=None: f"{name}({value}, {arg})",
        )

        assert args == [123]
        assert kwargs == [("key", "val")]

        # Same as before, but with flags=None
        ast2 = parse_tag(tag_content, flags=None)
        assert ast2.attrs[1].value.token.token == "my_flag"
        assert not ast2.attrs[1].is_flag

        # The compiled function should omit the flag
        compiled_func2 = compile_tag(ast2)
        args2, kwargs2 = compiled_func2(
            context={"my_flag": "x"},
            variable=lambda ctx, var: ctx[var],
            template_string=lambda ctx, expr: f"TEMPLATE_RESOLVED:{expr}",
            translation=lambda ctx, text: f"TRANSLATION_RESOLVED:{text}",
            filter=lambda ctx, name, value, arg=None: f"{name}({value}, {arg})",
        )

        assert args2 == [123, "x"]
        assert kwargs2 == [("key", "val")]

    # my_flag is NOT treated as flag because it's used as spread
    def test_flag_as_spread(self):
        tag_content = "{% my_tag ...my_flag %}"

        ast1 = parse_tag(tag_content, flags={"my_flag"})
        assert ast1.attrs[0].value.token.token == "my_flag"
        assert not ast1.attrs[0].is_flag

        compiled_func1 = compile_tag(ast1)
        args1, kwargs1 = compiled_func1(
            context={"my_flag": ["arg1", "arg2"]},
            variable=lambda ctx, var: ctx[var],
            template_string=lambda ctx, expr: f"TEMPLATE_RESOLVED:{expr}",
            translation=lambda ctx, text: f"TRANSLATION_RESOLVED:{text}",
            filter=lambda ctx, name, value, arg=None: f"{name}({value}, {arg})",
        )

        assert args1 == ["arg1", "arg2"]
        assert kwargs1 == []

        # Same as before, but with flags=None
        ast2 = parse_tag(tag_content, flags=None)
        assert ast2.attrs[0].value.token.token == "my_flag"
        assert not ast2.attrs[0].is_flag

        compiled_func2 = compile_tag(ast2)
        args2, kwargs2 = compiled_func2(
            context={"my_flag": ["arg1", "arg2"]},
            variable=lambda ctx, var: ctx[var],
            template_string=lambda ctx, expr: f"TEMPLATE_RESOLVED:{expr}",
            translation=lambda ctx, text: f"TRANSLATION_RESOLVED:{text}",
            filter=lambda ctx, name, value, arg=None: f"{name}({value}, {arg})",
        )

        assert args2 == ["arg1", "arg2"]
        assert kwargs2 == []

    # my_flag is NOT treated as flag because it's used as kwarg
    def test_flag_as_kwarg(self):
        tag_content = "{% my_tag my_flag=123 %}"

        ast1 = parse_tag(tag_content, flags={"my_flag"})
        assert ast1.attrs[0].key
        assert ast1.attrs[0].key.token == "my_flag"
        assert not ast1.attrs[0].is_flag

        compiled_func1 = compile_tag(ast1)
        args1, kwargs1 = compiled_func1(
            context={},
            variable=lambda ctx, var: ctx[var],
            template_string=lambda ctx, expr: f"TEMPLATE_RESOLVED:{expr}",
            translation=lambda ctx, text: f"TRANSLATION_RESOLVED:{text}",
            filter=lambda ctx, name, value, arg=None: f"{name}({value}, {arg})",
        )
        assert args1 == []
        assert kwargs1 == [("my_flag", 123)]

        # Same as before, but with flags=None
        ast2 = parse_tag(tag_content, flags=None)
        assert ast2.attrs[0].key
        assert ast2.attrs[0].key.token == "my_flag"
        assert not ast2.attrs[0].is_flag

        compiled_func2 = compile_tag(ast2)
        args2, kwargs2 = compiled_func2(
            context={},
            variable=lambda ctx, var: ctx[var],
            template_string=lambda ctx, expr: f"TEMPLATE_RESOLVED:{expr}",
            translation=lambda ctx, text: f"TRANSLATION_RESOLVED:{text}",
            filter=lambda ctx, name, value, arg=None: f"{name}({value}, {arg})",
        )
        assert args2 == []
        assert kwargs2 == [("my_flag", 123)]

    def test_flag_duplicate(self):
        tag_content = "{% my_tag my_flag my_flag %}"
        with pytest.raises(SyntaxError, match=r"Flag 'my_flag' may be specified only once."):
            parse_tag(tag_content, flags={"my_flag"})

    def test_flag_case_sensitive(self):
        tag_content = "{% my_tag my_flag %}"
        ast = parse_tag(tag_content, flags={"MY_FLAG"})
        assert ast.attrs[0].value.token.token == "my_flag"
        assert not ast.attrs[0].is_flag


class TestSelfClosing:
    def test_self_closing_simple(self):
        ast = parse_tag("{% my_tag / %}")
        assert ast.name.token == "my_tag"
        assert ast.is_self_closing is True
        assert ast.attrs == []

    def test_self_closing_with_args(self):
        ast = parse_tag("{% my_tag key=val / %}")
        assert ast.name.token == "my_tag"
        assert ast.is_self_closing is True
        assert len(ast.attrs) == 1
        assert ast.attrs[0].key
        assert ast.attrs[0].key.token == "key"
        assert ast.attrs[0].value.token.token == "val"

    def test_self_closing_in_middle_errors(self):
        with pytest.raises(
            SyntaxError,
            match=r"expected attribute or COMMENT",
        ):
            parse_tag("{% my_tag / key=val %}")
