from statistics import mean, stdev
import time

from djc_core_html_parser import set_html_attributes


def generate_large_html(num_elements: int = 1000) -> str:
    """Generate a large HTML document with various features for benchmarking."""
    elements = []
    for i in range(num_elements):
        # Mix of different elements and features
        if i % 5 == 0:
            # Void element with multiple attributes
            elements.append(f'<img src="image{i}.jpg" alt="Image {i}" class="img-{i}" loading="lazy" />')
        elif i % 5 == 1:
            # Nested divs with attributes
            elements.append(
                f"""
                <div class="container-{i}" data-index="{i}">
                    <div class="inner-{i}">
                        <p>Content {i}</p>
                        <!-- Comment {i} -->
                    </div>
                </div>
            """
            )
        elif i % 5 == 2:
            # Script tag with content
            elements.append(
                f"""
                <script type="text/javascript">
                    // Script {i}
                    console.log("Script {i}");
                    /* Multi-line
                       comment {i} */
                </script>
            """
            )
        elif i % 5 == 3:
            # CDATA section
            elements.append(
                f"""
                <![CDATA[
                    Raw content {i}
                    <not-a-tag>
                ]]>
            """
            )
        else:
            # Regular element with attributes
            elements.append(
                f"""
                <section id="section-{i}" class="section-{i}">
                    <h2>Heading {i}</h2>
                    <p class="text-{i}">Paragraph {i}</p>
                </section>
            """
            )

    return f"""
        <!DOCTYPE html>
        <html>
        <head>
            <title>Benchmark Page</title>
            <meta charset="utf-8">
        </head>
        <body>
            {''.join(elements)}
        </body>
        </html>
    """


# Generate test HTML
HTML_SIZE = 27_000  # Set to 11_000 for 2MB
NUM_ITER = 2
html = generate_large_html(HTML_SIZE)
print(f"\nBenchmarking with HTML size: {len(html) // 1_000} KB")

root_attributes = ["data-root-id"]
all_attributes = ["data-v-123"]

# Test transform
modify_times = []
for i in range(NUM_ITER):  # Run N iterations

    start = time.perf_counter()
    set_html_attributes(html, root_attributes, all_attributes, watch_on_attribute="data-id")
    modify_time = time.perf_counter() - start
    modify_times.append(modify_time)

print("\nTransform:")
print(f"  Total: {sum(modify_times):.3f}s")
print(f"  Min: {min(modify_times):.3f}s")
print(f"  Max: {max(modify_times):.3f}s")
print(f"  Avg: {mean(modify_times):.3f}s")
print(f"  Std: {stdev(modify_times):.3f}s")
