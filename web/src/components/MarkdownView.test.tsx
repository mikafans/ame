import { describe, it, expect } from "bun:test";
import React from "react";
import { renderToString } from "react-dom/server";
import { MarkdownView, sanitizeSvg } from "./MarkdownView";

function renderWithTheme(ui: React.ReactElement) {
  return renderToString(ui);
}

describe("MarkdownView", () => {
  it("renders safe HTML elements", () => {
    const markdown =
      "# Hello World\nThis is a [link](https://google.com) and **strong** text.";
    const html = renderWithTheme(<MarkdownView content={markdown} />);

    // Assert safe heading, link, and strong tags are present
    expect(html).toContain("Hello World");
    expect(html).toContain('href="https://google.com"');
    expect(html).toContain("strong");
  });

  it("strips dangerous tags and scripts", () => {
    const markdown =
      "Hello <script>alert('xss')</script> world <iframe src='https://malicious.com'></iframe>";
    const html = renderWithTheme(<MarkdownView content={markdown} />);

    // Script tag and iframe tag should not be rendered
    expect(html).not.toContain("<script>");
    expect(html).not.toContain("<iframe>");
  });

  it("blocks non-https and svg images", () => {
    // 1. Safe https image
    const htmlSafe = renderWithTheme(
      <MarkdownView content="![Safe](https://example.com/image.jpg)" />,
    );
    expect(htmlSafe).toContain('src="https://example.com/image.jpg"');

    // 2. Unsafe http image
    const htmlUnsafeHttp = renderWithTheme(
      <MarkdownView content="![Unsafe](http://example.com/image.jpg)" />,
    );
    expect(htmlUnsafeHttp).not.toContain("http://example.com/image.jpg");
    expect(htmlUnsafeHttp).not.toContain("<img");

    // 3. SVG image file
    const htmlSvg = renderWithTheme(
      <MarkdownView content="![SVG](https://example.com/image.svg)" />,
    );
    expect(htmlSvg).not.toContain("image.svg");
    expect(htmlSvg).not.toContain("<img");

    // 4. Data URI image
    const htmlDataUri = renderWithTheme(
      <MarkdownView content="![Data](data:image/png;base64,iVBORw0KGgoAAAANS)" />,
    );
    expect(htmlDataUri).not.toContain("data:image/png");
    expect(htmlDataUri).not.toContain("<img");
  });

  it("renders python and bash code blocks using existing Prism structure", () => {
    const markdown = "```python\nprint('Hello')\n```";
    const html = renderWithTheme(<MarkdownView content={markdown} />);

    expect(html).toContain("language-python");
    expect(html).toContain("print(&#x27;Hello&#x27;)");
  });

  it("renders mermaid block placeholder on server/initial mount", () => {
    const markdown = "```mermaid\ngraph TD\n  A --> B\n```";
    const html = renderWithTheme(<MarkdownView content={markdown} />);

    // Should render the loading indicator / skeleton initially on server
    expect(html).toContain("Rendering diagram...");
  });
});

describe("sanitizeSvg", () => {
  it("removes script tags from SVG content", () => {
    const dirty =
      '<svg><script>alert("XSS")</script><rect width="10" height="10"/></svg>';
    const clean = sanitizeSvg(dirty);
    expect(clean).not.toContain("<script>");
    expect(clean).not.toContain("alert");
    expect(clean).toContain("<rect");
  });

  it("removes event handlers (on*)", () => {
    const dirty =
      '<svg><rect width="10" height="10" onclick="alert(1)" onload = \'doSomething()\' onerror=jump /></svg>';
    const clean = sanitizeSvg(dirty);
    expect(clean).not.toContain("onclick");
    expect(clean).not.toContain("onload");
    expect(clean).not.toContain("onerror");
    expect(clean).toContain("rect");
  });

  it("removes javascript: URLs from href and xlink:href attributes", () => {
    const dirty =
      '<svg><a href="javascript:alert(1)"><image xlink:href="javascript:evil()"/></a></svg>';
    const clean = sanitizeSvg(dirty);
    expect(clean).not.toContain("javascript:alert(1)");
    expect(clean).not.toContain("javascript:evil()");
    expect(clean).toContain('href="#"');
    expect(clean).toContain('xlink:href="#"');
  });
});
