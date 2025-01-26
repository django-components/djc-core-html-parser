from typing import List, Dict, Optional

def set_html_attributes(
    html: str,
    root_attributes: List[str],
    all_attributes: List[str],
    check_end_names: Optional[bool] = None,
    watch_on_attribute: Optional[str] = None,
) -> tuple[str, Dict[str, List[str]]]:
    """
    Transform HTML by adding attributes to root and all elements.

    Args:
        html (str): The HTML string to transform. Can be a fragment or full document.
        root_attributes (List[str]): List of attribute names to add to root elements only.
        all_attributes (List[str]): List of attribute names to add to all elements.
        check_end_names (Optional[bool]): Whether to validate matching of end tags. Defaults to None.
        watch_on_attribute (Optional[str]): If set, captures which attributes were added to elements with this attribute.

    Returns:
        A tuple containing:
            - The transformed HTML string
            - A dictionary mapping captured attribute values to lists of attributes that were added
              to those elements. Only returned if watch_on_attribute is set, otherwise empty dict.

    Example:
        >>> html = '<div><p>Hello</p></div>'
        >>> set_html_attributes(html, ['data-root-id'], ['data-v-123'])
        '<div data-root-id="" data-v-123=""><p data-v-123="">Hello</p></div>'

    Raises:
        ValueError: If the HTML is malformed or cannot be parsed.
    """
    ...
